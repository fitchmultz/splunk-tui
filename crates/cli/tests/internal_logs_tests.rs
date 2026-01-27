//! Integration tests for `splunk-cli internal-logs` command.

mod common;

use common::splunk_cmd;
use predicates::prelude::*;

/// Test that `splunk-cli internal-logs` works with defaults.
#[test]
fn test_internal_logs_default() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.arg("internal-logs").assert();

    // Should fail with connection error (not a "command not found" error)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli internal-logs --count` works.
#[test]
fn test_internal_logs_with_count() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["internal-logs", "--count", "10"]).assert();

    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli internal-logs --earliest` works.
#[test]
fn test_internal_logs_with_earliest() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd
        .args(["internal-logs", "--earliest", "2024-01-01T00:00:00"])
        .assert();

    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `--count` and `--earliest` are shown in help text.
#[test]
fn test_internal_logs_help_shows_flags() {
    let mut cmd = splunk_cmd();

    cmd.args(["internal-logs", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--count").and(predicate::str::contains("--earliest")));
}

/// Test that output format option works.
#[test]
fn test_internal_logs_output_format() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["internal-logs", "-o", "json"]).assert();

    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}
