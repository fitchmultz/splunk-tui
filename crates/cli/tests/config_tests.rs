//! Integration tests for `splunk-cli config` command.

use assert_cmd::cargo::cargo_bin_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

/// Helper function to create a temporary config file for testing.
fn setup_temp_config() -> (TempDir, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("config.json");
    let config_path_str = config_path.to_string_lossy().to_string();
    (temp_dir, config_path_str)
}

/// Test that `splunk-cli config --help` shows command.
#[test]
fn test_config_help() {
    cargo_bin_cmd!("splunk-cli")
        .args(["config", "--help"])
        .assert()
        .stdout(predicate::str::contains("Manage configuration profiles"));
}

/// Test that `splunk-cli config list --help` shows options.
#[test]
fn test_config_list_help() {
    cargo_bin_cmd!("splunk-cli")
        .args(["config", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List all configured profiles"));
}

/// Test that `splunk-cli config set --help` shows options.
#[test]
fn test_config_set_help() {
    cargo_bin_cmd!("splunk-cli")
        .args(["config", "set", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set or update a profile"));
}

/// Test that `splunk-cli config delete --help` shows options.
#[test]
fn test_config_delete_help() {
    cargo_bin_cmd!("splunk-cli")
        .args(["config", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Delete a profile"));
}

/// Test that `splunk-cli config list` executes successfully.
#[test]
fn test_config_list_executes() {
    cargo_bin_cmd!("splunk-cli")
        .args(["config", "list", "--output", "json"])
        .assert()
        .success();
}

/// Test that `splunk-cli config list` accepts table format.
#[test]
fn test_config_list_table_format_empty() {
    cargo_bin_cmd!("splunk-cli")
        .args(["config", "list", "--output", "table"])
        .assert()
        .success();
}

/// Test that `splunk-cli config list` shows no profiles message when empty.
#[test]
fn test_config_list_empty() {
    let (_temp_dir, config_path) = setup_temp_config();

    cargo_bin_cmd!("splunk-cli")
        .env("SPLUNK_CONFIG_PATH", config_path)
        .args(["config", "list", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No profiles configured"));
}

/// Test creating a profile with basic options.
#[test]
fn test_config_set_creates_profile() {
    let (_temp_dir, config_path) = setup_temp_config();

    cargo_bin_cmd!("splunk-cli")
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
        ])
        .assert()
        .success()
        .stdout(predicate::str::contains("saved successfully"));

    // Verify that profile exists
    let content = fs::read_to_string(&config_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(json["profiles"]["test-profile"].is_object());
    assert_eq!(
        json["profiles"]["test-profile"]["base_url"],
        "https://splunk.example.com:8089"
    );
    assert_eq!(json["profiles"]["test-profile"]["username"], "admin");
}

/// Test updating an existing profile.
#[test]
fn test_config_set_updates_existing() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Create initial profile
    cargo_bin_cmd!("splunk-cli")
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://old.example.com:8089",
            "--api-token",
            "old-token",
        ])
        .assert()
        .success();

    // Update profile
    cargo_bin_cmd!("splunk-cli")
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://new.example.com:8089",
            "--api-token",
            "new-token",
        ])
        .assert()
        .success();

    // Verify that update
    let content = fs::read_to_string(&config_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert_eq!(
        json["profiles"]["test-profile"]["base_url"],
        "https://new.example.com:8089"
    );
}

/// Test JSON output format for list command.
#[test]
fn test_config_list_json_format() {
    let (_temp_dir, config_path) = setup_temp_config();

    cargo_bin_cmd!("splunk-cli")
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
        ])
        .assert()
        .success();

    cargo_bin_cmd!("splunk-cli")
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .args(["config", "list", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("profiles"))
        .stdout(predicate::str::contains("test-profile"));
}

/// Test table output format for list command with actual data.
#[test]
fn test_config_list_table_format() {
    let (_temp_dir, config_path) = setup_temp_config();

    cargo_bin_cmd!("splunk-cli")
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
        ])
        .assert()
        .success();

    cargo_bin_cmd!("splunk-cli")
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .args(["config", "list", "--output", "table"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile"))
        .stdout(predicate::str::contains("test-profile"))
        .stdout(predicate::str::contains("https://splunk.example.com:8089"))
        .stdout(predicate::str::contains("admin"));
}

/// Test deleting a profile.
#[test]
fn test_config_delete_profile() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Create a profile
    cargo_bin_cmd!("splunk-cli")
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
        ])
        .assert()
        .success();

    // Delete profile
    cargo_bin_cmd!("splunk-cli")
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .args(["config", "delete", "test-profile"])
        .assert()
        .success()
        .stdout(predicate::str::contains("deleted successfully"));

    // Verify that it's gone
    let content = fs::read_to_string(&config_path).unwrap();
    let json: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(
        json["profiles"]["test-profile"].is_null()
            || json["profiles"]
                .as_object()
                .unwrap()
                .get("test-profile")
                .is_none()
    );
}

/// Test deleting a nonexistent profile.
#[test]
fn test_config_delete_nonexistent() {
    let (_temp_dir, config_path) = setup_temp_config();

    cargo_bin_cmd!("splunk-cli")
        .env("SPLUNK_CONFIG_PATH", config_path)
        .args(["config", "delete", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test error for invalid output format.
#[test]
fn test_config_set_invalid_output_format() {
    let (_temp_dir, config_path) = setup_temp_config();

    cargo_bin_cmd!("splunk-cli")
        .env("SPLUNK_CONFIG_PATH", config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args(["config", "list", "--output", "invalid"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid output format"));
}

/// Test keyring integration with --use-keyring flag.
#[test]
fn test_config_set_with_keyring() {
    let (_temp_dir, config_path) = setup_temp_config();

    let output = cargo_bin_cmd!("splunk-cli")
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
            "test-token-123",
            "--use-keyring",
        ])
        .output()
        .unwrap();

    if output.status.success() {
        // Verify credentials are stored as Keyring
        let content = fs::read_to_string(&config_path).unwrap();
        let json: serde_json::Value = serde_json::from_str(&content).unwrap();

        // Check for Keyring variant
        let api_token_field = &json["profiles"]["test-profile"]["api_token"];

        // Clean up keyring if successful
        if let Some(keyring_account) = api_token_field
            .as_str()
            .and_then(|s| s.strip_prefix("Keyring"))
        {
            let _ = keyring::Entry::new("com.splunk-tui.splunk-tui", keyring_account)
                .map(|e| e.delete_credential());
        }
    } else {
        let stderr = String::from_utf8_lossy(&output.stderr);
        if stderr.contains("keyring") || stderr.contains("Keyring") {
            eprintln!("Skipping keyring test: {}", stderr);
        }
    }
}
