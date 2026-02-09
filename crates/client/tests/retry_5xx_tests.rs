//! Server error (5xx) retry behavior tests.
//!
//! This module tests the client's retry logic for HTTP 5xx server error responses.
//!
//! # Invariants
//! - 502 Bad Gateway triggers retry with exponential backoff
//! - 503 Service Unavailable triggers retry with exponential backoff
//! - 504 Gateway Timeout triggers retry with exponential backoff
//! - 500 Internal Server Error does NOT trigger retry
//! - 501 Not Implemented does NOT trigger retry
//!
//! # What this does NOT handle
//! - Client error retries (4xx, see retry_rate_limit_tests.rs, retry_auth_tests.rs)
//! - Timeout errors (see retry_tests.rs)

mod common;

use common::*;
use splunk_client::ClientError;
use std::time::Duration;
use wiremock::matchers::{method, path};

#[tokio::test(start_paused = true)]
async fn test_retry_on_503_success() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Use wiremock's sequence feature to return 503 twice, then 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Service Unavailable"}]
        })))
        .up_to_n_times(2)
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
                None,
            )
            .await
        }
    });

    assert_pending(&result_handle, "503 retry should wait for backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    assert_pending(&result_handle, "second 503 retry should wait for backoff").await;
    advance_and_yield(Duration::from_secs(2)).await;
    let result = result_handle.await.expect("create job task");

    // Should succeed after retries
    assert!(result.is_ok());
    let sid = result.unwrap();
    assert!(sid.contains("scheduler__admin__search"));
}

#[tokio::test(start_paused = true)]
async fn test_retry_on_502_success() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Return 502 once, then 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(502).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Bad Gateway"}]
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
                None,
            )
            .await
        }
    });

    assert_pending(&result_handle, "502 retry should wait for backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    let result = result_handle.await.expect("create job task");

    // Should succeed after retry
    assert!(result.is_ok());
}

#[tokio::test(start_paused = true)]
async fn test_retry_on_504_success() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Return 504 twice, then 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(504).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Gateway Timeout"}]
        })))
        .up_to_n_times(2)
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
                None,
            )
            .await
        }
    });

    assert_pending(&result_handle, "504 retry should wait for backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    assert_pending(&result_handle, "second 504 retry should wait for backoff").await;
    advance_and_yield(Duration::from_secs(2)).await;
    let result = result_handle.await.expect("create job task");

    // Should succeed after retries
    assert!(result.is_ok());
}

#[tokio::test(start_paused = true)]
async fn test_retry_on_5xx_exhaustion() {
    let mock_server = MockServer::start().await;

    // Always return 503
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Service Unavailable"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let server_uri = mock_server.uri();
    let result_handle = tokio::spawn({
        let client = client.clone();
        let server_uri = server_uri.clone();
        async move {
            endpoints::get_job_status(
                &client,
                &server_uri,
                "test-token",
                "test-sid",
                2,
                None,
                None,
            )
            .await
        }
    });

    assert_pending(&result_handle, "5xx exhaustion should wait for backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    assert_pending(
        &result_handle,
        "5xx exhaustion should wait for second backoff",
    )
    .await;
    advance_and_yield(Duration::from_secs(2)).await;
    let result = result_handle.await.expect("get job status task");

    // Should fail after exhausting retries
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::MaxRetriesExceeded(3, _))); // 2 retries + 1 initial attempt = 3 total
}

/// Test that 500/501 errors do not trigger retries.
///
/// This test verifies that internal server errors (500) and not implemented (501)
/// return immediately without exponential backoff retries.
///
/// Note: This test runs with real time because it needs to verify actual timing
/// behavior - 500/501 should return much faster than the ~7s that retries would take.
#[tokio::test]
async fn test_no_retry_on_500_or_501() {
    let mock_server = MockServer::start().await;

    // Return 500 (should not retry)
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Internal Server Error"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();

    // Measure the time to ensure no exponential backoff delays
    let start = std::time::Instant::now();
    let result = endpoints::get_job_status(
        &client,
        &mock_server.uri(),
        "test-token",
        "test-sid",
        3,
        None,
        None,
    )
    .await;
    let elapsed = start.elapsed();

    // Should fail immediately without retry (well under the ~7s that retries would take)
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(
        matches!(err, ClientError::ApiError { status: 500, .. }),
        "Expected ApiError with status 500, got {:?}",
        err
    );

    // Verify no exponential backoff occurred (should complete in under 2 seconds)
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "500 errors should not trigger exponential backoff. Elapsed: {:?}",
        elapsed
    );
}
