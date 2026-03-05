//! Delete command tests for `splunk-cli config`.
//!
//! Tests profile deletion and error handling for nonexistent profiles.

use crate::common::splunk_cmd;
use crate::config::setup_temp_config;
use predicates::prelude::*;
use std::fs;

/// Test deleting a profile.
#[test]
fn test_config_delete_profile() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Create a profile
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
        ])
        .assert()
        .success();

    // Delete profile
    splunk_cmd()
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

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", config_path)
        .args(["config", "delete", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}
