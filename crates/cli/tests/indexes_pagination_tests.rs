//! Integration tests for `splunk-cli indexes` pagination flags.

mod common;

use common::splunk_cmd;
use predicates::prelude::*;

#[test]
fn test_indexes_help_includes_offset() {
    let mut cmd = splunk_cmd();

    cmd.args(["indexes", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--offset").and(predicate::str::contains("Offset")));
}

#[test]
fn test_indexes_offset_flag_attempts_connection() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // We only assert that clap accepts the flags and command attempts to run.
    // In CI/no-live-Splunk, this should fail with a connection error.
    cmd.args(["indexes", "--count", "10", "--offset", "10"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

#[test]
fn test_indexes_offset_negative_rejected_by_clap() {
    let mut cmd = splunk_cmd();

    cmd.args(["indexes", "--offset", "-1"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("unexpected argument"));
}
