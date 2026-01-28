//! Integration tests for `--output-file` flag functionality.

mod common;

use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;
use wiremock::matchers::{header, method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

#[test]
fn test_output_file_flag_exists() {
    let mut cmd = common::splunk_cmd();
    cmd.args(["search", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--output-file"));
}

#[tokio::test]
async fn test_output_file_creates_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("results.json");

    let mock_server = MockServer::start().await;

    // Create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": { "sid": "test-sid-123" } }
            ]
        })))
        .mount(&mock_server)
        .await;

    // Job status
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid-123"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": {
                    "sid": "test-sid-123",
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
        .mount(&mock_server)
        .await;

    // Results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid-123/results"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [],
            "preview": false,
            "total": 0
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = common::splunk_cmd_with_base_url(&mock_server.uri());
    cmd.args([
        "search",
        "index=main | head 1",
        "--wait",
        "--output",
        "json",
        "--output-file",
        output_path.to_str().unwrap(),
    ])
    .assert()
    .success()
    .stderr(predicate::str::contains("Results written to"));

    // Verify file was created
    assert!(output_path.exists(), "Output file should be created");
    let content = fs::read_to_string(&output_path).unwrap();
    assert!(!content.is_empty(), "Output file should not be empty");
}

#[tokio::test]
async fn test_output_file_overwrites_existing() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("results.json");

    // Create existing file with different content
    fs::write(&output_path, "old content").unwrap();

    let mock_server = MockServer::start().await;

    // Create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": { "sid": "test-sid-123" } }
            ]
        })))
        .mount(&mock_server)
        .await;

    // Job status
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid-123"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": {
                    "sid": "test-sid-123",
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
        .mount(&mock_server)
        .await;

    // Results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid-123/results"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [],
            "preview": false,
            "total": 0
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = common::splunk_cmd_with_base_url(&mock_server.uri());
    cmd.args([
        "search",
        "index=main | head 1",
        "--wait",
        "--output",
        "json",
        "--output-file",
        output_path.to_str().unwrap(),
    ])
    .assert()
    .success();

    // Verify file was overwritten
    let content = fs::read_to_string(&output_path).unwrap();
    assert_ne!(content, "old content", "File should be overwritten");
}

#[tokio::test]
async fn test_output_file_creates_parent_directories() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir
        .path()
        .join("nested")
        .join("dir")
        .join("results.json");

    let mock_server = MockServer::start().await;

    // Create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": { "sid": "test-sid-123" } }
            ]
        })))
        .mount(&mock_server)
        .await;

    // Job status
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid-123"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": {
                    "sid": "test-sid-123",
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
        .mount(&mock_server)
        .await;

    // Results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid-123/results"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [],
            "preview": false,
            "total": 0
        })))
        .mount(&mock_server)
        .await;

    let mut cmd = common::splunk_cmd_with_base_url(&mock_server.uri());
    cmd.args([
        "search",
        "index=main | head 1",
        "--wait",
        "--output",
        "json",
        "--output-file",
        output_path.to_str().unwrap(),
    ])
    .assert()
    .success();

    // Verify parent directories were created and file exists
    assert!(output_path.exists(), "Output file should be created");
    assert!(
        output_path.parent().unwrap().exists(),
        "Parent directories should be created"
    );
}

#[test]
fn test_tail_mode_rejects_output_file() {
    let mut cmd = common::splunk_cmd_with_base_url("https://localhost:8089");
    cmd.args(["logs", "--tail", "--output-file", "/tmp/test-results.json"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "--output-file cannot be used with --tail mode",
        ));
}

#[tokio::test]
async fn test_output_file_with_different_formats() {
    let temp_dir = TempDir::new().unwrap();
    let formats = ["json", "csv", "xml", "table"];

    for fmt in formats {
        let output_path = temp_dir.path().join(format!("results.{}", fmt));

        let mock_server = MockServer::start().await;

        // Create job
        Mock::given(method("POST"))
            .and(path("/services/search/jobs"))
            .and(header("Authorization", "Bearer test-token"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "entry": [
                    { "content": { "sid": "test-sid-123" } }
                ]
            })))
            .mount(&mock_server)
            .await;

        // Job status
        Mock::given(method("GET"))
            .and(path("/services/search/jobs/test-sid-123"))
            .and(header("Authorization", "Bearer test-token"))
            .and(query_param("output_mode", "json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "entry": [
                    { "content": {
                        "sid": "test-sid-123",
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
            .mount(&mock_server)
            .await;

        // Results
        Mock::given(method("GET"))
            .and(path("/services/search/jobs/test-sid-123/results"))
            .and(header("Authorization", "Bearer test-token"))
            .and(query_param("output_mode", "json"))
            .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "results": [],
                "preview": false,
                "total": 0
            })))
            .mount(&mock_server)
            .await;

        let mut cmd = common::splunk_cmd_with_base_url(&mock_server.uri());
        cmd.args([
            "search",
            "index=main | head 1",
            "--wait",
            "--output",
            fmt,
            "--output-file",
            output_path.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Results written to"));

        assert!(
            output_path.exists(),
            "Output file should be created for format {}",
            fmt
        );
    }
}

#[tokio::test]
async fn test_output_file_with_no_parent_directory() {
    let temp_dir = TempDir::new().unwrap();
    // Use a simple filename without parent directory path
    let output_path = temp_dir.path().join("results.json");

    let mock_server = MockServer::start().await;

    // Create job
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": { "sid": "test-sid-123" } }
            ]
        })))
        .mount(&mock_server)
        .await;

    // Job status
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid-123"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [
                { "content": {
                    "sid": "test-sid-123",
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
        .mount(&mock_server)
        .await;

    // Results
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid-123/results"))
        .and(header("Authorization", "Bearer test-token"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "results": [],
            "preview": false,
            "total": 0
        })))
        .mount(&mock_server)
        .await;

    // Use cmd.current_dir() instead of global set_current_dir to maintain
    // test isolation and avoid affecting parallel tests.
    let mut cmd = common::splunk_cmd_with_base_url(&mock_server.uri());
    cmd.current_dir(temp_dir.path())
        .args([
            "search",
            "index=main | head 1",
            "--wait",
            "--output",
            "json",
            "--output-file",
            "results.json",
        ])
        .assert()
        .success()
        .stderr(predicate::str::contains("Results written to"));

    // Verify file was created
    assert!(output_path.exists(), "Output file should be created");
    let content = fs::read_to_string(&output_path).unwrap();
    assert!(!content.is_empty(), "Output file should not be empty");
}
