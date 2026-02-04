//! Integration tests for `splunk-cli health` command.

mod common;

use common::{connection_error_predicate, splunk_cmd};
use predicates::prelude::*;

/// Test that `splunk-cli health --help` shows the command.
#[test]
fn test_health_help() {
    let mut cmd = splunk_cmd();
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
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");
    cmd.arg("health")
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}
