//! Authentication endpoint tests.
//!
//! This module tests the Splunk authentication endpoints, including:
//! - Successful login with session key extraction
//! - Invalid credential handling
//! - Login response format validation
//!
//! # Invariants
//! - Login response must have sessionKey at the top level, not nested under entry[0].content
//! - 401 responses must return ApiError with appropriate status code
//!
//! # What this does NOT handle
//! - Session token refresh/retry logic (see retry_tests.rs)
//! - API token authentication (tested via SplunkClient in other modules)

mod common;

use common::*;
use splunk_client::ClientError;
use wiremock::matchers::{method, path, query_param};

#[tokio::test]
async fn test_login_success() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("auth/login_success.json");

    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::login(
        &client,
        &mock_server.uri(),
        "admin",
        "testpassword",
        3,
        None,
        None,
    )
    .await;

    if let Err(ref e) = result {
        eprintln!("Login error: {:?}", e);
    }
    assert!(result.is_ok());
    assert_eq!(result.unwrap(), "test-session-key-12345678");
}

#[tokio::test]
async fn test_login_invalid_credentials() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("auth/login_invalid_creds.json");

    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(401).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::login(
        &client,
        &mock_server.uri(),
        "admin",
        "wrongpassword",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    // 401 is now classified to AuthFailed/Unauthorized variant
    assert!(
        matches!(
            err,
            ClientError::AuthFailed(_) | ClientError::Unauthorized(_)
        ),
        "Expected auth error, got {:?}",
        err
    );
}

#[tokio::test]
async fn test_login_response_format_regression() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("auth/login_success.json");

    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::login(
        &client,
        &mock_server.uri(),
        "admin",
        "testpassword",
        3,
        None,
        None,
    )
    .await;

    assert!(result.is_ok());
    let token = result.unwrap();
    assert_eq!(token, "test-session-key-12345678");

    let fixture_value: serde_json::Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("fixtures/auth/login_success.json"),
        )
        .unwrap(),
    )
    .unwrap();

    assert!(
        fixture_value.get("sessionKey").is_some(),
        "Login response must have sessionKey at top level"
    );
    assert!(
        fixture_value.get("entry").is_none()
            || fixture_value["entry"][0]["content"]
                .get("sessionKey")
                .is_none(),
        "Login response must NOT have sessionKey nested under entry[0][content]"
    );
}
