//! Retry-After header handling tests.
//!
//! This module tests the client's handling of the Retry-After HTTP header,
//! which can be specified as either delta-seconds (integer) or HTTP-date (RFC 7231).
//!
//! # Invariants
//! - Retry-After header is respected when present
//! - Delta-seconds format (integer) is supported
//! - HTTP-date format (RFC 7231) is supported
//! - Invalid headers fall back to exponential backoff
//! - Past dates fall back to exponential backoff
//! - Maximum of backoff and Retry-After is used when both apply
//!
//! # What this does NOT handle
//! - Rate limiting behavior (see retry_rate_limit_tests.rs)
//! - Other retry scenarios (see retry_5xx_tests.rs, retry_auth_tests.rs)

mod common;

use common::*;
use std::time::Duration;
use wiremock::matchers::{method, path};

#[tokio::test(start_paused = true)]
async fn test_retry_respects_retry_after_header() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // First response returns 429 with Retry-After: 3
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "3")
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response returns 200
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
                3,
                None,
                None,
            )
            .await
        }
    });

    assert_pending(&result_handle, "retry-after should delay request").await;
    advance_and_yield(Duration::from_secs(3)).await;
    let result = result_handle.await.expect("create job task");

    assert!(result.is_ok());
}

#[tokio::test(start_paused = true)]
async fn test_retry_with_max_of_backoff_and_retry_after() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // First two responses return 429 with Retry-After: 1 (less than exponential backoff of 2 on second retry)
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "1")
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    // Third response returns 200
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
                3,
                None,
                None,
            )
            .await
        }
    });

    assert_pending(&result_handle, "first retry should wait for backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    assert_pending(&result_handle, "second retry should wait for backoff").await;
    advance_and_yield(Duration::from_secs(2)).await;
    let result = result_handle.await.expect("create job task");

    assert!(result.is_ok());
}

#[tokio::test(start_paused = true)]
async fn test_retry_falls_back_to_exponential_backoff() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // First response returns 429 WITHOUT Retry-After header
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Rate limited"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response returns 200
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
                3,
                None,
                None,
            )
            .await
        }
    });

    assert_pending(&result_handle, "retry should wait for backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    let result = result_handle.await.expect("create job task");

    assert!(result.is_ok());
}

#[tokio::test(start_paused = true)]
async fn test_retry_with_invalid_retry_after_header() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // First response returns 429 with invalid Retry-After header
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "invalid-date")
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response returns 200
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
                3,
                None,
                None,
            )
            .await
        }
    });

    assert_pending(&result_handle, "invalid retry-after should use backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    let result = result_handle.await.expect("create job task");

    // Should still succeed, falling back to exponential backoff
    assert!(result.is_ok());
}

#[tokio::test(start_paused = true)]
async fn test_retry_respects_retry_after_http_date() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Calculate a future HTTP-date (10 seconds from now)
    let retry_after = Duration::from_secs(10);
    let future_time =
        time::OffsetDateTime::now_utc() + time::Duration::seconds(retry_after.as_secs() as i64);
    let http_date = future_time
        .format(&time::format_description::well_known::Rfc2822)
        .unwrap();

    // First response returns 429 with Retry-After as HTTP-date
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", http_date.as_str())
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response returns 200
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
                3,
                None,
                None,
            )
            .await
        }
    });

    assert_pending(&result_handle, "http-date retry-after should delay request").await;
    advance_and_yield(retry_after).await;
    let result = result_handle.await.expect("create job task");
    assert!(result.is_ok());
}

#[tokio::test(start_paused = true)]
async fn test_retry_with_past_http_date() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // Use a past HTTP-date (RFC 7231 example date from 1994)
    let past_http_date = "Sun, 06 Nov 1994 08:49:37 GMT";

    // First response returns 429 with past Retry-After HTTP-date
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", past_http_date)
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response returns 200
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
                3,
                None,
                None,
            )
            .await
        }
    });

    assert_pending(&result_handle, "past http-date should use backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    let result = result_handle.await.expect("create job task");

    assert!(result.is_ok());
}

#[tokio::test(start_paused = true)]
async fn test_retry_with_invalid_http_date() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    // First response returns 429 with invalid HTTP-date format
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "not-a-valid-date")
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    // Second response returns 200
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
                3,
                None,
                None,
            )
            .await
        }
    });

    assert_pending(&result_handle, "invalid http-date should use backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    let result = result_handle.await.expect("create job task");

    assert!(result.is_ok());
}
