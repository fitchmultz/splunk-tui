//! Integration tests for `splunk-cli saved-searches` command.
//!
//! Tests cover:
//! - Help text verification for all subcommands (list, run, info, edit)
//! - Output format parsing validation (json, table, csv, xml)
//!
//! Does NOT:
//! - Test live Splunk server interactions (see `test-live` in Makefile).
//!
//! Invariants / Assumptions:
//! - All tests use hermetic CLI commands via `splunk_cmd()` to prevent env leakage.

mod common;

use common::splunk_cmd;
use predicates::prelude::*;

#[test]
fn test_saved_searches_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["saved-searches", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List saved searches"))
        .stdout(predicate::str::contains("Show detailed information"))
        .stdout(predicate::str::contains("Run a saved search"))
        .stdout(predicate::str::contains("Edit a saved search"));
}

#[test]
fn test_saved_searches_list_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["saved-searches", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--count"));
}

#[test]
fn test_saved_searches_run_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["saved-searches", "run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--wait"))
        .stdout(predicate::str::contains("--earliest"))
        .stdout(predicate::str::contains("--latest"))
        .stdout(predicate::str::contains("--count"));
}

#[test]
fn test_saved_searches_info_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["saved-searches", "info", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<NAME>"));
}

#[test]
fn test_saved_searches_edit_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["saved-searches", "edit", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<NAME>"))
        .stdout(predicate::str::contains("--search"))
        .stdout(predicate::str::contains("--description"))
        .stdout(predicate::str::contains("--disabled"));
}

#[test]
fn test_saved_searches_list_output_format_parsing() {
    let formats = ["json", "table", "csv", "xml"];

    for format in formats {
        let mut cmd = splunk_cmd();
        cmd.args(["saved-searches", "list", "--output", format, "--help"])
            .assert()
            .success();
    }
}
