//! Integration tests for `splunk-cli kvstore` command.

use predicates::prelude::*;

#[test]
fn test_kvstore_help() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.arg("kvstore").arg("--help");
    cmd.assert().success();
}

#[test]
fn test_kvstore_invalid_format() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.arg("kvstore").arg("--output").arg("invalid");
    cmd.assert().failure();
}

#[test]
fn test_kvstore_execution() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");
    cmd.arg("kvstore")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}
