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
    splunk_cmd()
        .args(["config", "set", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Set or update a profile"));
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
