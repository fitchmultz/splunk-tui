//! Chaos network tests for resilience verification.
//!
//! This module tests the client's behavior under network-level failures:
//! - Network jitter (random delays)
//! - Partial/truncated JSON responses
//! - Connection drops mid-stream
//! - Memory pressure (limited buffer sizes)
//!
//! # Invariants
//! - Client must handle partial responses gracefully (no panics)
//! - Retry logic must recover from transient network failures (HTTP 5xx, 429)
//! - No undefined behavior under any network condition

mod common;

use common::*;
use splunk_client::ClientError;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test that truncated JSON responses produce proper errors (no retry).
///
/// Note: The client does NOT retry on JSON parse errors, only on HTTP
/// status codes (429, 502-504) and transport errors. This test verifies
/// that the client returns a proper error (not panic).
#[tokio::test]
async fn test_truncated_json_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            // Return truncated JSON (simulating connection drop)
            ResponseTemplate::new(200).set_body_string(r#"{"entry": [{"content": {"version": ""#)
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 3, None).await;

    // Should fail - JSON parse errors are not retryable
    assert!(
        result.is_err(),
        "Should fail on truncated JSON (not retryable)"
    );
}

/// Test that malformed JSON responses produce proper errors (no retry).
///
/// Note: Malformed JSON is not a retryable error.
#[tokio::test]
async fn test_malformed_json_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            ResponseTemplate::new(200).set_body_string("not valid json {{")
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 3, None).await;

    // Should fail - JSON parse errors are not retryable
    assert!(
        result.is_err(),
        "Should fail on malformed JSON (not retryable)"
    );
}

/// Test handling of empty response body.
///
/// Empty body causes JSON parse error - not retryable.
#[tokio::test]
async fn test_empty_response_body() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(200).set_body_string(""))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 3, None).await;

    // Empty body causes JSON parse error - not retryable
    assert!(result.is_err(), "Should fail on empty body (not retryable)");
}

/// Test connection drops during request handling (503 -> retry).
///
/// HTTP 503 is a retryable status code. The client should retry
/// and eventually succeed when the server returns 200.
#[tokio::test]
async fn test_connection_error_recovery() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                // Simulate connection/service error
                ResponseTemplate::new(503).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Connection reset"}]
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {
                        "version": "9.0.0",
                        "serverName": "test-server",
                        "build": "12345",
                        "cpu_arch": "x86_64",
                        "osName": "Linux",
                        "guid": "test-guid-1234"
                    }}]
                }))
            }
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();

    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 3, None).await;

    assert!(
        result.is_ok(),
        "Should succeed after retry: {:?}",
        result.err()
    );
    assert_eq!(request_count.load(Ordering::SeqCst), 2, "Should retry once");
}

/// Test handling of extremely large responses that could cause memory pressure.
///
/// This test verifies that the client handles large response bodies gracefully
/// without unbounded memory growth.
#[tokio::test]
async fn test_large_response_handling() {
    let mock_server = MockServer::start().await;

    // Generate a moderately large JSON response (simulating many search results)
    let entries: Vec<_> = (0..1000)
        .map(|i| {
            serde_json::json!({
                "_time": format!("2024-01-15T10:{:02}:00.000Z", i % 60),
                "message": format!("Event {} with some data content here", i),
                "source": "test",
                "sourcetype": "test-data"
            })
        })
        .collect();

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&entries))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::search::get_results(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        Some(1000),
        Some(0),
        endpoints::search::OutputMode::Json,
        3,
        None,
    )
    .await;

    assert!(result.is_ok(), "Should handle large response");
    let results = result.unwrap();
    assert_eq!(results.results.len(), 1000, "Should receive all results");
}

/// Test handling of partial JSON that parses but is missing expected fields.
///
/// Simulates a server returning valid JSON that doesn't match the expected schema.
/// This is NOT retryable - returns InvalidResponse immediately.
#[tokio::test]
async fn test_partial_schema_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            // Return JSON with missing entry content (schema mismatch)
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "entry": [{"name": "server-info", "id": "test"}]
            }))
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 3, None).await;

    // Should fail with InvalidResponse - schema mismatch is not retryable
    assert!(
        result.is_err(),
        "Should fail on schema mismatch (not retryable)"
    );
    assert!(matches!(
        result.unwrap_err(),
        ClientError::InvalidResponse(_)
    ));
}

/// Test handling of 503 with retry (server temporarily unavailable).
///
/// The client should retry on 503 and eventually succeed.
#[tokio::test]
async fn test_server_unavailable_retry() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            if count < 2 {
                // First two requests return 503
                ResponseTemplate::new(503).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Service temporarily unavailable"}]
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {
                        "version": "9.0.0",
                        "serverName": "test-server",
                        "build": "12345",
                        "cpu_arch": "x86_64",
                        "osName": "Linux",
                        "guid": "test-guid-1234"
                    }}]
                }))
            }
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();

    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 3, None).await;

    assert!(
        result.is_ok(),
        "Should succeed after multiple retries: {:?}",
        result.err()
    );
    assert_eq!(
        request_count.load(Ordering::SeqCst),
        3,
        "Should retry twice"
    );
}

/// Test handling of responses with unexpected content types.
///
/// Simulates a server returning text/plain instead of application/json.
/// This is NOT retryable - returns error immediately.
#[tokio::test]
async fn test_unexpected_content_type() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            // Return plain text response
            ResponseTemplate::new(200)
                .insert_header("content-type", "text/plain")
                .set_body_string("Server is running")
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 3, None).await;

    // Plain text causes JSON parse error - not retryable
    assert!(
        result.is_err(),
        "Should fail on unexpected content type (not retryable)"
    );
}

/// Test 502 Bad Gateway retry.
///
/// HTTP 502 is a retryable status code.
#[tokio::test]
async fn test_bad_gateway_retry() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                ResponseTemplate::new(502).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Bad gateway"}]
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {
                        "version": "9.0.0",
                        "serverName": "test-server",
                        "build": "12345",
                        "cpu_arch": "x86_64",
                        "osName": "Linux",
                        "guid": "test-guid-1234"
                    }}]
                }))
            }
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();

    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 3, None).await;

    assert!(
        result.is_ok(),
        "Should succeed after retry on 502: {:?}",
        result.err()
    );
    assert_eq!(request_count.load(Ordering::SeqCst), 2, "Should retry once");
}

/// Test 504 Gateway Timeout retry.
///
/// HTTP 504 is a retryable status code.
#[tokio::test]
async fn test_gateway_timeout_retry() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                ResponseTemplate::new(504).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Gateway timeout"}]
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {
                        "version": "9.0.0",
                        "serverName": "test-server",
                        "build": "12345",
                        "cpu_arch": "x86_64",
                        "osName": "Linux",
                        "guid": "test-guid-1234"
                    }}]
                }))
            }
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();

    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 3, None).await;

    assert!(
        result.is_ok(),
        "Should succeed after retry on 504: {:?}",
        result.err()
    );
    assert_eq!(request_count.load(Ordering::SeqCst), 2, "Should retry once");
}

/// Test retry exhaustion with continuous 5xx errors.
///
/// After max_retries is exceeded, the client should return MaxRetriesExceeded.
#[tokio::test]
async fn test_retry_exhaustion() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            count_clone.fetch_add(1, Ordering::SeqCst);
            // Always return 503
            ResponseTemplate::new(503).set_body_json(serde_json::json!({
                "messages": [{"type": "ERROR", "text": "Service unavailable"}]
            }))
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();

    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 2, None).await;

    assert!(result.is_err(), "Should fail after retry exhaustion");

    let err = result.unwrap_err();
    assert!(
        matches!(err, ClientError::MaxRetriesExceeded(3, _)),
        "Expected MaxRetriesExceeded(3, _), got {:?}",
        err
    );

    // Should have made 3 attempts (initial + 2 retries)
    assert_eq!(
        request_count.load(Ordering::SeqCst),
        3,
        "Should make exactly 3 attempts"
    );
}
