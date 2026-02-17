//! Integration tests for `splunk-cli search validate` command.
//!
//! Responsibilities:
//! - Validate SPL validation command and argument handling.
//! - Ensure proper exit codes (0 for valid, 1 for invalid).
//!
//! Does NOT:
//! - Perform live validation against a real Splunk server (see `test-live`).

mod common;

use common::{connection_error_predicate, splunk_cmd};
use predicates::prelude::*;

const TEST_BASE_URL: &str = "https://localhost:8089";

#[test]
fn test_search_validate_help() {
    let mut cmd = splunk_cmd();
    cmd.args(["search", "validate", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Validate SPL syntax")
                .and(predicate::str::contains("--file"))
                .and(predicate::str::contains("--json")),
        );
}

#[test]
fn test_search_validate_requires_query_or_file() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", TEST_BASE_URL);
    cmd.args(["search", "validate"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Failed to validate"));
}

#[test]
fn test_search_validate_with_query_attempts_connection() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", TEST_BASE_URL);
    cmd.args(["search", "validate", "index=main | stats count"])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

#[test]
fn test_search_help_shows_subcommands() {
    let mut cmd = splunk_cmd();
    cmd.args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("execute").and(predicate::str::contains("validate")));
}
