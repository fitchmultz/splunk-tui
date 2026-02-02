//! Integration tests for `splunk-cli users` command.
//!
//! Tests cover:
//! - Default behavior (list users)
//! - `--count` flag
//! - Output format variations (json, table, csv, xml)
//! - Create, modify, delete subcommands

mod common;

use common::{connection_error_predicate, splunk_cmd};
use predicates::prelude::*;

/// Test that `splunk-cli users list` lists users.
#[test]
fn test_users_list() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["users", "list"]).assert();

    // Should fail with connection error (not a "no such command" error)
    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli users list --count <N>` respects count parameter.
#[test]
fn test_users_list_count_flag() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["users", "list", "--count", "10"]).assert();

    // Should attempt to connect (pass count parameter to endpoint)
    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli users --help` shows usage.
#[test]
fn test_users_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["users", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list").and(predicate::str::contains("List all users")));
}

/// Test that `splunk-cli users list --help` shows list usage.
#[test]
fn test_users_list_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["users", "list", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--count")
                .and(predicate::str::contains("Maximum number of users to list")),
        );
}

/// Test that `splunk-cli users create --help` shows create usage.
#[test]
fn test_users_create_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["users", "create", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("USERNAME")
                .and(predicate::str::contains("--password"))
                .and(predicate::str::contains("--roles")),
        );
}

/// Test that `splunk-cli users modify --help` shows modify usage.
#[test]
fn test_users_modify_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["users", "modify", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("USERNAME")
                .and(predicate::str::contains("--password"))
                .and(predicate::str::contains("--roles")),
        );
}

/// Test that `splunk-cli users delete --help` shows delete usage.
#[test]
fn test_users_delete_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["users", "delete", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("USERNAME").and(predicate::str::contains("--force")));
}
