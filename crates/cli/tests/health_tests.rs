//! Integration tests for `splunk-cli health` command.

use predicates::prelude::*;

/// Test that `splunk-cli health --help` shows the command.
#[test]
fn test_health_help() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.args(["health", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Perform a comprehensive system health check",
        ));
}

/// Test that `splunk-cli health` executes and tries to connect.
#[test]
fn test_health_execution() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");
    cmd.arg("health")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}
