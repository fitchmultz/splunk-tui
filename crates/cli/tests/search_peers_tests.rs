//! Integration tests for `splunk-cli search-peers` command.
//!
//! Tests cover:
//! - Default behavior (list search peers)
//! - `--detailed` flag
//! - `--count` and `--offset` pagination flags
//! - Output format variations (json, csv, xml)
//! - `--output-file` flag
//! - Error handling

mod common;

use common::splunk_cmd;
use predicates::prelude::*;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test that `splunk-cli search-peers --help` shows correct flags.
#[test]
fn test_search_peers_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["search-peers", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--detailed")
                .and(predicate::str::contains("--count"))
                .and(predicate::str::contains("--offset"))
                .and(predicate::str::contains("--output"))
                .and(predicate::str::contains("--output-file")),
        );
}

/// Test that `splunk-cli search-peers` attempts to connect to the server.
#[test]
fn test_search_peers_execution() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.arg("search-peers").assert();

    // Should fail with connection error (not a "command not found" error)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli search-peers --detailed` works.
#[test]
fn test_search_peers_with_detailed() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["search-peers", "--detailed"]).assert();

    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli search-peers --count <N>` works.
#[test]
fn test_search_peers_with_count() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["search-peers", "--count", "10"]).assert();

    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli search-peers --offset <N>` works.
#[test]
fn test_search_peers_with_offset() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["search-peers", "--offset", "5"]).assert();

    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli search-peers --count and --offset` work together.
#[test]
fn test_search_peers_with_count_and_offset() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd
        .args(["search-peers", "--count", "10", "--offset", "5"])
        .assert();

    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli search-peers --output json` produces valid JSON with mock server.
#[tokio::test]
async fn test_search_peers_output_json() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/search_peers/list_search_peers.json");

    Mock::given(method("GET"))
        .and(path("/services/search/distributed/peers"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["search-peers", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("peer1"))
        .stdout(predicate::str::contains("peer2"));
}

/// Test that `splunk-cli search-peers --output csv` produces CSV with mock server.
#[tokio::test]
async fn test_search_peers_output_csv() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/search_peers/list_search_peers.json");

    Mock::given(method("GET"))
        .and(path("/services/search/distributed/peers"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["search-peers", "--output", "csv"])
        .assert()
        .success()
        .stdout(predicate::str::contains("peer1"))
        .stdout(predicate::str::contains("peer2"));
}

/// Test that `splunk-cli search-peers --output xml` produces XML with mock server.
#[tokio::test]
async fn test_search_peers_output_xml() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/search_peers/list_search_peers.json");

    Mock::given(method("GET"))
        .and(path("/services/search/distributed/peers"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["search-peers", "--output", "xml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("peer1"))
        .stdout(predicate::str::contains("peer2"));
}

/// Test that `splunk-cli search-peers --output-file` writes to file.
#[tokio::test]
async fn test_search_peers_output_file() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/search_peers/list_search_peers.json");

    Mock::given(method("GET"))
        .and(path("/services/search/distributed/peers"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let output_file = temp_dir.path().join("search_peers.json");

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args([
        "search-peers",
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
    assert!(content.contains("peer1"));
    assert!(content.contains("peer2"));
}

/// Test full search peers list with mock server using fixture data.
#[tokio::test]
async fn test_search_peers_with_mock_server() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/search_peers/list_search_peers.json");

    Mock::given(method("GET"))
        .and(path("/services/search/distributed/peers"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    let result = cmd.arg("search-peers").assert();

    result
        .success()
        .stdout(predicate::str::contains("peer1"))
        .stdout(predicate::str::contains("peer2"))
        .stdout(predicate::str::contains("192.168.1.10"))
        .stdout(predicate::str::contains("192.168.1.11"))
        .stdout(predicate::str::contains("Up"))
        .stdout(predicate::str::contains("Down"));
}

/// Test search peers with detailed output using mock server.
#[tokio::test]
async fn test_search_peers_detailed_with_mock_server() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/search_peers/list_search_peers.json");

    Mock::given(method("GET"))
        .and(path("/services/search/distributed/peers"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    let result = cmd.args(["search-peers", "--detailed"]).assert();

    result
        .success()
        .stdout(predicate::str::contains("peer1"))
        .stdout(predicate::str::contains("peer2"))
        .stdout(predicate::str::contains("abc-123-def-456"))
        .stdout(predicate::str::contains("xyz-789-abc-012"));
}

/// Test that pagination parameters are passed correctly.
#[tokio::test]
async fn test_search_peers_pagination_params() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/search/distributed/peers"))
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

    cmd.args(["search-peers", "--count", "5", "--offset", "10"])
        .assert()
        .success();
}

/// Test error handling when server returns 401 Unauthorized.
#[tokio::test]
async fn test_search_peers_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/search/distributed/peers"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.arg("search-peers").assert().failure();
}

/// Test error handling when server returns 500 Internal Server Error.
#[tokio::test]
async fn test_search_peers_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/search/distributed/peers"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.arg("search-peers").assert().failure();
}
