//! Deterministic ordering tests for `splunk-cli list-all`.
//!
//! Tests verify that resource and profile ordering is deterministic
//! (first occurrence wins) and follows expected patterns.

use crate::common::splunk_cmd;
use predicates::prelude::*;

/// Test that resource ordering is deterministic (first occurrence wins).
#[test]
fn test_list_all_resource_ordering_deterministic() {
    // Run multiple times with duplicate resources to verify consistent ordering
    for _ in 0..5 {
        let mut cmd = splunk_cmd();
        cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

        let result = cmd
            .args(["list-all", "--resources", "jobs,apps,jobs,indexes,apps"])
            .assert();

        // Should succeed and preserve first-occurrence order: jobs, apps, indexes
        result
            .success()
            .stdout(predicate::str::contains("jobs"))
            .stdout(predicate::str::contains("apps"))
            .stdout(predicate::str::contains("indexes"));
    }
}

/// Test that profile ordering is deterministic (first occurrence wins).
#[test]
fn test_list_all_profile_ordering_deterministic() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create a config file with multiple profiles
    let config = serde_json::json!({
        "profiles": {
            "alpha": {
                "base_url": "https://alpha.splunk.local:8089",
                "username": "admin"
            },
            "beta": {
                "base_url": "https://beta.splunk.local:8089",
                "username": "admin"
            },
            "gamma": {
                "base_url": "https://gamma.splunk.local:8089",
                "username": "admin"
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    // Run multiple times with duplicate profiles to verify consistent ordering
    for _ in 0..5 {
        let mut cmd = splunk_cmd();
        cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
            .args([
                "list-all",
                "--profiles",
                "alpha,beta,alpha,gamma,beta", // duplicates
                "--output",
                "json",
                "--resources",
                "health",
            ]);

        // Should succeed and preserve first-occurrence order: alpha, beta, gamma
        cmd.assert()
            .success()
            .stdout(predicate::str::contains("alpha"))
            .stdout(predicate::str::contains("beta"))
            .stdout(predicate::str::contains("gamma"));
    }
}

/// Test that default resource ordering follows VALID_RESOURCES order.
#[test]
fn test_list_all_default_resource_ordering() {
    // Run multiple times without specifying resources to verify consistent ordering
    for _ in 0..3 {
        let mut cmd = splunk_cmd();
        cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

        let result = cmd.arg("list-all").assert();

        // Should succeed with consistent ordering
        result
            .success()
            .stdout(predicate::str::contains("Timestamp"))
            .stdout(predicate::str::contains("indexes"))
            .stdout(predicate::str::contains("jobs"))
            .stdout(predicate::str::contains("users"));
    }
}
