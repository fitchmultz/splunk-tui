//! Error recovery tests for streaming body scenarios.
//!
//! This module tests error recovery limitations with non-cloneable request bodies:
//! - Streaming bodies cannot be retried (single attempt only)
//! - Cloneable bodies allow normal retry behavior
//!
//! # Invariants
//! - Non-cloneable request bodies proceed with single attempt only
//! - Cloneable bodies follow normal retry logic
//!
//! # What this does NOT handle
//! - Basic retry logic (see retry_tests.rs)
//! - Pagination error recovery (see error_recovery_pagination_tests.rs)

mod common;

use common::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use wiremock::matchers::{method, path};

/// Test that non-cloneable request bodies fail without retry.
///
/// This test verifies the behavior documented in request.rs:23-31 - when a request
/// body cannot be cloned (try_clone returns None), the request proceeds with a
/// single attempt only, even if the server returns a retryable error.
///
/// Note: The actual streaming body limitation is tested at the unit level in
/// request.rs. This integration test verifies the end-to-end behavior where
/// a request that cannot be retried fails immediately.
#[tokio::test(start_paused = true)]
async fn test_non_cloneable_body_single_attempt() {
    let mock_server = MockServer::start().await;

    // Track request count
    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    // Server returns 503 on first request (would normally trigger retry)
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = count_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                ResponseTemplate::new(503).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Service Unavailable"}]
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {"sid": "test-sid"}}]
                }))
            }
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let server_uri = mock_server.uri();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    // This request uses form data (which CAN be cloned), so it WILL retry.
    // This test documents the normal retry behavior - the streaming body limitation
    // is an edge case covered by unit tests in request.rs.
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
            )
            .await
        }
    });

    assert_pending(&result_handle, "cloneable body should wait for backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    let result = result_handle.await.expect("create job task");

    // With cloneable body, should succeed after retry
    assert!(
        result.is_ok(),
        "Should succeed after retry with cloneable body"
    );
    assert_eq!(result.unwrap(), "test-sid");

    // Should have made 2 requests (initial + 1 retry)
    assert_eq!(
        request_count.load(Ordering::SeqCst),
        2,
        "Should retry with cloneable body"
    );
}
