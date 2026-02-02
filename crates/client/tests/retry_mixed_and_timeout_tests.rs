//! Mixed retry scenarios and timeout handling tests.
//!
//! This module tests the client's retry logic for mixed error scenarios
//! (different error codes in sequence) and request timeout handling.
//!
//! # Invariants
//! - Mixed error codes (e.g., 503 then 429) are handled correctly with cumulative backoff
//! - Timeout errors trigger retry with exponential backoff
//! - Timeout retries eventually succeed if the server recovers
//!
//! # What this does NOT handle
//! - Basic rate limiting retries (see retry_rate_limit_tests.rs)
//! - Authentication retries (see retry_auth_tests.rs)
//! - Server error retries (see retry_server_error_tests.rs)
//! - Retry-After header handling (see retry_after_header_tests.rs)

mod common;

use common::*;
use std::time::Duration;
use wiremock::matchers::{method, path};

#[tokio::test(start_paused = true)]
async fn test_retry_mixed_503_and_429() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Return 503, then 429, then 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Service Unavailable"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Rate limited"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let server_uri = mock_server.uri();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    let result_handle = tokio::spawn({
        let client = client.clone();
        let server_uri = server_uri.clone();
        async move {
            endpoints::create_job(
                &client,
                &server_uri,
                "test-token",
                "search index=main",
                &options,
                3, // max_retries
                None,
            )
            .await
        }
    });

    assert_pending(&result_handle, "mixed retries should wait for backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    assert_pending(
        &result_handle,
        "mixed retries should wait for second backoff",
    )
    .await;
    advance_and_yield(Duration::from_secs(2)).await;
    let result = result_handle.await.expect("create job task");

    // Should succeed after handling both 503 and 429
    assert!(result.is_ok());
}

/// Test that verifies timeout errors trigger retry behavior.
///
/// This test uses a mock server that delays responses longer than the client
/// timeout, causing reqwest to return a timeout error. The retry logic should
/// attempt the request multiple times before succeeding.
///
/// Note: This test runs with real time because:
/// - wiremock's `set_delay` uses real `std::time::Duration` (not tokio time)
/// - The HTTP client timeout is based on real time
#[tokio::test]
async fn test_retry_on_timeout() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // First two requests will timeout (we simulate this by having the mock
    // server delay longer than the client timeout)
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Timeout"}]
                }))
                .set_delay(std::time::Duration::from_secs(5)),
        )
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    // Third request succeeds immediately
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    // Create client with a short timeout
    let client = Client::builder()
        .timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();

    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    // This should succeed after 2 timeouts (with retries) then success
    // With 100ms timeout + 1s + 2s backoff = ~3s total
    let start = std::time::Instant::now();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3, // max_retries
        None,
    )
    .await;

    let elapsed = start.elapsed();

    assert!(result.is_ok(), "Timeout retries should eventually succeed");

    // Should complete in reasonable time (with exponential backoff: 1s + 2s = 3s + overhead)
    assert!(
        elapsed < std::time::Duration::from_secs(8),
        "Timeout retries should complete with exponential backoff. Elapsed: {:?}",
        elapsed
    );
}
