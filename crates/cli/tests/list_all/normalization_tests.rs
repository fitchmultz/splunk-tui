//! Input normalization tests for `splunk-cli list-all`.
//!
//! Tests verify whitespace trimming, case insensitivity, and deduplication
//! for both resource names and profile names.

use crate::common::splunk_cmd;
use predicates::prelude::*;

/// Test that resources with whitespace are normalized (trimmed).
#[test]
fn test_list_all_whitespace_in_resources() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // Test with whitespace around resource names
    let result = cmd
        .args(["list-all", "--resources", " jobs , apps , indexes "])
        .assert();

    // Should succeed (whitespace is trimmed, resources are valid)
    result
        .success()
        .stdout(predicate::str::contains("jobs"))
        .stdout(predicate::str::contains("apps"))
        .stdout(predicate::str::contains("indexes"));
}

/// Test that resources are case-insensitive (normalized to lowercase).
#[test]
fn test_list_all_case_insensitive_resources() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // Test with uppercase and mixed case resource names
    let result = cmd
        .args(["list-all", "--resources", "JOBS,Apps,INDEXES"])
        .assert();

    // Should succeed (case is normalized to lowercase)
    result
        .success()
        .stdout(predicate::str::contains("jobs"))
        .stdout(predicate::str::contains("apps"))
        .stdout(predicate::str::contains("indexes"));
}

/// Test that duplicate resources are deduplicated.
#[test]
fn test_list_all_deduped_resources() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // Test with duplicate resource names
    let result = cmd
        .args(["list-all", "--resources", "jobs,jobs,apps,apps,apps"])
        .assert();

    // Should succeed (duplicates are removed)
    result
        .success()
        .stdout(predicate::str::contains("jobs"))
        .stdout(predicate::str::contains("apps"));
}

/// Test combined normalization: whitespace + case + dedupe.
#[test]
fn test_list_all_combined_normalization() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // Test with mixed case, whitespace, and duplicates
    let result = cmd
        .args([
            "list-all",
            "--resources",
            " JOBS , jobs , JOBS , apps , Apps ",
        ])
        .assert();

    // Should succeed (all normalized to unique lowercase values)
    result
        .success()
        .stdout(predicate::str::contains("jobs"))
        .stdout(predicate::str::contains("apps"));
}

/// Test that profile names with whitespace are normalized (trimmed).
#[test]
fn test_list_all_whitespace_in_profiles() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create a config file with profiles that have no whitespace
    let config = serde_json::json!({
        "profiles": {
            "dev": {
                "base_url": "https://dev.splunk.local:8089",
                "username": "admin"
            },
            "prod": {
                "base_url": "https://prod.splunk.com:8089",
                "api_token": "prod-token"
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
        .args([
            "list-all",
            "--profiles",
            " dev , prod ", // whitespace around profile names
            "--output",
            "json",
            "--resources",
            "health",
        ]);

    // Should succeed (whitespace is trimmed from profile names)
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("dev"))
        .stdout(predicate::str::contains("prod"))
        .stdout(predicate::str::contains("https://dev.splunk.local:8089"))
        .stdout(predicate::str::contains("https://prod.splunk.com:8089"));
}

/// Test that profile names with whitespace and case are handled correctly.
/// Note: Profile names are case-sensitive (preserved), only whitespace is trimmed.
#[test]
fn test_list_all_profile_case_preserved() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create a config file with mixed-case profile name
    let config = serde_json::json!({
        "profiles": {
            "DevEnv": {
                "base_url": "https://dev.splunk.local:8089",
                "username": "admin"
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
        .args([
            "list-all",
            "--profiles",
            " DevEnv ", // whitespace around profile name
            "--output",
            "json",
            "--resources",
            "health",
        ]);

    // Should succeed (whitespace trimmed, case preserved)
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("DevEnv"))
        .stdout(predicate::str::contains("https://dev.splunk.local:8089"));
}

/// Test that invalid resource type after normalization still fails.
#[test]
fn test_list_all_invalid_resource_after_normalization() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // Test with whitespace around an invalid resource name
    let result = cmd
        .args(["list-all", "--resources", " invalid_resource "])
        .assert();

    // Should fail with validation error
    result
        .failure()
        .stderr(predicate::str::contains("Invalid resource type"))
        .stderr(predicate::str::contains("Valid types:"));
}
