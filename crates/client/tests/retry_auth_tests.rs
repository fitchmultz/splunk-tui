//! Session authentication retry behavior tests.
//!
//! This module tests the client's retry logic for HTTP 401 Unauthorized and
//! 403 Forbidden responses when using session token authentication.
//!
//! # Invariants
//! - 401/403 trigger session re-authentication only for SessionToken auth strategy
//! - API token auth does NOT trigger re-authentication on 401/403
//! - Re-authentication is attempted once per request
//!
//! # What this does NOT handle
//! - API token authentication (see test_no_retry_on_401_api_token)
//! - Other authentication methods

mod common;

use common::*;
use secrecy::SecretString;
use splunk_client::{AuthStrategy, ClientError, SplunkClient};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::matchers::{method, path, query_param};

#[tokio::test]
async fn test_retry_on_401_session_auth() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");
    let list_indexes_fixture = load_fixture("indexes/list_indexes.json");

    // Track login requests using Arc<AtomicUsize>
    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    // Mock login endpoint - returns fresh session key
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_: &wiremock::Request| {
            login_count_clone.fetch_add(1, Ordering::SeqCst);
            ResponseTemplate::new(200).set_body_json(&login_fixture)
        })
        .mount(&mock_server)
        .await;

    // First call to list_indexes returns 401, second returns 200
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
        .respond_with(ResponseTemplate::new(200).set_body_json(&list_indexes_fixture))
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

    // This should trigger a retry with re-login
    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_ok());
    let indexes = result.unwrap();
    assert_eq!(indexes.len(), 3);

    // Should have called login twice (initial + retry)
    assert_eq!(login_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_retry_on_403_session_auth() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");
    let job_fixture = load_fixture("search/create_job_success.json");

    // Track login requests using Arc<AtomicUsize>
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

    // First call to create_job returns 403, second returns 200
    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Forbidden - session expired"}]
        })))
        .up_to_n_times(1)
        .mount(&mock_server)
        .await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&job_fixture))
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

    // This should trigger a retry with re-login
    let options = splunk_client::endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };
    let result = client
        .create_search_job("search index=main", &options)
        .await;

    assert!(result.is_ok());
    let sid = result.unwrap();
    assert!(sid.contains("scheduler__admin__search"));

    // Should have called login twice (initial + retry)
    assert_eq!(login_count.load(Ordering::SeqCst), 2);
}

#[tokio::test]
async fn test_no_retry_on_401_api_token() {
    let mock_server = MockServer::start().await;

    // API token auth - return 401
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Invalid token"}]
        })))
        .mount(&mock_server)
        .await;

    // Should never be called for API token auth
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(serde_json::json!({
            "sessionKey": "should-not-be-called"
        })))
        .mount(&mock_server)
        .await;

    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("invalid-token".to_string().into()),
    };

    let client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // Should fail immediately without retry
    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    // "Invalid token" message is classified as Unauthorized variant
    assert!(
        matches!(err, ClientError::Unauthorized(_)),
        "Expected Unauthorized, got {:?}",
        err
    );
}

#[tokio::test]
async fn test_retry_fails_on_second_401() {
    let mock_server = MockServer::start().await;

    let login_fixture = load_fixture("auth/login_success.json");

    // Mock login endpoint
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&login_fixture))
        .mount(&mock_server)
        .await;

    // Always return 401 even after retry
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
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

    // Initial login
    client.login().await.unwrap();

    // Should fail even after retry
    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    // "Session expired" message is classified as SessionExpired variant
    assert!(
        matches!(err, ClientError::SessionExpired { .. }),
        "Expected SessionExpired, got {:?}",
        err
    );
}
