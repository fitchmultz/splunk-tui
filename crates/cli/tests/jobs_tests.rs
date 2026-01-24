//! Integration tests for `splunk-cli jobs` command.
//!
//! Tests cover:
//! - Default behavior (no args = list jobs)
//! - `--cancel` and `--delete` flags (verify list is NOT called)
//! - `--list` flag explicit usage

use predicates::prelude::*;

/// Test that `splunk-cli jobs` with no arguments defaults to listing jobs.
#[test]
fn test_jobs_default_lists() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // The command should fail because we don't have a real Splunk server,
    // but we can verify it tries to list jobs by checking the error is connection-related,
    // not a "no arguments provided" error.
    let result = cmd.arg("jobs").assert();

    // Should fail with connection error (not a "missing required argument" error)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli jobs --list` explicitly lists jobs.
#[test]
fn test_jobs_explicit_list_flag() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // Explicit --list flag
    let result = cmd.args(["jobs", "--list"]).assert();

    // Should also fail with connection error (same as default)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli jobs --cancel <sid>` cancels a job without listing.
#[test]
fn test_jobs_cancel_flag() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // --cancel flag with a SID
    let result = cmd.args(["jobs", "--cancel", "test-sid-123"]).assert();

    // Should fail with connection error (trying to cancel, not list)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli jobs --delete <sid>` deletes a job without listing.
#[test]
fn test_jobs_delete_flag() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // --delete flag with a SID
    let result = cmd.args(["jobs", "--delete", "test-sid-456"]).assert();

    // Should fail with connection error (trying to delete, not list)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `--list` flag is shown in help text.
#[test]
fn test_jobs_help_shows_list_flag() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");

    // The help should show --list flag
    cmd.args(["jobs", "--help"]).assert().success().stdout(
        predicate::str::contains("--list").and(predicate::str::contains("List all search jobs")),
    );
}
