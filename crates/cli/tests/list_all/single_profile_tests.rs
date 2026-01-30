//! Single-profile mode tests for `splunk-cli list-all`.
//!
//! Tests verify backward compatibility with single-profile mode (no --profiles or --all-profiles).
//! These tests use environment variables for configuration.

use crate::common::splunk_cmd;
use predicates::prelude::*;

/// Test that `splunk-cli list-all` defaults to listing all resources.
#[test]
fn test_list_all_default_lists_all() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.arg("list-all").assert();

    // Should succeed (handles errors gracefully, returns results with error status)
    result
        .success()
        .stdout(predicate::str::contains("Timestamp"))
        .stdout(predicate::str::contains("indexes"))
        .stdout(predicate::str::contains("jobs"))
        .stdout(predicate::str::contains("users"));
}

/// Test that `splunk-cli list-all --resources indexes,jobs,users` filters resources.
#[test]
fn test_list_all_filtered_resources() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd
        .args(["list-all", "--resources", "indexes,jobs,users"])
        .assert();

    // Should succeed (handles errors gracefully, returns results with error status)
    result
        .success()
        .stdout(predicate::str::contains("indexes"))
        .stdout(predicate::str::contains("jobs"))
        .stdout(predicate::str::contains("users"))
        .stdout(predicate::str::contains("error")); // Connection errors expected
}

/// Test that `splunk-cli list-all --resources` with single resource works.
#[test]
fn test_list_all_single_resource() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["list-all", "--resources", "indexes"]).assert();

    // Should succeed (handles errors gracefully, returns results with error status)
    result
        .success()
        .stdout(predicate::str::contains("indexes"))
        .stdout(predicate::str::contains("error")); // Connection error expected
}

/// Test that `splunk-cli list-all --resources` with invalid resource type shows error.
#[test]
fn test_list_all_invalid_resource_type() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd
        .args(["list-all", "--resources", "invalid_type"])
        .assert();

    // Should fail with validation error (not connection error)
    result
        .failure()
        .stderr(predicate::str::contains("Invalid resource type"))
        .stderr(predicate::str::contains("Valid types:"));
}

/// Test that list-all has timeout protection and doesn't hang on slow/unresponsive endpoints.
/// This test verifies that individual resource fetches have timeout protection.
#[test]
fn test_list_all_timeout_protection() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");
    cmd.env("SPLUNK_TIMEOUT", "1");

    let result = cmd.args(["list-all", "--resources", "indexes"]).assert();

    // Should complete quickly (not hang) even with short timeout
    // Result will show error status for indexes (connection error expected, not timeout)
    result.success();
}

/// Test that single-profile mode still works (backward compatibility).
#[test]
fn test_list_all_single_profile_backward_compat() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089").args([
        "list-all",
        "--resources",
        "health",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Timestamp"));
}

/// Test that `splunk-cli list-all` handles error gracefully when resources fail to fetch.
/// This test verifies that partial failures don't crash the command.
#[test]
fn test_list_all_error_handling() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.arg("list-all").assert();

    // Should succeed (handles errors gracefully, returns results with error status)
    result.success().stdout(predicate::str::contains("error")); // Errors captured in output
}

/// Test that `splunk-cli list-all --resources` with comma-separated values works.
#[test]
fn test_list_all_comma_separated_resources() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd
        .args([
            "list-all",
            "--resources",
            "indexes,jobs,apps,users,cluster,health,kvstore,license,saved-searches",
        ])
        .assert();

    // Should succeed (handles errors gracefully, returns results with error status)
    result
        .success()
        .stdout(predicate::str::contains("indexes"))
        .stdout(predicate::str::contains("jobs"))
        .stdout(predicate::str::contains("apps"))
        .stdout(predicate::str::contains("users"))
        .stdout(predicate::str::contains("cluster"))
        .stdout(predicate::str::contains("health"))
        .stdout(predicate::str::contains("kvstore"))
        .stdout(predicate::str::contains("license"))
        .stdout(predicate::str::contains("saved-searches"));
}
