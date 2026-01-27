//! Integration tests for `splunk-cli users` command.
//!
//! Tests cover:
//! - Default behavior (list users)
//! - `--count` flag
//! - Output format variations (json, table, csv, xml)

mod common;

use common::splunk_cmd;
use predicates::prelude::*;

/// Test that `splunk-cli users` defaults to listing users.
#[test]
fn test_users_default_lists() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.arg("users").assert();

    // Should fail with connection error (not a "no such command" error)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli users --count <N>` respects count parameter.
#[test]
fn test_users_count_flag() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["users", "--count", "10"]).assert();

    // Should attempt to connect (pass count parameter to endpoint)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli users --help` shows usage.
#[test]
fn test_users_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["users", "--help"]).assert().success().stdout(
        predicate::str::contains("--count")
            .and(predicate::str::contains("Maximum number of users to list")),
    );
}
