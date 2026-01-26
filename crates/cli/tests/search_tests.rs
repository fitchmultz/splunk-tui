//! Integration tests for `splunk-cli search` command.

use predicates::prelude::*;

const TEST_BASE_URL: &str = "https://localhost:8089";
const TEST_QUERY: &str = "search index=_internal | head 1";

fn splunk_cli_cmd() -> assert_cmd::Command {
    assert_cmd::cargo::cargo_bin_cmd!("splunk-cli")
}

fn splunk_cli_cmd_with_base_url() -> assert_cmd::Command {
    let mut cmd = splunk_cli_cmd();
    cmd.env("SPLUNK_BASE_URL", TEST_BASE_URL);
    cmd
}

#[test]
fn test_search_help() {
    let mut cmd = splunk_cli_cmd();
    cmd.args(["search", "--help"]).assert().success().stdout(
        predicate::str::contains("Execute a search query")
            .and(predicate::str::contains("--wait"))
            .and(predicate::str::contains("--earliest"))
            .and(predicate::str::contains("--latest"))
            .and(predicate::str::contains("--count")),
    );
}

#[test]
fn test_search_requires_query_argument() {
    let mut cmd = splunk_cli_cmd();
    cmd.arg("search")
        .assert()
        .failure()
        .stderr(predicate::str::contains("<QUERY>").or(predicate::str::contains("<query>")));
}

#[test]
fn test_search_with_query_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["search", TEST_QUERY])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

#[test]
fn test_search_with_wait_flag_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["search", TEST_QUERY, "--wait"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

#[test]
fn test_search_with_earliest_and_latest_flags_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args([
        "search",
        TEST_QUERY,
        "--earliest",
        "-24h",
        "--latest",
        "now",
    ])
    .assert()
    .failure()
    .stderr(predicate::str::contains("Connection refused"));
}

#[test]
fn test_search_with_count_flag_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["search", TEST_QUERY, "--count", "10"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

#[test]
fn test_search_with_output_json_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["--output", "json", "search", TEST_QUERY])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

#[test]
fn test_search_with_output_csv_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["--output", "csv", "search", TEST_QUERY])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

#[test]
fn test_search_with_output_table_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["--output", "table", "search", TEST_QUERY])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

#[test]
fn test_search_with_output_xml_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["--output", "xml", "search", TEST_QUERY])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}
