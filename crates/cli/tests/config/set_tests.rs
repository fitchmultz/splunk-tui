//! Set/create/update tests for `splunk-cli config`.
//!
//! Tests profile creation, updates, and output format integration.

use crate::common::splunk_cmd;
use crate::config::setup_temp_config;
use predicates::prelude::*;
use std::fs;

/// Test creating a profile with basic options.
#[test]
fn test_config_set_creates_profile() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
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
    splunk_cmd()
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
    splunk_cmd()
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
        ])
        .assert()
        .success();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .args(["-o", "json", "config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("profiles"))
        .stdout(predicate::str::contains("test-profile"));
}

/// Test table output format for list command with actual data.
#[test]
fn test_config_list_table_format() {
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
        ])
        .assert()
        .success();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .args(["-o", "table", "config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile"))
        .stdout(predicate::str::contains("test-profile"))
        .stdout(predicate::str::contains("https://splunk.example.com:8089"))
        .stdout(predicate::str::contains("admin"));
}
