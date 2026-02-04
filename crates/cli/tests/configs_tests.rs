//! Integration tests for `splunk-cli configs` command.
//!
//! Tests cover:
//! - List subcommand with `--config-file`, `--count`, `--offset` flags
//! - View subcommand for specific stanzas
//! - Output format variations (json, csv, xml)
//! - `--output-file` flag
//! - Error handling

mod common;

use common::{connection_error_predicate, splunk_cmd};
use predicates::prelude::*;
use wiremock::matchers::{header, method, path, path_regex, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test that `splunk-cli configs --help` shows correct subcommands.
#[test]
fn test_configs_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["configs", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("list").and(predicate::str::contains("view")));
}

/// Test that `splunk-cli configs list --help` shows correct flags.
#[test]
fn test_configs_list_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["configs", "list", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--config-file")
                .and(predicate::str::contains("--count"))
                .and(predicate::str::contains("--offset")),
        );
}

/// Test that `splunk-cli configs list` returns static config file list without server.
#[test]
fn test_configs_list_execution() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["configs", "list"]).assert();

    // list_config_files() returns a static list without making HTTP calls
    result
        .success()
        .stdout(predicate::str::contains("props"))
        .stdout(predicate::str::contains("transforms"));
}

/// Test that `splunk-cli configs list --config-file <FILE>` works.
#[test]
fn test_configs_list_with_config_file() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd
        .args(["configs", "list", "--config-file", "props"])
        .assert();

    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli configs list --count <N>` works with static list.
#[test]
fn test_configs_list_with_count() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["configs", "list", "--count", "10"]).assert();

    // list_config_files() returns a static list without making HTTP calls
    result.success().stdout(predicate::str::contains("props"));
}

/// Test that `splunk-cli configs list --offset <N>` works with static list.
#[test]
fn test_configs_list_with_offset() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd.args(["configs", "list", "--offset", "5"]).assert();

    // list_config_files() returns a static list without making HTTP calls
    result.success().stdout(predicate::str::contains("props"));
}

/// Test that `splunk-cli configs list --count and --offset` work together with static list.
#[test]
fn test_configs_list_with_count_and_offset() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd
        .args(["configs", "list", "--count", "10", "--offset", "5"])
        .assert();

    // list_config_files() returns a static list without making HTTP calls
    result.success().stdout(predicate::str::contains("props"));
}

/// Test that `splunk-cli configs view` attempts to connect to the server.
#[test]
fn test_configs_view_execution() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:9999");

    let result = cmd
        .args(["configs", "view", "props", "source::..."])
        .assert();

    result.failure().stderr(connection_error_predicate());
}

/// Test full configs list stanzas with mock server using fixture data.
#[tokio::test]
async fn test_configs_list_stanzas_mock_server() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/configs/list_config_stanzas.json");

    Mock::given(method("GET"))
        .and(path("/services/configs/conf-props"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "30"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    let result = cmd
        .args(["configs", "list", "--config-file", "props"])
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("source::..."))
        .stdout(predicate::str::contains("host::myhost"));
}

/// Test that `splunk-cli configs list --output json` produces valid JSON with mock server.
#[tokio::test]
async fn test_configs_list_output_json() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/configs/list_config_stanzas.json");

    Mock::given(method("GET"))
        .and(path("/services/configs/conf-props"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "30"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args([
        "configs",
        "list",
        "--config-file",
        "props",
        "--output",
        "json",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("source::..."));
}

/// Test that `splunk-cli configs list --output csv` produces CSV with mock server.
#[tokio::test]
async fn test_configs_list_output_csv() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/configs/list_config_stanzas.json");

    Mock::given(method("GET"))
        .and(path("/services/configs/conf-props"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "30"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args([
        "configs",
        "list",
        "--config-file",
        "props",
        "--output",
        "csv",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("source::..."));
}

/// Test that `splunk-cli configs list --output xml` produces XML with mock server.
#[tokio::test]
async fn test_configs_list_output_xml() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/configs/list_config_stanzas.json");

    Mock::given(method("GET"))
        .and(path("/services/configs/conf-props"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "30"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args([
        "configs",
        "list",
        "--config-file",
        "props",
        "--output",
        "xml",
    ])
    .assert()
    .success()
    .stdout(predicate::str::contains("source"));
}

/// Test that `splunk-cli configs list --output-file` writes to file.
#[tokio::test]
async fn test_configs_list_output_file() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/configs/list_config_stanzas.json");

    Mock::given(method("GET"))
        .and(path("/services/configs/conf-props"))
        .and(query_param("output_mode", "json"))
        .and(query_param("count", "30"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let temp_dir = tempfile::tempdir().unwrap();
    let output_file = temp_dir.path().join("configs.json");

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args([
        "configs",
        "list",
        "--config-file",
        "props",
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
    assert!(content.contains("source::..."));
}

/// Test configs view with mock server using fixture data.
#[tokio::test]
async fn test_configs_view_mock_server() {
    let mock_server = MockServer::start().await;

    let fixture_data = include_str!("../../client/fixtures/configs/get_config_stanza.json");

    // Use path_regex to handle URL-encoded stanza name (source::... becomes source%3A%3A...)
    Mock::given(method("GET"))
        .and(path_regex("/services/configs/conf-props/source.*"))
        .and(query_param("output_mode", "json"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_string(fixture_data))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    let result = cmd
        .args(["configs", "view", "props", "source::..."])
        .assert();

    result
        .success()
        .stdout(predicate::str::contains("source::..."))
        .stdout(predicate::str::contains("access_combined"));
}

/// Test that pagination parameters are passed correctly for config stanzas.
#[tokio::test]
async fn test_configs_list_pagination_params() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/configs/conf-props"))
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

    cmd.args([
        "configs",
        "list",
        "--config-file",
        "props",
        "--count",
        "5",
        "--offset",
        "10",
    ])
    .assert()
    .success();
}

/// Test error handling when server returns 401 Unauthorized.
#[tokio::test]
async fn test_configs_unauthorized() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/configs/conf-props"))
        .respond_with(ResponseTemplate::new(401))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["configs", "list", "--config-file", "props"])
        .assert()
        .failure();
}

/// Test error handling when server returns 500 Internal Server Error.
#[tokio::test]
async fn test_configs_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/configs/conf-props"))
        .respond_with(ResponseTemplate::new(500))
        .mount(&mock_server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", mock_server.uri());

    cmd.args(["configs", "list", "--config-file", "props"])
        .assert()
        .failure();
}
