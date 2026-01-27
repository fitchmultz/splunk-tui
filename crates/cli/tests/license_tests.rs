//! Integration tests for `splunk-cli license` command.

mod common;

use common::splunk_cmd;
use predicates::prelude::*;

/// Test that `splunk-cli license --help` shows the command.
#[test]
fn test_license_help() {
    let mut cmd = splunk_cmd();
    cmd.args(["license", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Show license information"));
}

/// Test that `splunk-cli license` executes and tries to connect.
#[test]
fn test_license_execution() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");
    cmd.arg("license")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}
