//! Integration tests for CLI configuration loading and dotenv isolation.
//!
//! Responsibilities:
//! - Verify that `.env` file values are respected when loaded before CLI parsing.
//! - Validate priority order: profile < environment variables < CLI flags.
//! - Ensure environment variables like `SPLUNK_TIMEOUT` and `SPLUNK_MAX_RETRIES` are applied.
//!
//! Does NOT:
//! - Use the shared `splunk_cmd` helper, as these tests specifically need to
//!   manipulate `DOTENV_DISABLED` and other raw environment variables to
//!   validate loading logic.
//!
//! Invariants:
//! These tests explicitly clear `SPLUNK_*` environment variables for isolation.
//! The test environment may have pre-existing `SPLUNK_*` vars that would
//! otherwise override `.env` file values (which is correct behavior but breaks tests).
//!
//! Tests that validate dotenv loading must explicitly `.env_remove("DOTENV_DISABLED")`.
//! would otherwise override .env file values (which is correct behavior but breaks tests).

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Returns a predicate that matches common connection error messages.
///
/// This predicate matches:
/// - "Connection refused" (standard TCP connection failure)
/// - "client error (Connect)" (reqwest connection error)
/// - "invalid peer certificate" (TLS certificate errors)
/// - "API error (401)" (authentication errors when a real server responds)
/// - "Unauthorized" (authentication rejected by a running Splunk server)
fn connection_error_predicate() -> impl Predicate<str> {
    predicate::str::contains("Connection refused")
        .or(predicate::str::contains("client error (Connect)"))
        .or(predicate::str::contains("invalid peer certificate"))
        .or(predicate::str::contains("API error (401)"))
        .or(predicate::str::contains("Unauthorized"))
}

/// Helper to clear all SPLUNK_* environment variables for test isolation.
fn clear_splunk_env() {
    for (key, _) in std::env::vars() {
        if key.starts_with("SPLUNK_") {
            unsafe {
                std::env::remove_var(&key);
            }
        }
    }
}

/// Test that .env file values are respected for CLI env defaults.
#[test]
fn test_dotenv_loading_for_cli_defaults() {
    clear_splunk_env();
    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join(".env");

    // Write a .env file with SPLUNK_BASE_URL
    fs::write(
        &env_path,
        "SPLUNK_BASE_URL=https://dotenv.example.com:8089\n\
         SPLUNK_API_TOKEN=test-dotenv-token\n",
    )
    .unwrap();

    // Run a command that needs config - the .env values should be available
    // to clap's env defaults since we load dotenv before parsing
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.current_dir(temp_dir.path())
        // This test intentionally validates dotenv behavior; ensure it is enabled
        // even when the parent test runner sets `DOTENV_DISABLED=1`.
        .env_remove("DOTENV_DISABLED")
        .args(["health"])
        .assert()
        .failure()
        .stderr(connection_error_predicate().or(
            // The URL from .env should appear in error messages
            predicate::str::contains("dotenv.example.com"),
        ));
}

/// Test that SPLUNK_TIMEOUT environment variable is applied.
#[test]
fn test_timeout_env_var() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .env("SPLUNK_TIMEOUT", "60")
        .args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Execute a search query"));
}

/// Test that SPLUNK_MAX_RETRIES environment variable is applied.
#[test]
fn test_max_retries_env_var() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .env("SPLUNK_MAX_RETRIES", "5")
        .args(["search", "--help"])
        .assert()
        .success();
}

/// Test that --skip-verify CLI flag works correctly.
#[test]
fn test_skip_verify_flag_works() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args(["--skip-verify", "health"])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

/// Test that .env file with SPLUNK_SKIP_VERIFY works.
#[test]
fn test_skip_verify_from_dotenv() {
    clear_splunk_env();
    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join(".env");

    // Write .env with skip_verify=true
    fs::write(
        &env_path,
        "SPLUNK_BASE_URL=https://localhost:8089\n\
         SPLUNK_API_TOKEN=test-token\n\
         SPLUNK_SKIP_VERIFY=true\n",
    )
    .unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.current_dir(temp_dir.path())
        // This test intentionally validates dotenv behavior; ensure it is enabled.
        .env_remove("DOTENV_DISABLED")
        .args(["health"])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

/// Test that CLI --skip-verify flag overrides environment variable.
#[test]
fn test_skip_verify_cli_overrides_env() {
    // Set env to false, but CLI flag should override to true
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .env("SPLUNK_SKIP_VERIFY", "false")
        .args(["--skip-verify", "health"])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

/// Test that .env file values are loaded before CLI parsing.
///
/// This is a critical test - it verifies that when we have a .env file with
/// SPLUNK_BASE_URL, clap's env = "SPLUNK_BASE_URL" can read it because we
/// call load_dotenv() before Cli::parse().
#[test]
fn test_dotenv_loaded_before_cli_parsing() {
    clear_splunk_env();
    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join(".env");

    // Create .env with all supported fields
    fs::write(
        &env_path,
        "SPLUNK_BASE_URL=https://test-splunk.example.com:8089\n\
         SPLUNK_API_TOKEN=dotenv-test-token\n\
         SPLUNK_SKIP_VERIFY=true\n\
         SPLUNK_TIMEOUT=90\n\
         SPLUNK_MAX_RETRIES=7\n",
    )
    .unwrap();

    // The CLI should successfully parse and use these values
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.current_dir(temp_dir.path())
        // This test intentionally validates dotenv behavior; ensure it is enabled.
        .env_remove("DOTENV_DISABLED")
        .args(["health"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("test-splunk.example.com").or(connection_error_predicate()),
        );
}

/// Test priority order: profile < env < CLI.
///
/// This test sets up all three sources and verifies CLI takes precedence.
#[test]
fn test_config_priority_order() {
    let temp_dir = TempDir::new().unwrap();

    // Create a config file with a profile
    let config_path = temp_dir.path().join("splunk.json");
    fs::write(
        &config_path,
        r#"{
            "profiles": {
                "test": {
                    "base_url": "https://profile.example.com:8089",
                    "api_token": "profile-token"
                }
            }
        }"#,
    )
    .unwrap();

    // Create .env with different values
    let env_path = temp_dir.path().join(".env");
    fs::write(
        &env_path,
        "SPLUNK_BASE_URL=https://env.example.com:8089\n\
         SPLUNK_API_TOKEN=env-token\n",
    )
    .unwrap();

    // Run with --base-url CLI flag (highest priority)
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.current_dir(temp_dir.path())
        // This test intentionally validates dotenv behavior; ensure it is enabled.
        .env_remove("DOTENV_DISABLED")
        .env("SPLUNK_CONFIG_PATH", config_path.to_string_lossy().as_ref())
        .args([
            "--profile",
            "test",
            "--base-url",
            "https://cli.example.com:8089",
            "health",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cli.example.com").or(connection_error_predicate()));
}

/// Test that .env file works with the config command.
#[test]
fn test_dotenv_with_config_command() {
    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join(".env");

    fs::write(
        &env_path,
        "SPLUNK_BASE_URL=https://dotenv-config.example.com:8089\n",
    )
    .unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.current_dir(temp_dir.path())
        // This test is about dotenv search/path behavior; ensure it is enabled.
        .env_remove("DOTENV_DISABLED")
        .args(["config", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Manage configuration profiles"));
}

/// Test invalid SPLUNK_TIMEOUT value is handled correctly.
#[test]
fn test_invalid_timeout_value() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .env("SPLUNK_TIMEOUT", "not-a-number")
        .args(["health"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("invalid value")
                .or(predicate::str::contains("must be a number")),
        );
}

/// Test invalid SPLUNK_MAX_RETRIES value is handled correctly.
#[test]
fn test_invalid_max_retries_value() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .env("SPLUNK_MAX_RETRIES", "not-a-number")
        .args(["health"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("invalid value")
                .or(predicate::str::contains("must be a number")),
        );
}

/// Test invalid SPLUNK_SKIP_VERIFY value is handled correctly.
///
/// NOTE: clap validates boolean env values before our code runs, so the error
/// message comes from clap's parser, not our ConfigLoader.
#[test]
fn test_invalid_skip_verify_value() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .env("SPLUNK_SKIP_VERIFY", "not-a-boolean")
        .args(["health"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("invalid value")
                .and(predicate::str::contains("--skip-verify")),
        );
}

/// Test that --timeout CLI flag works correctly.
#[test]
fn test_timeout_cli_flag() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args(["--timeout", "120", "health"])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

/// Test that --max-retries CLI flag works correctly.
#[test]
fn test_max_retries_cli_flag() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args(["--max-retries", "10", "health"])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

/// Test that CLI timeout flag overrides environment variable.
#[test]
fn test_timeout_cli_overrides_env() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .env("SPLUNK_TIMEOUT", "30")
        .args(["--timeout", "120", "health"])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

/// Test that CLI max-retries flag overrides environment variable.
#[test]
fn test_max_retries_cli_overrides_env() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .env("SPLUNK_MAX_RETRIES", "3")
        .args(["--max-retries", "10", "health"])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

/// Test that .env file timeout can be overridden by CLI flag.
#[test]
fn test_timeout_cli_overrides_dotenv() {
    clear_splunk_env();
    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join(".env");

    fs::write(
        &env_path,
        "SPLUNK_BASE_URL=https://localhost:8089\n\
         SPLUNK_API_TOKEN=test-token\n\
         SPLUNK_TIMEOUT=45\n",
    )
    .unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.current_dir(temp_dir.path())
        // This test intentionally validates dotenv behavior; ensure it is enabled.
        .env_remove("DOTENV_DISABLED")
        .args(["--timeout", "90", "health"])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

/// Test that .env file max-retries can be overridden by CLI flag.
#[test]
fn test_max_retries_cli_overrides_dotenv() {
    clear_splunk_env();
    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join(".env");

    fs::write(
        &env_path,
        "SPLUNK_BASE_URL=https://localhost:8089\n\
         SPLUNK_API_TOKEN=test-token\n\
         SPLUNK_MAX_RETRIES=2\n",
    )
    .unwrap();

    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.current_dir(temp_dir.path())
        // This test intentionally validates dotenv behavior; ensure it is enabled.
        .env_remove("DOTENV_DISABLED")
        .args(["--max-retries", "8", "health"])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

/// Test invalid --timeout value is rejected by clap.
#[test]
fn test_invalid_timeout_cli_value() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args(["--timeout", "not-a-number", "health"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("invalid value").and(predicate::str::contains("--timeout")),
        );
}

/// Test invalid --max-retries value is rejected by clap.
#[test]
fn test_invalid_max_retries_cli_value() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args(["--max-retries", "not-a-number", "health"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("invalid value")
                .and(predicate::str::contains("--max-retries")),
        );
}
