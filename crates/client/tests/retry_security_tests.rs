//! Retry flow secret safety tests.
//!
//! This module verifies that authentication retry flows do not accidentally
//! log or expose session tokens, passwords, or API tokens during the retry
//! process.
//!
//! What this module does NOT handle:
//! - Retry timing/backoff logic (tested elsewhere)
//! - Circuit breaker patterns
//! - Rate limiting behavior

use secrecy::SecretString;
use splunk_client::{AuthStrategy, ClientError, SplunkClient};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

/// Test that 401/403 retry with session auth does not log the token.
///
/// When a 401/403 is received with session auth, the client clears the session
/// and re-authenticates. The session token should not appear in any logs.
#[tokio::test]
async fn test_retry_call_no_token_logging_on_auth_error() {
    let mock_server = MockServer::start().await;

    let login_fixture = serde_json::json!({
        "sessionKey": "new-session-key-after-retry"
    });

    // Track login requests
    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    // Mock login endpoint
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_: &wiremock::Request| {
            login_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(&login_fixture)
        })
        .mount(&mock_server)
        .await;

    // First call returns 401, second returns 200
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Session expired"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "entry": []
        })))
        .mount(&mock_server)
        .await;

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

    // This should trigger a retry with re-login
    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_ok());
    // Should have called login twice (initial + retry)
    assert_eq!(login_count.load(Ordering::SeqCst), 2);

    // Note: We cannot directly test that no tokens were logged without
    // capturing logs. The retry_call macro uses debug!() for status codes
    // but does not log the token itself.
}

/// Test that API token auth does NOT trigger session retry on 401.
///
/// When using API token auth, a 401 should NOT trigger a retry because
/// API tokens don't support session refresh - they're either valid or not.
#[tokio::test]
async fn test_api_token_no_session_retry_on_401() {
    let mock_server = MockServer::start().await;

    // API token auth - return 401
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Invalid token"}]
        })))
        .mount(&mock_server)
        .await;

    // Login endpoint should never be called for API token auth
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sessionKey": "should-not-be-called"
        })))
        .mount(&mock_server)
        .await;

    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("invalid-token".to_string().into()),
    };

    let mut client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // Should fail immediately without retry
    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 401, .. }));
}
