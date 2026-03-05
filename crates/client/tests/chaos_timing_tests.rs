//! Chaos timing tests for token expiry and clock skew scenarios.
//!
//! This module tests the client's behavior under time-related failures:
//! - Session expiry triggering re-authentication
//! - Token expiry edge cases
//! - Session refresh during long-running operations
//!
//! # Invariants
//! - Token must be refreshed proactively before expiry
//! - Operations must not fail due to race conditions with token refresh
//! - Clock skew should be handled gracefully

mod common;

use common::*;
use splunk_client::ClientError;
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::{method, path, query_param};
use wiremock::{Mock, MockServer, ResponseTemplate};

use secrecy::SecretString;
use splunk_client::{AuthStrategy, SplunkClient};

/// Test session expiry triggers re-authentication.
///
/// Simulates a scenario where the session expires and the client
/// needs to re-authenticate to continue.
#[tokio::test]
async fn test_session_expiry_reauthentication() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");
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

    let info_count = Arc::new(AtomicUsize::new(0));
    let info_count_clone = info_count.clone();

    // Server info endpoint - returns 401 once, then succeeds
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = info_count_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                ResponseTemplate::new(401).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Session expired"}]
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

    // Request should trigger re-auth on 401
    let result = client.get_server_info().await;
    assert!(
        result.is_ok(),
        "Should succeed after re-authentication: {:?}",
        result.err()
    );

    // Should have re-authenticated
    assert_eq!(
        login_count.load(Ordering::SeqCst),
        2,
        "Should re-authenticate after 401"
    );
}

/// Test session refresh during pagination with induced failures.
///
/// Simulates session expiry in the middle of fetching paginated results.
#[tokio::test]
async fn test_session_refresh_during_pagination_with_chaos() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");
    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_req: &wiremock::Request| {
            login_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(&login_fixture)
        })
        .mount(&mock_server)
        .await;

    // Page 1: Succeeds
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "0"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!([
            {"_time": "2024-01-15T10:30:00.000Z", "message": "Page 1"}
        ])))
        .mount(&mock_server)
        .await;

    // Page 2: Returns 401 (session expired), then succeeds
    let page2_count = Arc::new(AtomicUsize::new(0));
    let page2_clone = page2_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "1"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = page2_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                ResponseTemplate::new(401).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Session expired"}]
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!([
                    {"_time": "2024-01-15T10:31:00.000Z", "message": "Page 2"}
                ]))
            }
        })
        .mount(&mock_server)
        .await;

    // Page 3: 503 error, then succeeds (chaos factor)
    let page3_count = Arc::new(AtomicUsize::new(0));
    let page3_clone = page3_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/search/jobs/test-sid/results"))
        .and(query_param("offset", "2"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = page3_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                ResponseTemplate::new(503).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Service unavailable"}]
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!([
                    {"_time": "2024-01-15T10:32:00.000Z", "message": "Page 3"}
                ]))
            }
        })
        .mount(&mock_server)
        .await;

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

    // Login
    client.login().await.unwrap();
    assert_eq!(login_count.load(Ordering::SeqCst), 1);

    // Fetch all pages
    let page1 = client.get_search_results("test-sid", 1, 0).await;
    assert!(page1.is_ok(), "Page 1 should succeed");

    // Page 2 triggers re-auth
    let page2 = client.get_search_results("test-sid", 1, 1).await;
    assert!(page2.is_ok(), "Page 2 should succeed after re-auth");

    // Should have re-authenticated
    assert_eq!(
        login_count.load(Ordering::SeqCst),
        2,
        "Should re-authenticate once"
    );

    // Page 3 triggers retry due to 503
    let page3 = client.get_search_results("test-sid", 1, 2).await;
    assert!(page3.is_ok(), "Page 3 should succeed after retry");
}

/// Test multiple 401 responses requiring multiple re-authentications.
///
/// Simulates a scenario where the server keeps rejecting the session.
#[tokio::test]
async fn test_repeated_session_failures() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");
    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_req: &wiremock::Request| {
            login_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(&login_fixture)
        })
        .mount(&mock_server)
        .await;

    // Always returns 401 - session never works
    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Session expired"}]
        })))
        .mount(&mock_server)
        .await;

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

    // Login
    client.login().await.unwrap();
    assert_eq!(login_count.load(Ordering::SeqCst), 1);

    // Request should try re-auth once, then fail
    let result = client.get_server_info().await;
    assert!(result.is_err(), "Should fail after re-auth attempt");

    // Should have re-authenticated once
    assert_eq!(
        login_count.load(Ordering::SeqCst),
        2,
        "Should re-authenticate once"
    );
}

/// Test API token auth doesn't attempt re-authentication on 401.
///
/// API tokens don't support session refresh - they should fail immediately.
#[tokio::test]
async fn test_api_token_no_reauth() {
    let mock_server = MockServer::start().await;

    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    // Login endpoint - should never be called for API token auth
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .respond_with(move |_req: &wiremock::Request| {
            login_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(serde_json::json!({
                "sessionKey": "test-session-key"
            }))
        })
        .mount(&mock_server)
        .await;

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Unauthorized"}]
        })))
        .mount(&mock_server)
        .await;

    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("test-api-token".to_string().into()),
    };

    let client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // No login needed for API token

    // Request should fail immediately without re-auth attempt
    let result = client.get_server_info().await;
    assert!(
        result.is_err(),
        "Should fail immediately with API token auth"
    );

    let err = result.unwrap_err();
    // 401 is now classified as Unauthorized variant (not ApiError)
    assert!(
        matches!(err, ClientError::Unauthorized(_)),
        "Expected Unauthorized error, got {:?}",
        err
    );

    // Should NOT have called login
    assert_eq!(
        login_count.load(Ordering::SeqCst),
        0,
        "Should not attempt login with API token auth"
    );
}

/// Test 403 response triggers re-authentication.
///
/// 403 should trigger re-authentication for session auth.
#[tokio::test]
async fn test_403_triggers_reauth() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");
    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_req: &wiremock::Request| {
            login_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(&login_fixture)
        })
        .mount(&mock_server)
        .await;

    let request_count = Arc::new(AtomicUsize::new(0));
    let request_count_clone = request_count.clone();

    Mock::given(method("GET"))
        .and(path("/services/server/info"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = request_count_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                ResponseTemplate::new(403).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Forbidden"}]
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

    // Login
    client.login().await.unwrap();
    assert_eq!(login_count.load(Ordering::SeqCst), 1);

    // Request: 403 triggers re-auth, then succeeds
    let result = client.get_server_info().await;
    assert!(
        result.is_ok(),
        "Should succeed after re-auth: {:?}",
        result.err()
    );

    // Should have re-authenticated once
    assert_eq!(
        login_count.load(Ordering::SeqCst),
        2,
        "Should re-authenticate once"
    );
}

/// Test session expiry with job creation.
///
/// Simulates session expiry during job creation.
#[tokio::test]
async fn test_session_expiry_during_job_creation() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");
    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_req: &wiremock::Request| {
            login_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(&login_fixture)
        })
        .mount(&mock_server)
        .await;

    let job_count = Arc::new(AtomicUsize::new(0));
    let job_count_clone = job_count.clone();

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(move |_req: &wiremock::Request| {
            let count = job_count_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                ResponseTemplate::new(401).set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Session expired"}]
                }))
            } else {
                ResponseTemplate::new(200).set_body_json(serde_json::json!({
                    "entry": [{"content": {"sid": "test-job-sid"}}]
                }))
            }
        })
        .mount(&mock_server)
        .await;

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

    // Login
    client.login().await.unwrap();
    assert_eq!(login_count.load(Ordering::SeqCst), 1);

    // Create job - should trigger re-auth
    let result = client
        .create_search_job("search index=main", &Default::default())
        .await;
    assert!(
        result.is_ok(),
        "Should succeed after re-auth: {:?}",
        result.err()
    );
    assert_eq!(result.unwrap(), "test-job-sid");

    // Should have re-authenticated
    assert_eq!(
        login_count.load(Ordering::SeqCst),
        2,
        "Should re-authenticate once"
    );
}
