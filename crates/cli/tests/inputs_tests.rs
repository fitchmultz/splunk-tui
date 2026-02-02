//! Integration tests for `splunk-cli inputs` command.
//!
//! Tests cover:
//! - List subcommand with `--detailed`, `--input-type`, `--count`, `--offset` flags
//! - Output format variations (json, csv, xml)
//! - `--output-file` flag
//! - Error handling

mod common;

use common::{connection_error_predicate, splunk_cmd};
use predicates::prelude::*;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test that `splunk-cli inputs --help` shows correct subcommands.
#[test]
fn test_inputs_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["inputs", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list"));
}

/// Test that `splunk-cli inputs list --help` shows correct flags.
#[test]
fn test_inputs_list_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["inputs", "list", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--detailed")
                .and(predicate::str::contains("--input-type"))
                .and(predicate::str::contains("--count"))
                .and(predicate::str::contains("--offset")),
        );
}

/// Test that `splunk-cli inputs list` attempts to connect to the server.
#[test]
fn test_inputs_execution() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["inputs", "list"]).assert();

    // Should fail with connection error (not a "command not found" error)
    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli inputs list --detailed` works.
#[test]
fn test_inputs_with_detailed() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["inputs", "list", "--detailed"]).assert();

    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli inputs list --input-type <TYPE>` works.
#[test]
fn test_inputs_with_input_type() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd
        .args(["inputs", "list", "--input-type", "tcp/raw"])
        .assert();

    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli inputs list --count <N>` works.
#[test]
fn test_inputs_with_count() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["inputs", "list", "--count", "50"]).assert();

    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli inputs list --offset <N>` works.
#[test]
fn test_inputs_with_offset() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["inputs", "list", "--offset", "10"]).assert();

    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli inputs list --count and --offset` work together.
#[test]
fn test_inputs_with_count_and_offset() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd
        .args(["inputs", "list", "--count", "50", "--offset", "10"])
        .assert();

    result.failure().stderr(connection_error_predicate());
}

/// Test full inputs list with mock server using fixture data (monitor type).
#[tokio::test]
async fn test_inputs_list_mock_server() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/inputs/list_inputs_monitor.json");

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/monitor"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "100"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    let result = cmd
        .args(["inputs", "list", "--input-type", "monitor"])
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("/var/log"));
}

/// Test inputs list with specific type using mock server (tcp/raw type).
#[tokio::test]
async fn test_inputs_list_with_type_mock_server() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/inputs/list_inputs_tcp.json");

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/tcp/raw"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "100"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    let result = cmd
        .args(["inputs", "list", "--input-type", "tcp/raw"])
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("9997"))
        .stdout(predicate::str::contains("9998"));
}

/// Test that `splunk-cli inputs list --output json` produces valid JSON with mock server.
#[tokio::test]
async fn test_inputs_output_json() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/inputs/list_inputs_monitor.json");

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/monitor"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "100"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args([
        "inputs",
        "list",
        "--input-type",
        "monitor",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("/var/log"));
}

/// Test that `splunk-cli inputs list --output csv` produces CSV with mock server.
#[tokio::test]
async fn test_inputs_output_csv() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/inputs/list_inputs_monitor.json");

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/monitor"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "100"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args([
        "inputs",
        "list",
        "--input-type",
        "monitor",
        "--output",
        "csv",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("/var/log"));
}

/// Test that `splunk-cli inputs list --output xml` produces XML with mock server.
#[tokio::test]
async fn test_inputs_output_xml() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/inputs/list_inputs_monitor.json");

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/monitor"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "100"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args([
        "inputs",
        "list",
        "--input-type",
        "monitor",
        "--output",
        "xml",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("var"));
}

/// Test that `splunk-cli inputs list --output-file` writes to file.
#[tokio::test]
async fn test_inputs_output_file() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/inputs/list_inputs_monitor.json");

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/monitor"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "100"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let output_file = temp_dir.path().join("inputs.json");

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args([
        "inputs",
        "list",
        "--input-type",
        "monitor",
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
    assert!(content.contains("/var/log"));
}

/// Test inputs with detailed output using mock server.
#[tokio::test]
async fn test_inputs_detailed_mock_server() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/inputs/list_inputs_tcp.json");

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/tcp/raw"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "100"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    let result = cmd
        .args(["inputs", "list", "--input-type", "tcp/raw", "--detailed"])
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("9997"))
        .stdout(predicate::str::contains("tcp"));
}

/// Test that pagination parameters are passed correctly.
#[tokio::test]
async fn test_inputs_pagination_params() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/monitor"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "25"))
        .and(query_param("offset", "50"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": []
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args([
        "inputs",
        "list",
        "--input-type",
        "monitor",
        "--count",
        "25",
        "--offset",
        "50",
    ])
    .assert()
    .success();
}

/// Test error handling when server returns 401 Unauthorized.
#[tokio::test]
async fn test_inputs_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/monitor"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["inputs", "list", "--input-type", "monitor"])
        .assert()
        .failure();
}

/// Test error handling when server returns 500 Internal Server Error.
#[tokio::test]
async fn test_inputs_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/inputs/monitor"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["inputs", "list", "--input-type", "monitor"])
        .assert()
        .failure();
}
