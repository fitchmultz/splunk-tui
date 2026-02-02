//! Integration tests for `splunk-cli lookups` command.
//!
//! Tests cover:
//! - Default behavior (list lookup tables)
//! - `--count` and `--offset` pagination flags
//! - Output format variations (json, csv, xml)
//! - `--output-file` flag
//! - Empty results handling
//! - Error handling

mod common;

use common::{connection_error_predicate, splunk_cmd};
use predicates::prelude::*;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test that `splunk-cli lookups --help` shows correct flags.
#[test]
fn test_lookups_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["lookups", "--help"]).assert().success().stdout(
        predicate::str::contains("--count")
            .and(predicate::str::contains("--offset"))
            .and(predicate::str::contains("--output"))
            .and(predicate::str::contains("--output-file")),
    );
}

/// Test that `splunk-cli lookups` attempts to connect to the server.
#[test]
fn test_lookups_execution() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.arg("lookups").assert();

    // Should fail with connection error (not a "command not found" error)
    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli lookups --count <N>` works.
#[test]
fn test_lookups_with_count() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["lookups", "--count", "10"]).assert();

    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli lookups --offset <N>` works.
#[test]
fn test_lookups_with_offset() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["lookups", "--offset", "5"]).assert();

    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli lookups --count and --offset` work together.
#[test]
fn test_lookups_with_count_and_offset() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd
        .args(["lookups", "--count", "10", "--offset", "5"])
        .assert();

    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli lookups --output json` produces valid JSON with mock server.
#[tokio::test]
async fn test_lookups_output_json() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
            "entry": [
                {
                    "name": "my_lookup",
                    "content": {
                        "name": "my_lookup",
                        "filename": "my_lookup.csv",
                        "owner": "admin",
                        "app": "search",
                        "sharing": "app",
                        "size": "1024"
                    }
                }
            ]
        }"#,
        ))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["lookups", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my_lookup"));
}

/// Test that `splunk-cli lookups --output csv` produces CSV with mock server.
#[tokio::test]
async fn test_lookups_output_csv() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
            "entry": [
                {
                    "name": "my_lookup",
                    "content": {
                        "name": "my_lookup",
                        "filename": "my_lookup.csv",
                        "owner": "admin",
                        "app": "search",
                        "sharing": "app",
                        "size": "1024"
                    }
                }
            ]
        }"#,
        ))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["lookups", "--output", "csv"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my_lookup"));
}

/// Test that `splunk-cli lookups --output xml` produces XML with mock server.
#[tokio::test]
async fn test_lookups_output_xml() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
            "entry": [
                {
                    "name": "my_lookup",
                    "content": {
                        "name": "my_lookup",
                        "filename": "my_lookup.csv",
                        "owner": "admin",
                        "app": "search",
                        "sharing": "app",
                        "size": "1024"
                    }
                }
            ]
        }"#,
        ))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["lookups", "--output", "xml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("my_lookup"));
}

/// Test that `splunk-cli lookups --output-file` writes to file.
#[tokio::test]
async fn test_lookups_output_file() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(
            r#"{
            "entry": [
                {
                    "name": "my_lookup",
                    "content": {
                        "name": "my_lookup",
                        "filename": "my_lookup.csv",
                        "owner": "admin",
                        "app": "search",
                        "sharing": "app",
                        "size": "1024"
                    }
                }
            ]
        }"#,
        ))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let output_file = temp_dir.path().join("lookups.json");

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args([
        "lookups",
        "--output",
        "json",
        "--output-file",
        output_file.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("Results written to"));

    // Verify file was created and contains expected content
    assert!(output_file.exists());
    let content = std::fs::read_to_string(&output_file).unwrap();
    assert!(content.contains("my_lookup"));
}

/// Test full lookups list with mock server using fixture data.
#[tokio::test]
async fn test_lookups_with_mock_server() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/lookups/list_lookup_tables.json");

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    let result = cmd.arg("lookups").assert();

    result
        .success()
        .stdout(predicate::str::contains("my_lookup"))
        .stdout(predicate::str::contains("countries"));
}

/// Test empty lookup tables response handling.
#[tokio::test]
async fn test_lookups_empty_response() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/lookups/list_lookup_tables_empty.json");

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    // Should succeed even with empty results
    cmd.arg("lookups").assert().success();
}

/// Test that pagination parameters are passed correctly.
#[tokio::test]
async fn test_lookups_pagination_params() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "5"))
        .and(query_param("offset", "10"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": []
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["lookups", "--count", "5", "--offset", "10"])
        .assert()
        .success();
}

/// Test error handling when server returns 401 Unauthorized.
#[tokio::test]
async fn test_lookups_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.arg("lookups").assert().failure();
}

/// Test error handling when server returns 500 Internal Server Error.
#[tokio::test]
async fn test_lookups_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/lookup-table-files"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.arg("lookups").assert().failure();
}

/// Test that lookups does NOT have --detailed flag (unlike forwarders).
#[test]
fn test_lookups_no_detailed_flag() {
    let mut cmd = splunk_cmd();

    cmd.args(["lookups", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--detailed").not());
}
