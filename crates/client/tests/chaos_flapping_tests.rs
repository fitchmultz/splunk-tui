//! Chaos flapping tests for rapid status code changes.
//!
//! This module tests the client's behavior when upstream servers rapidly
//! change their state (flapping), simulating load balancer issues or
//! rolling deployments.
//!
//! # Invariants
//! - Client must remain stable during rapid state changes
//! - Retry logic must eventually converge to success
//! - No infinite retry loops

mod common;

use common::*;
use splunk_client::ClientError;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::{method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test rapid 200/503 alternation (flapping).
///
/// Simulates a server that rapidly alternates between healthy and unhealthy states,
/// which can happen during rolling deployments or load balancer issues.
#[tokio::test]
async fn test_status_code_flapping_200_503() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            // Alternate between 200 and 503
            if count % 2 == 0 {
                ResponseTemplate::new(503).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Service unavailable"}]
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {
                        "version": "9.0.0",
                        "serverName": "test-server",
                        "build": "12345"
                    }}]
                }))
            }
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();

    // Should eventually succeed on an odd-numbered attempt
    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 5, None).await;
    assert!(
        result.is_ok(),
        "Should eventually succeed despite flapping: {:?}",
        result.err()
    );
}

/// Test rapid 429/200 alternation (rate limit flapping).
///
/// Simulates a rate limiter that rapidly switches between limiting and allowing requests.
#[tokio::test]
async fn test_rate_limit_flapping() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            // First two requests: 429, then success
            if count < 2 {
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "1")
                    .set_body_json(serde_json::json!({
                        "messages": [{"type": "ERROR", "text": "Rate limited"}]
                    }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {
                        "version": "9.0.0",
                        "serverName": "test-server",
                        "build": "12345"
                    }}]
                }))
            }
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();

    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 5, None).await;
    assert!(
        result.is_ok(),
        "Should succeed after handling rate limit flapping: {:?}",
        result.err()
    );
}

/// Test cascading failure recovery.
///
/// Simulates a scenario where a server starts failing but gradually recovers.
#[tokio::test]
async fn test_cascading_failure_recovery() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            match count {
                0 => ResponseTemplate::new(503), // First attempt: fail
                1 => ResponseTemplate::new(503), // Second attempt: fail
                2 => ResponseTemplate::new(502), // Third attempt: different error
                3 => ResponseTemplate::new(429) // Fourth attempt: rate limited
                    .insert_header("retry-after", "1"),
                _ => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {
                        "version": "9.0.0",
                        "serverName": "test-server",
                        "build": "12345"
                    }}]
                })),
            }
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();

    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 5, None).await;
    assert!(
        result.is_ok(),
        "Should recover from cascading failures: {:?}",
        result.err()
    );
}

/// Test random status code chaos.
///
/// Uses randomized status codes to test general resilience.
#[tokio::test]
async fn test_random_status_code_chaos() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            // Deterministic "random" pattern for reproducibility
            let statuses = [503, 429, 502, 504, 200];
            let status = statuses[count % statuses.len()];

            if status == 200 {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {
                        "version": "9.0.0",
                        "serverName": "test-server",
                        "build": "12345"
                    }}]
                }))
            } else if status == 429 {
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "1")
                    .set_body_json(serde_json::json!({
                        "messages": [{"type": "ERROR", "text": "Rate limited"}]
                    }))
            } else {
                ResponseTemplate::new(status).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Server error"}]
                }))
            }
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();

    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 10, None).await;
    assert!(
        result.is_ok(),
        "Should eventually succeed with random chaos: {:?}",
        result.err()
    );
}

/// Test load balancer flapping between healthy and unhealthy backends.
///
/// Simulates a load balancer that alternates between sending requests
/// to healthy and unhealthy backend servers.
#[tokio::test]
async fn test_load_balancer_flapping() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    // Pattern: 502, 503, 200, 502, 503, 200... (simulating 3 backends, one healthy)
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            match count % 3 {
                0 => ResponseTemplate::new(502).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Bad gateway"}]
                })),
                1 => ResponseTemplate::new(503).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Service unavailable"}]
                })),
                _ => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {
                        "version": "9.0.0",
                        "serverName": "test-server",
                        "build": "12345"
                    }}]
                })),
            }
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();

    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 5, None).await;
    assert!(
        result.is_ok(),
        "Should succeed despite load balancer flapping: {:?}",
        result.err()
    );
}

/// Test retry exhaustion with continuous flapping.
///
/// Verifies that the client gives up after max retries when flapping continues.
#[tokio::test]
async fn test_flapping_retry_exhaustion() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    // Always returns retryable errors
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            // Cycle through different retryable errors
            let statuses = [503, 502, 504, 429];
            let status = statuses[count % statuses.len()];

            if status == 429 {
                ResponseTemplate::new(429)
                    .insert_header("retry-after", "1")
                    .set_body_json(serde_json::json!({
                        "messages": [{"type": "ERROR", "text": "Rate limited"}]
                    }))
            } else {
                ResponseTemplate::new(status).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Server error"}]
                }))
            }
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
        "Should exhaust all retry attempts"
    );
}

/// Test rapid state changes with different error messages.
///
/// Verifies that the client handles varying error messages during flapping.
#[tokio::test]
async fn test_flapping_with_varying_error_messages() {
    let mock_server = MockServer::start().await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            match count {
                0 => ResponseTemplate::new(503).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Service temporarily unavailable"}]
                })),
                1 => ResponseTemplate::new(503).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Backend server at capacity"}]
                })),
                2 => ResponseTemplate::new(502).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Bad gateway from upstream"}]
                })),
                _ => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {
                        "version": "9.0.0",
                        "serverName": "test-server",
                        "build": "12345"
                    }}]
                })),
            }
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();

    let result =
        endpoints::get_server_info(&client, &mock_server.uri(), "test-token", 5, None).await;
    assert!(
        result.is_ok(),
        "Should succeed despite varying error messages: {:?}",
        result.err()
    );
}
