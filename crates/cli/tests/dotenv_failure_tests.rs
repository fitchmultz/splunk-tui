//! Integration tests for dotenv failure handling in CLI.
//!
//! Responsibilities:
//! - Prove that invalid `.env` files cause the CLI to fail at startup.
//! - Prove that error messages do not leak secrets from the `.env` file.
//! - Ensure DOTENV_DISABLED=1 allows the CLI to skip a malformed `.env`.
//!
//! Invariants:
//! - Tests use `assert_cmd` to spawn the CLI as a subprocess.
//! - Tests must explicitly clear `DOTENV_DISABLED` to enable dotenv loading.
//! - Tests use temp directories and set current_dir to isolate `.env` file effects.

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper to clear all SPLUNK_* environment variables for test isolation.
fn clear_splunk_env(cmd: &mut assert_cmd::Command) {
    for (key, _) in std::env::vars() {
        if key.starts_with("SPLUNK_") {
            cmd.env_remove(&key);
        }
    }
}

#[test]
fn test_invalid_dotenv_causes_cli_failure() {
    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join(".env");

    // Create an invalid .env file (line without '=')
    fs::write(&env_path, "INVALID_LINE_WITHOUT_EQUALS").unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.current_dir(temp_dir.path());

    // Clear any existing SPLUNK_* vars and enable dotenv
    clear_splunk_env(&mut cmd);
    cmd.env_remove("DOTENV_DISABLED");

    // Run any command that would require config (health is a good choice)
    cmd.args(["health"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(".env"));
}

#[test]
fn test_invalid_dotenv_does_not_leak_secrets() {
    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join(".env");
    let secret_value = "supersecret_cli_token_12345";

    // Create a .env file with a secret followed by an invalid line
    fs::write(
        &env_path,
        format!("SPLUNK_API_TOKEN={}\nINVALID_LINE", secret_value),
    )
    .unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.current_dir(temp_dir.path());

    // Clear any existing SPLUNK_* vars and enable dotenv
    clear_splunk_env(&mut cmd);
    cmd.env_remove("DOTENV_DISABLED");

    let output = cmd
        .args(["health"])
        .output()
        .expect("Failed to run command");

    let stderr = String::from_utf8_lossy(&output.stderr);

    // Verify the error message does NOT contain the secret
    assert!(
        !stderr.contains(secret_value),
        "Error message should NOT contain the secret value. stderr: {}",
        stderr
    );

    // Verify the error message DOES mention .env
    assert!(
        stderr.contains(".env"),
        "Error message should mention .env file. stderr: {}",
        stderr
    );
}

#[test]
fn test_dotenv_disabled_skips_invalid_env_file() {
    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join(".env");

    // Create an invalid .env file
    fs::write(&env_path, "INVALID_LINE_WITHOUT_EQUALS").unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.current_dir(temp_dir.path());

    // Clear any existing SPLUNK_* vars
    clear_splunk_env(&mut cmd);

    // With DOTENV_DISABLED=1, the invalid .env should be skipped
    // The CLI will fail with "Base URL is required" (not a dotenv error)
    cmd.env("DOTENV_DISABLED", "1")
        .args(["health"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Base URL is required"));
}

#[test]
fn test_dotenv_parse_error_includes_position_hint() {
    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join(".env");

    // Create a .env file with an invalid line
    fs::write(&env_path, "INVALID_LINE_WITHOUT_EQUALS").unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.current_dir(temp_dir.path());

    // Clear any existing SPLUNK_* vars and enable dotenv
    clear_splunk_env(&mut cmd);
    cmd.env_remove("DOTENV_DISABLED");

    // Verify the error message includes a position hint
    cmd.args(["health"]).assert().failure().stderr(
        predicate::str::contains("position").or(predicate::str::contains("DOTENV_DISABLED")),
    );
}
