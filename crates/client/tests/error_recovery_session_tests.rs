//! Error recovery tests for session expiry scenarios.
//!
//! This module tests error recovery when sessions expire during long-running
//! operations like job polling:
//! - Session expiry during job status polling
//! - Re-authentication and continuation of operations
//!
//! # Invariants
//! - Session expiry should trigger re-authentication
//! - Operations should continue after successful re-authentication
//!
//! # What this does NOT handle
//! - Session expiry during pagination (see error_recovery_pagination_tests.rs)
//! - Basic auth retry logic (see retry_auth_tests.rs)

mod common;

use common::*;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::{method, path, query_param};

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

    let client = SplunkClient::builder()
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
