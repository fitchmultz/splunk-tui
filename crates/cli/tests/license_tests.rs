//! Integration tests for `splunk license` command.

use predicates::prelude::*;

/// Test that `splunk license --help` shows the command.
#[test]
fn test_license_help() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk");
    cmd.args(["license", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Show license information"));
}

/// Test that `splunk license` executes and tries to connect.
#[test]
fn test_license_execution() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk");
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");
    cmd.arg("license")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}
