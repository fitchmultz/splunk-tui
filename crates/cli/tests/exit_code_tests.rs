//! Integration tests for structured exit codes.
//!
//! These tests verify that splunk-cli returns the correct exit codes
//! for different error scenarios, enabling reliable shell scripting.

mod common;

use common::splunk_cmd;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test that successful commands return exit code 0.
#[tokio::test]
async fn test_success_returns_exit_code_0() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [{
                "content": {
                    "serverName": "test-server",
                    "version": "9.0.0",
                    "build": "abcdef"
                }
            }]
        })))
        .mount(&server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.arg("health").assert().code(0);
}

/// Test that authentication failures return exit code 2.
#[tokio::test]
async fn test_auth_failure_returns_exit_code_2() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{ "type": "ERROR", "text": "Unauthorized" }]
        })))
        .mount(&server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.env("SPLUNK_API_TOKEN", "invalid-token");
    cmd.arg("health").assert().code(2);
}

/// Test that connection refused returns exit code 3.
#[test]
fn test_connection_refused_returns_exit_code_3() {
    let mut cmd = splunk_cmd();
    // Use a port that's unlikely to be open
    cmd.env("SPLUNK_BASE_URL", "https://localhost:1");
    cmd.arg("health").assert().code(3);
}

/// Test that resource not found returns exit code 4.
#[tokio::test]
async fn test_not_found_returns_exit_code_4() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/nonexistent-sid"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "messages": [{ "type": "ERROR", "text": "Job not found" }]
        })))
        .mount(&server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.env("SPLUNK_API_TOKEN", "test-token");
    cmd.args(["jobs", "--inspect", "nonexistent-sid"])
        .assert()
        .code(4);
}

/// Test that permission denied (403) returns exit code 6.
#[tokio::test]
async fn test_permission_denied_returns_exit_code_6() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/authentication/users"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "messages": [{ "type": "ERROR", "text": "Forbidden" }]
        })))
        .mount(&server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.env("SPLUNK_API_TOKEN", "test-token");
    cmd.args(["users", "list"]).assert().code(6);
}

/// Test that rate limiting (429) returns exit code 7.
#[tokio::test]
async fn test_rate_limited_returns_exit_code_7() {
    let server = MockServer::start().await;

    // Use a very short retry-after to avoid test timeout
    // The client will retry but with minimal delay
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(429).insert_header("retry-after", "0"))
        .mount(&server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.env("SPLUNK_API_TOKEN", "test-token");
    cmd.arg("health").assert().code(7);
}

/// Test that service unavailable (503) returns exit code 8.
#[tokio::test]
async fn test_service_unavailable_returns_exit_code_8() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(503))
        .mount(&server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.env("SPLUNK_API_TOKEN", "test-token");
    cmd.arg("health").assert().code(8);
}

/// Test that bad gateway (502) returns exit code 8 (service unavailable category).
#[tokio::test]
async fn test_bad_gateway_returns_exit_code_8() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(502))
        .mount(&server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.env("SPLUNK_API_TOKEN", "test-token");
    cmd.arg("health").assert().code(8);
}

/// Test that gateway timeout (504) returns exit code 8 (service unavailable category).
#[tokio::test]
async fn test_gateway_timeout_returns_exit_code_8() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(504))
        .mount(&server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.env("SPLUNK_API_TOKEN", "test-token");
    cmd.arg("health").assert().code(8);
}

/// Test that general errors return exit code 1.
#[tokio::test]
async fn test_general_error_returns_exit_code_1() {
    let server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "messages": [{ "type": "ERROR", "text": "Internal Server Error" }]
        })))
        .mount(&server)
        .await;

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.env("SPLUNK_API_TOKEN", "test-token");
    cmd.arg("health").assert().code(1);
}
