//! Secure input tests for `splunk-cli config set`.
//!
//! Tests --password-stdin, --password-file, --api-token-stdin, --api-token-file,
//! --no-prompt, and argument conflict validation.

use crate::common::splunk_cmd;
use crate::config::setup_temp_config;
use predicates::prelude::*;
use std::fs;

/// Test that --password and --password-stdin conflict.
#[test]
fn test_password_and_password_stdin_conflict() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--password",
            "testpass",
            "--password-stdin",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

/// Test that --password and --password-file conflict.
#[test]
fn test_password_and_password_file_conflict() {
    let (_temp_dir, config_path) = setup_temp_config();
    let password_file = _temp_dir.path().join("pass.txt");

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--password",
            "testpass",
            "--password-file",
            password_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

/// Test that --password-stdin and --password-file conflict.
#[test]
fn test_password_stdin_and_password_file_conflict() {
    let (_temp_dir, config_path) = setup_temp_config();
    let password_file = _temp_dir.path().join("pass.txt");

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--password-stdin",
            "--password-file",
            password_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

/// Test that --api-token and --api-token-stdin conflict.
#[test]
fn test_api_token_and_api_token_stdin_conflict() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--api-token",
            "testtoken",
            "--api-token-stdin",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

/// Test that --api-token-stdin and --api-token-file conflict.
#[test]
fn test_api_token_stdin_and_api_token_file_conflict() {
    let (_temp_dir, config_path) = setup_temp_config();
    let token_file = _temp_dir.path().join("token.txt");

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--api-token-stdin",
            "--api-token-file",
            token_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

/// Test reading password from file.
#[test]
fn test_password_file_reads_content() {
    let (_temp_dir, config_path) = setup_temp_config();
    let password_file = _temp_dir.path().join("password.txt");
    fs::write(&password_file, "secret-from-file\n").unwrap();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--username",
            "admin",
            "--password-file",
            password_file.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("saved successfully"));

    // Verify profile was created with username
    let content = fs::read_to_string(&config_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(json["profiles"]["test-profile"]["username"], "admin");
}

/// Test reading API token from file.
#[test]
fn test_api_token_file_reads_content() {
    let (_temp_dir, config_path) = setup_temp_config();
    let token_file = _temp_dir.path().join("token.txt");
    fs::write(&token_file, "token-from-file\n").unwrap();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--api-token-file",
            token_file.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("saved successfully"));
}

/// Test that empty password file fails.
#[test]
fn test_password_file_empty_fails() {
    let (_temp_dir, config_path) = setup_temp_config();
    let password_file = _temp_dir.path().join("password.txt");
    fs::write(&password_file, "   \n").unwrap(); // Whitespace only

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--username",
            "admin",
            "--password-file",
            password_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("empty"));
}

/// Test that missing password file fails.
#[test]
fn test_password_file_missing_fails() {
    let (_temp_dir, config_path) = setup_temp_config();
    let password_file = _temp_dir.path().join("nonexistent.txt");

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--username",
            "admin",
            "--password-file",
            password_file.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to read"));
}

/// Test reading password from stdin.
#[test]
fn test_password_stdin_reads_content() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--username",
            "admin",
            "--password-stdin",
        ])
        .write_stdin("secret-from-stdin\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("saved successfully"));
}

/// Test reading API token from stdin.
#[test]
fn test_api_token_stdin_reads_content() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--api-token-stdin",
        ])
        .write_stdin("token-from-stdin\n")
        .assert()
        .success()
        .stdout(predicate::str::contains("saved successfully"));
}

/// Test --no-prompt fails fast when credentials are missing.
#[test]
fn test_no_prompt_fails_fast_without_credentials() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .env_remove("SPLUNK_USERNAME")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--username",
            "admin",
            "--no-prompt",
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Failed to validate profile: Password or API token must be provided when using username",
        ));
}

/// Test --no-prompt succeeds when credentials are provided.
#[test]
fn test_no_prompt_succeeds_with_credentials() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--username",
            "admin",
            "--password",
            "testpass",
            "--no-prompt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("saved successfully"));
}

/// Test --no-prompt works with file input.
#[test]
fn test_no_prompt_with_password_file() {
    let (_temp_dir, config_path) = setup_temp_config();
    let password_file = _temp_dir.path().join("password.txt");
    fs::write(&password_file, "secret-from-file\n").unwrap();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--username",
            "admin",
            "--password-file",
            password_file.to_str().unwrap(),
            "--no-prompt",
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("saved successfully"));
}

/// Test that help text includes new options.
#[test]
fn test_config_set_help_shows_secure_options() {
    let output = splunk_cmd()
        .args(["config", "set", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    assert!(output.status.success());
    assert!(
        stdout.contains("--password-stdin"),
        "Help should mention --password-stdin"
    );
    assert!(
        stdout.contains("--password-file"),
        "Help should mention --password-file"
    );
    assert!(
        stdout.contains("--api-token-stdin"),
        "Help should mention --api-token-stdin"
    );
    assert!(
        stdout.contains("--api-token-file"),
        "Help should mention --api-token-file"
    );
    assert!(
        stdout.contains("--no-prompt"),
        "Help should mention --no-prompt"
    );
}
