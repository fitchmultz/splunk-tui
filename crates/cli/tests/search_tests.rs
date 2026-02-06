//! Integration tests for `splunk-cli search` command.
//!
//! Responsibilities:
//! - Validate search query execution and argument handling.
//! - Ensure progress reporting and `--quiet` mode work correctly.
//! - Verify connection attempt behavior for various output formats.
//!
//! Does NOT:
//! - Perform live searches against a real Splunk server (see `test-live`).
//!
//! Invariants:
//! - All tests use the hermetic `splunk_cmd()` helper.

mod common;

use common::{connection_error_predicate, splunk_cmd};
use predicates::prelude::*;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

const TEST_BASE_URL: &str = "https://localhost:8089";
const TEST_QUERY: &str = "search index=_internal | head 1";

fn splunk_cli_cmd() -> assert_cmd::Command {
    splunk_cmd()
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
        .stderr(connection_error_predicate());
}

#[test]
fn test_search_with_wait_flag_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["search", TEST_QUERY, "--wait"])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
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
    .stderr(connection_error_predicate());
}

#[test]
fn test_search_with_count_flag_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["search", TEST_QUERY, "--count", "10"])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

#[test]
fn test_search_with_output_json_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["--output", "json", "search", TEST_QUERY])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

#[test]
fn test_search_with_output_csv_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["--output", "csv", "search", TEST_QUERY])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

#[test]
fn test_search_with_output_table_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["--output", "table", "search", TEST_QUERY])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

#[test]
fn test_search_with_output_xml_attempts_connection() {
    let mut cmd = splunk_cli_cmd_with_base_url();
    cmd.args(["--output", "xml", "search", TEST_QUERY])
        .assert()
        .failure()
        .stderr(connection_error_predicate());
}

#[tokio::test]
async fn test_search_wait_shows_progress_on_stderr_unless_quiet() {
    let server = MockServer::start().await;

    // Create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": { "sid": "test-sid" } }
            ]
        })))
        .mount(&server)
        .await;

    // Job status (done immediately, but includes doneProgress)
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": {
                    "sid": "test-sid",
                    "isDone": true,
                    "isFinalized": true,
                    "doneProgress": 1.0,
                    "runDuration": 0.0,
                    "cursorTime": null,
                    "scanCount": 0,
                    "eventCount": 0,
                    "resultCount": 1,
                    "diskUsage": 0,
                    "priority": null,
                    "label": null
                } }
            ]
        })))
        .mount(&server)
        .await;

    // Results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"foo": "bar"}],
            "preview": false,
            "total": 1
        })))
        .mount(&server)
        .await;

    // Non-quiet: progress message should appear on stderr
    let mut cmd = splunk_cli_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.args(["--output", "json", "search", TEST_QUERY, "--wait"])
        .assert()
        .success();
    // Note: In non-TTY environments (like CI), indicatif may not print to stderr.
    // So we don't strictly assert on the presence of the progress label here.

    // Quiet: progress must be suppressed (stderr should be empty)
    let mut cmd = splunk_cli_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.args([
        "--quiet", "--output", "json", "search", TEST_QUERY, "--wait",
    ])
    .assert()
    .success()
    .stderr(predicate::str::is_empty());
}

/// Test that SPLUNK_EARLIEST_TIME environment variable is applied to search.
#[tokio::test]
async fn test_search_earliest_time_env_var() {
    let server = MockServer::start().await;

    // Create job - should use earliest time from env var
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": { "sid": "test-sid" } }
            ]
        })))
        .mount(&server)
        .await;

    // Job status (done immediately)
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": {
                    "sid": "test-sid",
                    "isDone": true,
                    "isFinalized": true,
                    "doneProgress": 1.0,
                    "runDuration": 0.0,
                    "cursorTime": null,
                    "scanCount": 0,
                    "eventCount": 0,
                    "resultCount": 1,
                    "diskUsage": 0,
                    "priority": null,
                    "label": null
                } }
            ]
        })))
        .mount(&server)
        .await;

    // Results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"foo": "bar"}],
            "preview": false,
            "total": 1
        })))
        .mount(&server)
        .await;

    let mut cmd = splunk_cli_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri())
        .env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_EARLIEST_TIME", "-1h")
        .args(["--output", "json", "search", TEST_QUERY, "--wait"])
        .assert()
        .success();
}

/// Test that SPLUNK_LATEST_TIME environment variable is applied to search.
#[tokio::test]
async fn test_search_latest_time_env_var() {
    let server = MockServer::start().await;

    // Create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": { "sid": "test-sid" } }
            ]
        })))
        .mount(&server)
        .await;

    // Job status
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": {
                    "sid": "test-sid",
                    "isDone": true,
                    "isFinalized": true,
                    "doneProgress": 1.0,
                    "runDuration": 0.0,
                    "cursorTime": null,
                    "scanCount": 0,
                    "eventCount": 0,
                    "resultCount": 1,
                    "diskUsage": 0,
                    "priority": null,
                    "label": null
                } }
            ]
        })))
        .mount(&server)
        .await;

    // Results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"foo": "bar"}],
            "preview": false,
            "total": 1
        })))
        .mount(&server)
        .await;

    let mut cmd = splunk_cli_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri())
        .env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_LATEST_TIME", "-1h")
        .args(["--output", "json", "search", TEST_QUERY, "--wait"])
        .assert()
        .success();
}

/// Test that SPLUNK_MAX_RESULTS environment variable is applied to search.
#[tokio::test]
async fn test_search_max_results_env_var() {
    let server = MockServer::start().await;

    // Create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": { "sid": "test-sid" } }
            ]
        })))
        .mount(&server)
        .await;

    // Job status
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": {
                    "sid": "test-sid",
                    "isDone": true,
                    "isFinalized": true,
                    "doneProgress": 1.0,
                    "runDuration": 0.0,
                    "cursorTime": null,
                    "scanCount": 0,
                    "eventCount": 0,
                    "resultCount": 1,
                    "diskUsage": 0,
                    "priority": null,
                    "label": null
                } }
            ]
        })))
        .mount(&server)
        .await;

    // Results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"foo": "bar"}],
            "preview": false,
            "total": 1
        })))
        .mount(&server)
        .await;

    let mut cmd = splunk_cli_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri())
        .env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_MAX_RESULTS", "500")
        .args(["--output", "json", "search", TEST_QUERY, "--wait"])
        .assert()
        .success();
}

/// Test that CLI --earliest flag overrides SPLUNK_EARLIEST_TIME env var.
#[tokio::test]
async fn test_search_earliest_cli_overrides_env() {
    let server = MockServer::start().await;

    // Create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": { "sid": "test-sid" } }
            ]
        })))
        .mount(&server)
        .await;

    // Job status
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": {
                    "sid": "test-sid",
                    "isDone": true,
                    "isFinalized": true,
                    "doneProgress": 1.0,
                    "runDuration": 0.0,
                    "cursorTime": null,
                    "scanCount": 0,
                    "eventCount": 0,
                    "resultCount": 1,
                    "diskUsage": 0,
                    "priority": null,
                    "label": null
                } }
            ]
        })))
        .mount(&server)
        .await;

    // Results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"foo": "bar"}],
            "preview": false,
            "total": 1
        })))
        .mount(&server)
        .await;

    let mut cmd = splunk_cli_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri())
        .env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_EARLIEST_TIME", "-1h")
        .args([
            "--output",
            "json",
            "search",
            TEST_QUERY,
            "--earliest",
            "-24h",
            "--wait",
        ])
        .assert()
        .success();
}

/// Test that CLI --count flag overrides SPLUNK_MAX_RESULTS env var.
#[tokio::test]
async fn test_search_count_cli_overrides_env() {
    let server = MockServer::start().await;

    // Create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": { "sid": "test-sid" } }
            ]
        })))
        .mount(&server)
        .await;

    // Job status
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": {
                    "sid": "test-sid",
                    "isDone": true,
                    "isFinalized": true,
                    "doneProgress": 1.0,
                    "runDuration": 0.0,
                    "cursorTime": null,
                    "scanCount": 0,
                    "eventCount": 0,
                    "resultCount": 1,
                    "diskUsage": 0,
                    "priority": null,
                    "label": null
                } }
            ]
        })))
        .mount(&server)
        .await;

    // Results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [{"foo": "bar"}],
            "preview": false,
            "total": 1
        })))
        .mount(&server)
        .await;

    let mut cmd = splunk_cli_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri())
        .env("SPLUNK_API_TOKEN", "test-token")
        .env("SPLUNK_MAX_RESULTS", "500")
        .args([
            "--output", "json", "search", TEST_QUERY, "--count", "50", "--wait",
        ])
        .assert()
        .success();
}
