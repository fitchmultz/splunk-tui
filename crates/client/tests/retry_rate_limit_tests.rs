//! Rate limit (429) retry behavior tests.
//!
//! This module tests the client's retry logic for HTTP 429 Too Many Requests
//! responses, with and without Retry-After headers.
//!
//! # Invariants
//! - 429 responses trigger retry with exponential backoff
//! - Retry-After header is respected when present (both delta-seconds and HTTP-date)
//!
//! # What this does NOT handle
//! - Retry-After header parsing details (see retry_header_tests.rs)
//! - Other rate limiting scenarios (see retry_header_tests.rs)

mod common;

use common::*;
use splunk_client::ClientError;
use std::time::Duration;
use wiremock::matchers::{method, path};

#[tokio::test(start_paused = true)]
async fn test_retry_on_429_success() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Use wiremock's sequence feature to return 429 twice, then 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Rate limited"}]
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
            )
            .await
        }
    });

    assert_pending(&result_handle, "429 retry should wait for backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    assert_pending(&result_handle, "second 429 retry should wait for backoff").await;
    advance_and_yield(Duration::from_secs(2)).await;
    let result = result_handle.await.expect("create job task");

    // Should succeed after retries
    assert!(result.is_ok());
    let sid = result.unwrap();
    assert!(sid.contains("scheduler__admin__search"));
}

#[tokio::test(start_paused = true)]
async fn test_retry_on_429_exhaustion() {
    let mock_server = MockServer::start().await;

    // Always return 429
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Rate limited"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let server_uri = mock_server.uri();
    let result_handle = tokio::spawn({
        let client = client.clone();
        let server_uri = server_uri.clone();
        async move {
            endpoints::get_job_status(&client, &server_uri, "test-token", "test-sid", 2, None).await
        }
    });

    assert_pending(&result_handle, "429 exhaustion should wait for backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    assert_pending(
        &result_handle,
        "429 exhaustion should wait for second backoff",
    )
    .await;
    advance_and_yield(Duration::from_secs(2)).await;
    let result = result_handle.await.expect("get job status task");

    // Should fail after exhausting retries
    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::MaxRetriesExceeded(3, _))); // 2 retries + 1 initial attempt = 3 total
}
