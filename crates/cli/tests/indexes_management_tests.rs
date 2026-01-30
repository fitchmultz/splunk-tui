//! Integration tests for `splunk-cli indexes` create, modify, and delete commands.

mod common;

use common::splunk_cmd;
use predicates::prelude::*;

#[test]
fn test_indexes_create_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["indexes", "create", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--name").or(predicate::str::contains("<NAME>")));
}

#[test]
fn test_indexes_create_requires_name() {
    let mut cmd = splunk_cmd();

    cmd.args(["indexes", "create"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("argument")));
}

#[test]
fn test_indexes_modify_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["indexes", "modify", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--max-data-size-mb")
                .or(predicate::str::contains("max-data-size")),
        );
}

#[test]
fn test_indexes_modify_requires_name() {
    let mut cmd = splunk_cmd();

    cmd.args(["indexes", "modify"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("argument")));
}

#[test]
fn test_indexes_delete_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["indexes", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--force").or(predicate::str::contains("force")));
}

#[test]
fn test_indexes_delete_requires_name() {
    let mut cmd = splunk_cmd();

    cmd.args(["indexes", "delete"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("required").or(predicate::str::contains("argument")));
}

#[test]
fn test_indexes_create_attempts_connection() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // We only assert that clap accepts the flags and command attempts to run.
    // In CI/no-live-Splunk, this should fail with a connection error.
    cmd.args([
        "indexes",
        "create",
        "test_index",
        "--max-data-size-mb",
        "1000",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("Connection refused"));
}

#[test]
fn test_indexes_modify_attempts_connection() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    cmd.args(["indexes", "modify", "main", "--max-data-size-mb", "2000"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

#[test]
fn test_indexes_delete_attempts_connection() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    cmd.args(["indexes", "delete", "test_index", "--force"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}
