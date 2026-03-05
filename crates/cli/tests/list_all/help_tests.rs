//! Help flag and documentation tests for `splunk-cli list-all`.
//!
//! Tests verify that --help output contains expected options and descriptions.

use crate::common::splunk_cmd;
use predicates::prelude::*;

/// Test that `splunk-cli list-all --help` shows command.
#[test]
fn test_list_all_help() {
    let mut cmd = splunk_cmd();
    cmd.args(["list-all", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "List all Splunk resources in unified overview",
        ))
        .stdout(predicate::str::contains("--resources"))
        .stdout(predicate::str::contains(
            "Optional comma-separated list of resource types",
        ));
}

/// Test that `splunk-cli list-all --help` shows multi-profile options.
#[test]
fn test_list_all_help_shows_multi_profile_options() {
    let mut cmd = splunk_cmd();
    cmd.args(["list-all", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--profiles"))
        .stdout(predicate::str::contains("--all-profiles"));
}
