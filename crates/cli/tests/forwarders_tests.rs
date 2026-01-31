//! Integration tests for `splunk-cli forwarders` command.
//!
//! Tests cover:
//! - Default behavior (list forwarders)
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

/// Test that `splunk-cli forwarders --help` shows correct flags.
#[test]
fn test_forwarders_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["forwarders", "--help"])
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

/// Test that `splunk-cli forwarders` attempts to connect to the server.
#[test]
fn test_forwarders_execution() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.arg("forwarders").assert();

    // Should fail with connection error (not a "command not found" error)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli forwarders --detailed` works.
#[test]
fn test_forwarders_with_detailed() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["forwarders", "--detailed"]).assert();

    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli forwarders --count <N>` works.
#[test]
fn test_forwarders_with_count() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["forwarders", "--count", "10"]).assert();

    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli forwarders --offset <N>` works.
#[test]
fn test_forwarders_with_offset() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["forwarders", "--offset", "5"]).assert();

    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli forwarders --count and --offset` work together.
#[test]
fn test_forwarders_with_count_and_offset() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd
        .args(["forwarders", "--count", "10", "--offset", "5"])
        .assert();

    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli forwarders --output json` produces valid JSON with mock server.
#[tokio::test]
async fn test_forwarders_output_json() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/deployment/server/clients"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                {
                    "name": "forwarder1.example.com",
                    "content": {
                        "hostname": "forwarder1.example.com",
                        "clientName": "uf_forwarder1",
                        "ipAddress": "192.168.1.101",
                        "version": "9.1.2"
                    }
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["forwarders", "--output", "json"])
        .assert()
        .success()
        .stdout(predicate::str::contains("forwarder1.example.com"));
}

/// Test that `splunk-cli forwarders --output csv` produces CSV with mock server.
#[tokio::test]
async fn test_forwarders_output_csv() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/deployment/server/clients"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                {
                    "name": "forwarder1.example.com",
                    "content": {
                        "hostname": "forwarder1.example.com",
                        "clientName": "uf_forwarder1",
                        "ipAddress": "192.168.1.101",
                        "version": "9.1.2"
                    }
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["forwarders", "--output", "csv"])
        .assert()
        .success()
        .stdout(predicate::str::contains("forwarder1.example.com"));
}

/// Test that `splunk-cli forwarders --output xml` produces XML with mock server.
#[tokio::test]
async fn test_forwarders_output_xml() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/deployment/server/clients"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                {
                    "name": "forwarder1.example.com",
                    "content": {
                        "hostname": "forwarder1.example.com",
                        "clientName": "uf_forwarder1",
                        "ipAddress": "192.168.1.101",
                        "version": "9.1.2"
                    }
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["forwarders", "--output", "xml"])
        .assert()
        .success()
        .stdout(predicate::str::contains("forwarder1"));
}

/// Test that `splunk-cli forwarders --output-file` writes to file.
#[tokio::test]
async fn test_forwarders_output_file() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/deployment/server/clients"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                {
                    "name": "forwarder1.example.com",
                    "content": {
                        "hostname": "forwarder1.example.com",
                        "clientName": "uf_forwarder1",
                        "ipAddress": "192.168.1.101",
                        "version": "9.1.2"
                    }
                }
            ]
        })))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let output_file = temp_dir.path().join("forwarders.json");

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args([
        "forwarders",
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
    assert!(content.contains("forwarder1.example.com"));
}

/// Test full forwarders list with mock server using fixture data.
#[tokio::test]
async fn test_forwarders_with_mock_server() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/forwarders/list_forwarders.json");

    Mock::given(method("GET"))
        .and(path("/services/deployment/server/clients"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    let result = cmd.arg("forwarders").assert();

    result
        .success()
        .stdout(predicate::str::contains("forwarder1.example.com"))
        .stdout(predicate::str::contains("forwarder2.example.com"))
        .stdout(predicate::str::contains("windows-forwarder.corp.local"));
}

/// Test forwarders with detailed output using mock server.
#[tokio::test]
async fn test_forwarders_detailed_with_mock_server() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/forwarders/list_forwarders.json");

    Mock::given(method("GET"))
        .and(path("/services/deployment/server/clients"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    let result = cmd.args(["forwarders", "--detailed"]).assert();

    result
        .success()
        .stdout(predicate::str::contains("forwarder1.example.com"))
        .stdout(predicate::str::contains("192.168.1.101"));
}

/// Test that pagination parameters are passed correctly.
#[tokio::test]
async fn test_forwarders_pagination_params() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/deployment/server/clients"))
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

    cmd.args(["forwarders", "--count", "5", "--offset", "10"])
        .assert()
        .success();
}

/// Test error handling when server returns 401 Unauthorized.
#[tokio::test]
async fn test_forwarders_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/deployment/server/clients"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.arg("forwarders").assert().failure();
}

/// Test error handling when server returns 500 Internal Server Error.
#[tokio::test]
async fn test_forwarders_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/deployment/server/clients"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.arg("forwarders").assert().failure();
}
