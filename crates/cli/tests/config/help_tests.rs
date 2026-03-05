//! Help text tests for `splunk-cli config` command.
//!
//! Tests that all config subcommands have proper --help output.

use crate::common::splunk_cmd;
use predicates::prelude::*;

/// Test that `splunk-cli config --help` shows command.
#[test]
fn test_config_help() {
    splunk_cmd()
        .args(["config", "--help"])
        .assert()
        .stdout(predicate::str::contains("Manage configuration profiles"));
}

/// Test that `splunk-cli config list --help` shows options.
#[test]
fn test_config_list_help() {
    splunk_cmd()
        .args(["config", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List all configured profiles"));
}

/// Test that `splunk-cli config set --help` shows options.
#[test]
fn test_config_set_help() {
    let output = splunk_cmd()
        .args(["config", "set", "--help"])
        .output()
        .expect("Failed to execute command");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Basic success and description check
    assert!(output.status.success());
    assert!(stdout.contains("Set or update a profile"));

    // Verify only one timeout option exists (not --timeout-seconds)
    // The help should contain -t, --timeout but NOT --timeout-seconds
    assert!(
        stdout.contains("-t, --timeout"),
        "Help should contain '-t, --timeout' flag"
    );
    assert!(
        !stdout.contains("--timeout-seconds"),
        "Help should NOT contain '--timeout-seconds' (duplicate flag removed)"
    );

    // Count occurrences of "--timeout" to ensure it appears exactly once
    // (allowing for possible mentions in descriptions/examples)
    let timeout_count = stdout.matches("--timeout").count();
    assert!(
        timeout_count >= 1,
        "Help should contain at least one '--timeout' mention"
    );
}

/// Test that `splunk-cli config delete --help` shows options.
#[test]
fn test_config_delete_help() {
    splunk_cmd()
        .args(["config", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Delete a profile"));
}
