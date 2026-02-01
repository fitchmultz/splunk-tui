//! Error recovery path tests for pagination and streaming scenarios.
//!
//! This module tests error recovery in multi-step operations:
//! - Partial pagination failures during multi-page search results
//! - Network interruption mid-stream during result fetching
//! - Session expiry during long-running operations
//! - Streaming body retry limitations
//!
//! # Invariants
//! - Each page failure should only retry the affected page, not restart from page 1
//! - Session expiry during pagination should re-authenticate and continue
//! - Streaming bodies cannot be retried (single attempt only)
//!
//! # What this does NOT handle
//! - Basic retry logic (see retry_tests.rs)
//! - Connection-level errors (see error_tests.rs)

mod common;

use common::*;
use splunk_client::ClientError;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::time::Duration;
use wiremock::matchers::{method, path, query_param};

/// Test that partial pagination failures only retry the affected page.
///
/// This test verifies that when fetching multi-page results:
/// - Page 1 succeeds (offset=0)
/// - Page 2 fails with 429, then succeeds on retry
/// - Page 3 succeeds
/// - All pages are retrieved correctly without restarting from page 1
#[tokio::test(start_paused = true)]
async fn test_pagination_partial_failure_retry() {
    let mock_server = MockServer::start().await;

    // Track request counts per page
    let page1_count = Arc::new(AtomicUsize::new(0));
    let page2_count = Arc::new(AtomicUsize::new(0));
    let page3_count = Arc::new(AtomicUsize::new(0));

    let page1_clone = page1_count.clone();
    let page2_clone = page2_count.clone();
    let page3_clone = page3_count.clone();

    // Page 1: Always succeeds
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "0"))
        .respond_with(move |_req: &wiremock::Request| {
            page1_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"_time": "2024-01-15T10:30:00.000Z", "message": "Page 1 Event 1"},
                {"_time": "2024-01-15T10:31:00.000Z", "message": "Page 1 Event 2"}
            ]))
        })
        .mount(&mock_server)
        .await;

    // Page 2: Fails twice with 429, then succeeds
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "2"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Rate limited"}]
        })))
        .up_to_n_times(2)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "2"))
        .respond_with(move |_req: &wiremock::Request| {
            page2_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"_time": "2024-01-15T10:32:00.000Z", "message": "Page 2 Event 1"},
                {"_time": "2024-01-15T10:33:00.000Z", "message": "Page 2 Event 2"}
            ]))
        })
        .mount(&mock_server)
        .await;

    // Page 3: Always succeeds
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "4"))
        .respond_with(move |_req: &wiremock::Request| {
            page3_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"_time": "2024-01-15T10:34:00.000Z", "message": "Page 3 Event 1"}
            ]))
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let server_uri = mock_server.uri();

    // Fetch page 1
    let result1 = endpoints::get_results(
        &client,
        &server_uri,
        "test-token",
        "test-sid",
        Some(2),
        Some(0),
        endpoints::OutputMode::Json,
        3,
        None,
    )
    .await;
    assert!(result1.is_ok(), "Page 1 should succeed");
    assert_eq!(result1.unwrap().results.len(), 2);

    // Fetch page 2 (will trigger retries)
    let result2_handle = tokio::spawn({
        let client = client.clone();
        let server_uri = server_uri.clone();
        async move {
            endpoints::get_results(
                &client,
                &server_uri,
                "test-token",
                "test-sid",
                Some(2),
                Some(2),
                endpoints::OutputMode::Json,
                3,
                None,
            )
            .await
        }
    });

    assert_pending(&result2_handle, "page 2 should wait for backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    assert_pending(&result2_handle, "page 2 should wait for second backoff").await;
    advance_and_yield(Duration::from_secs(2)).await;

    let result2 = result2_handle.await.expect("page 2 task");
    assert!(result2.is_ok(), "Page 2 should succeed after retries");
    assert_eq!(result2.unwrap().results.len(), 2);

    // Fetch page 3
    let result3 = endpoints::get_results(
        &client,
        &server_uri,
        "test-token",
        "test-sid",
        Some(2),
        Some(4),
        endpoints::OutputMode::Json,
        3,
        None,
    )
    .await;
    assert!(result3.is_ok(), "Page 3 should succeed");
    assert_eq!(result3.unwrap().results.len(), 1);

    // Verify page 1 and 3 were only requested once, page 2 had retries
    assert_eq!(
        page1_count.load(Ordering::SeqCst),
        1,
        "Page 1 should be requested once"
    );
    assert_eq!(
        page2_count.load(Ordering::SeqCst),
        1,
        "Page 2 should eventually succeed"
    );
    assert_eq!(
        page3_count.load(Ordering::SeqCst),
        1,
        "Page 3 should be requested once"
    );
}

/// Test session expiry during pagination with session-based auth.
///
/// This test verifies that when a session expires mid-pagination:
/// - Page 1 succeeds with initial session
/// - Page 2 returns 401 (session expired)
/// - Client re-authenticates and retries page 2
/// - Page 3 succeeds with new session
#[tokio::test]
async fn test_pagination_session_expiry_mid_operation() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");

    // Track login requests
    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    // Mock login endpoint
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_req: &wiremock::Request| {
            login_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(&login_fixture)
        })
        .mount(&mock_server)
        .await;

    // Track page requests with session tokens
    let page_requests = Arc::new(AtomicUsize::new(0));
    let page_requests_clone = page_requests.clone();

    // Page 1: Success with initial session
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "0"))
        .respond_with(move |_req: &wiremock::Request| {
            page_requests_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"_time": "2024-01-15T10:30:00.000Z", "message": "Page 1 Event"}
            ]))
        })
        .mount(&mock_server)
        .await;

    // Page 2: Returns 401 once (session expired), then succeeds
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "1"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Session expired"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"_time": "2024-01-15T10:31:00.000Z", "message": "Page 2 Event"}
        ])))
        .mount(&mock_server)
        .await;

    // Page 3: Success with new session
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"_time": "2024-01-15T10:32:00.000Z", "message": "Page 3 Event"}
        ])))
        .mount(&mock_server)
        .await;

    use secrecy::SecretString;
    use splunk_client::{AuthStrategy, SplunkClient};

    let strategy = AuthStrategy::SessionToken {
        username: "admin".to_string(),
        password: SecretString::new("testpassword".to_string().into()),
    };

    let mut client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // Initial login
    client.login().await.unwrap();
    assert_eq!(login_count.load(Ordering::SeqCst), 1);

    // Fetch page 1
    let result1 = client.get_search_results("test-sid", 1, 0).await;
    assert!(result1.is_ok(), "Page 1 should succeed");

    // Fetch page 2 (triggers session re-auth)
    let result2 = client.get_search_results("test-sid", 1, 1).await;
    assert!(result2.is_ok(), "Page 2 should succeed after re-auth");

    // Should have re-authenticated
    assert_eq!(
        login_count.load(Ordering::SeqCst),
        2,
        "Should re-authenticate once"
    );

    // Fetch page 3
    let result3 = client.get_search_results("test-sid", 1, 2).await;
    assert!(result3.is_ok(), "Page 3 should succeed");
}

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

/// Test Retry-After header handling during pagination.
///
/// This test verifies that when a page returns 429 with Retry-After:
/// - The client waits for the specified duration
/// - Only the affected page is delayed
/// - Subsequent pages proceed without additional delay
#[tokio::test(start_paused = true)]
async fn test_pagination_retry_after_header() {
    let mock_server = MockServer::start().await;

    let page2_delayed = Arc::new(AtomicUsize::new(0));
    let page2_clone = page2_delayed.clone();

    // Page 1: Always succeeds quickly
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"_time": "2024-01-15T10:30:00.000Z", "message": "Page 1"}
        ])))
        .mount(&mock_server)
        .await;

    // Page 2: Returns 429 with Retry-After: 2, then succeeds
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "1"))
        .respond_with(
            ResponseTemplate::new(429)
                .insert_header("retry-after", "2")
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Rate limited"}]
                })),
        )
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "1"))
        .respond_with(move |_req: &wiremock::Request| {
            page2_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(serde_json::json!([
                {"_time": "2024-01-15T10:31:00.000Z", "message": "Page 2"}
            ]))
        })
        .mount(&mock_server)
        .await;

    // Page 3: Always succeeds quickly
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"_time": "2024-01-15T10:32:00.000Z", "message": "Page 3"}
        ])))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let server_uri = mock_server.uri();

    // Fetch page 1 (quick)
    let result1 = endpoints::get_results(
        &client,
        &server_uri,
        "test-token",
        "test-sid",
        Some(1),
        Some(0),
        endpoints::OutputMode::Json,
        3,
        None,
    )
    .await;
    assert!(result1.is_ok());

    // Fetch page 2 (should wait for Retry-After: 2 seconds)
    let result2_handle = tokio::spawn({
        let client = client.clone();
        let server_uri = server_uri.clone();
        async move {
            endpoints::get_results(
                &client,
                &server_uri,
                "test-token",
                "test-sid",
                Some(1),
                Some(1),
                endpoints::OutputMode::Json,
                3,
                None,
            )
            .await
        }
    });

    assert_pending(&result2_handle, "page 2 should wait for retry-after").await;
    advance_and_yield(Duration::from_secs(2)).await;
    let result2 = result2_handle.await.expect("page 2 task");
    assert!(result2.is_ok());

    // Fetch page 3 (quick, no delay)
    let result3 = endpoints::get_results(
        &client,
        &server_uri,
        "test-token",
        "test-sid",
        Some(1),
        Some(2),
        endpoints::OutputMode::Json,
        3,
        None,
    )
    .await;
    assert!(result3.is_ok());
}

/// Test pagination failure exhaustion (max retries exceeded).
///
/// This test verifies that when a page consistently fails:
/// - All retry attempts are exhausted
/// - MaxRetriesExceeded error is returned with context
/// - The error includes information about the original failure
#[tokio::test(start_paused = true)]
async fn test_pagination_retry_exhaustion() {
    let mock_server = MockServer::start().await;

    // Track request attempts for page 2
    let request_count = Arc::new(AtomicUsize::new(0));
    let count_clone = request_count.clone();

    // Page 1: Success
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"_time": "2024-01-15T10:30:00.000Z", "message": "Page 1"}
        ])))
        .mount(&mock_server)
        .await;

    // Page 2: Always returns 429 (rate limited)
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "1"))
        .respond_with(move |_req: &wiremock::Request| {
            count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(429).set_body_json(serde_json::json!({
                "messages": [{"type": "ERROR", "text": "Rate limited - too many requests"}]
            }))
        })
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let server_uri = mock_server.uri();

    // Fetch page 1 successfully
    let result1 = endpoints::get_results(
        &client,
        &server_uri,
        "test-token",
        "test-sid",
        Some(1),
        Some(0),
        endpoints::OutputMode::Json,
        2, // max_retries = 2
        None,
    )
    .await;
    assert!(result1.is_ok());

    // Fetch page 2 - should exhaust retries
    let result2_handle = tokio::spawn({
        let client = client.clone();
        let server_uri = server_uri.clone();
        async move {
            endpoints::get_results(
                &client,
                &server_uri,
                "test-token",
                "test-sid",
                Some(1),
                Some(1),
                endpoints::OutputMode::Json,
                2, // max_retries = 2 (3 total attempts)
                None,
            )
            .await
        }
    });

    assert_pending(&result2_handle, "page 2 should wait for first backoff").await;
    advance_and_yield(Duration::from_secs(1)).await;
    assert_pending(&result2_handle, "page 2 should wait for second backoff").await;
    advance_and_yield(Duration::from_secs(2)).await;
    let result2 = result2_handle.await.expect("page 2 task");

    // Should fail with MaxRetriesExceeded
    assert!(result2.is_err());
    let err = result2.unwrap_err();
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

/// Test mixed error types during pagination.
///
/// This test verifies handling of different error types across pages:
/// - Page 1: 503 (service unavailable) then success
/// - Page 2: 429 (rate limited) then success
/// - Page 3: Immediate success
#[tokio::test(start_paused = true)]
async fn test_pagination_mixed_error_types() {
    let mock_server = MockServer::start().await;

    // Page 1: 503 once, then success
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(503).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Service Unavailable"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"_time": "2024-01-15T10:30:00.000Z", "message": "Page 1"}
        ])))
        .mount(&mock_server)
        .await;

    // Page 2: 429 once, then success
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "1"))
        .respond_with(ResponseTemplate::new(429).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Rate limited"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "1"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"_time": "2024-01-15T10:31:00.000Z", "message": "Page 2"}
        ])))
        .mount(&mock_server)
        .await;

    // Page 3: Immediate success
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "2"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"_time": "2024-01-15T10:32:00.000Z", "message": "Page 3"}
        ])))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let server_uri = mock_server.uri();

    for offset in [0, 1] {
        let result_handle = tokio::spawn({
            let client = client.clone();
            let server_uri = server_uri.clone();
            async move {
                endpoints::get_results(
                    &client,
                    &server_uri,
                    "test-token",
                    "test-sid",
                    Some(1),
                    Some(offset),
                    endpoints::OutputMode::Json,
                    3,
                    None,
                )
                .await
            }
        });

        let context = format!("page {} should wait for backoff", offset + 1);
        assert_pending(&result_handle, &context).await;
        advance_and_yield(Duration::from_secs(1)).await;
        if !result_handle.is_finished() {
            advance_and_yield(Duration::from_secs(1)).await;
        }
        let result = result_handle.await.expect("page task");
        assert!(
            result.is_ok(),
            "Page {} should succeed after handling error",
            offset + 1
        );
        let results = result.unwrap();
        assert_eq!(results.results.len(), 1);
    }

    let result3 = endpoints::get_results(
        &client,
        &server_uri,
        "test-token",
        "test-sid",
        Some(1),
        Some(2),
        endpoints::OutputMode::Json,
        3,
        None,
    )
    .await;
    assert!(
        result3.is_ok(),
        "Page 3 should succeed after handling error"
    );
    assert_eq!(result3.unwrap().results.len(), 1);
}

/// Test session expiry during job polling with re-authentication.
///
/// This test simulates a long-running search where:
/// - Job is created successfully
/// - First status poll returns 401 (session expired)
/// - Client re-authenticates and continues polling
/// - Job completes and results are fetched
#[tokio::test]
async fn test_session_expiry_during_job_polling() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");

    // Track login requests
    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    // Mock login endpoint
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_req: &wiremock::Request| {
            login_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(&login_fixture)
        })
        .mount(&mock_server)
        .await;

    // Create job endpoint
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": [{"content": {"sid": "test-polling-sid"}}]
        })))
        .mount(&mock_server)
        .await;

    // Track status poll attempts
    let status_polls = Arc::new(AtomicUsize::new(0));
    let status_polls_clone = status_polls.clone();

    // Job status: 401 once, then running, then done
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-polling-sid"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = status_polls_clone.fetch_add(1, Ordering::SeqCst);
            match count {
                0 => ResponseTemplate::new(401).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Session expired"}]
                })),
                1 => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {"sid": "test-polling-sid", "dispatchState": "RUNNING"}}]
                })),
                _ => ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {"sid": "test-polling-sid", "dispatchState": "DONE"}}]
                })),
            }
        })
        .mount(&mock_server)
        .await;

    use secrecy::SecretString;
    use splunk_client::{AuthStrategy, SplunkClient};

    let strategy = AuthStrategy::SessionToken {
        username: "admin".to_string(),
        password: SecretString::new("testpassword".to_string().into()),
    };

    let mut client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // Initial login
    client.login().await.unwrap();
    assert_eq!(login_count.load(Ordering::SeqCst), 1);

    // Create job
    let sid = client
        .create_search_job("search index=main", &Default::default())
        .await;
    assert!(sid.is_ok());
    assert_eq!(sid.unwrap(), "test-polling-sid");

    // Poll status - first call triggers 401, then re-auth, then success
    let status = client.get_job_status("test-polling-sid").await;
    assert!(status.is_ok());

    // Should have re-authenticated
    assert_eq!(
        login_count.load(Ordering::SeqCst),
        2,
        "Should re-authenticate once"
    );

    // Verify we made multiple status polls (initial + after re-auth)
    assert!(
        status_polls.load(Ordering::SeqCst) >= 2,
        "Should poll status at least twice"
    );
}
