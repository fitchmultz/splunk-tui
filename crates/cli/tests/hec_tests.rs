//! Integration tests for `splunk-cli hec` command.

mod common;

use common::splunk_cmd;
use predicates::prelude::*;

/// Test that `splunk-cli hec --help` shows the command.
#[test]
fn test_hec_help() {
    let mut cmd = splunk_cmd();
    cmd.args(["hec", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "Send events to Splunk via HTTP Event Collector",
        ));
}

/// Test that `splunk-cli hec send --help` shows the subcommand.
#[test]
fn test_hec_send_help() {
    let mut cmd = splunk_cmd();
    cmd.args(["hec", "send", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Send a single event to HEC"));
}

/// Test that `splunk-cli hec send-batch --help` shows the subcommand.
#[test]
fn test_hec_send_batch_help() {
    let mut cmd = splunk_cmd();
    cmd.args(["hec", "send-batch", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Send a batch of events to HEC"));
}

/// Test that `splunk-cli hec health --help` shows the subcommand.
#[test]
fn test_hec_health_help() {
    let mut cmd = splunk_cmd();
    cmd.args(["hec", "health", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Check HEC health endpoint"));
}

/// Test that `splunk-cli hec check-ack --help` shows the subcommand.
#[test]
fn test_hec_check_ack_help() {
    let mut cmd = splunk_cmd();
    cmd.args(["hec", "check-ack", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Check acknowledgment status"));
}

/// Test that `hec send` requires hec_url and hec_token.
#[test]
fn test_hec_send_requires_url_and_token() {
    let mut cmd = splunk_cmd();
    cmd.args(["hec", "send", "{}", "--hec-url", "https://localhost:8088"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("hec-token").or(predicate::str::contains("HEC_TOKEN")));
}

/// Test that `hec send-batch` requires a file.
#[test]
fn test_hec_send_batch_requires_file() {
    let mut cmd = splunk_cmd();
    cmd.args([
        "hec",
        "send-batch",
        "--hec-url",
        "https://localhost:8088",
        "--hec-token",
        "test",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("required"));
}

/// Test that `hec health` executes and tries to connect.
#[test]
fn test_hec_health_execution() {
    let mut cmd = splunk_cmd();
    cmd.args([
        "hec",
        "health",
        "--hec-url",
        "https://localhost:8088",
        "--hec-token",
        "test",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("Connection refused").or(predicate::str::contains("error")));
}

/// Test that `hec send` executes and tries to connect.
#[test]
fn test_hec_send_execution() {
    let mut cmd = splunk_cmd();
    cmd.args([
        "hec",
        "send",
        r#"{"message": "test"}"#,
        "--hec-url",
        "https://localhost:8088",
        "--hec-token",
        "test",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("Connection refused").or(predicate::str::contains("error")));
}

/// Test that `hec check-ack` requires ack_ids.
#[test]
fn test_hec_check_ack_requires_ids() {
    let mut cmd = splunk_cmd();
    cmd.args([
        "hec",
        "check-ack",
        "--hec-url",
        "https://localhost:8088",
        "--hec-token",
        "test",
    ])
    .assert()
    .failure()
    .stderr(
        predicate::str::contains("ack-ids").or(predicate::str::contains("No acknowledgment IDs")),
    );
}
