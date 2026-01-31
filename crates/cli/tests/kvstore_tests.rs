//! Integration tests for `splunk-cli kvstore` command.

mod common;

use common::splunk_cmd;
use predicates::prelude::*;

#[test]
fn test_kvstore_help() {
    let mut cmd = splunk_cmd();
    cmd.arg("kvstore").arg("--help");
    cmd.assert().success();
}

#[test]
fn test_kvstore_invalid_format() {
    let mut cmd = splunk_cmd();
    cmd.arg("kvstore").arg("--output").arg("invalid");
    cmd.assert().failure();
}

#[test]
fn test_kvstore_status_execution() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");
    cmd.arg("kvstore")
        .arg("status")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}
