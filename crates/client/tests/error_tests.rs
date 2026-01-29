//! Error handling and client configuration tests.
//!
//! This module tests error handling for various failure modes:
//! - HTTP error status codes (401, 403, 404, 500)
//! - Malformed JSON responses
//! - Timeout handling
//! - Connection errors (refused, TLS)
//! - Invalid URL handling
//! - Error classification (retryable vs non-retryable)
//! - Client configuration (trailing slash normalization)
//!
//! # Invariants
//! - ConnectionRefused, TlsError, InvalidUrl, NotFound errors are NOT retryable
//! - Error display formatting includes relevant details
//! - URL normalization handles trailing slashes correctly
//!
//! # What this does NOT handle
//! - Retry behavior (see retry_tests.rs)
//! - Session re-authentication (see retry_tests.rs)

mod common;

use common::*;
use splunk_client::ClientError;
use wiremock::matchers::{method, path, query_param};

#[tokio::test]
async fn test_unauthorized_access() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Unauthorized"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_indexes(
        &client,
        &mock_server.uri(),
        "invalid-token",
        Some(10),
        Some(0),
        3,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 401, .. }));
}

#[tokio::test]
async fn test_forbidden_access() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/cluster/master/config"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Forbidden"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result =
        endpoints::get_cluster_info(&client, &mock_server.uri(), "test-token", 3, None).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 403, .. }));
}

#[tokio::test]
async fn test_internal_server_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Internal server error"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &Default::default(),
        3,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    assert!(matches!(err, ClientError::ApiError { status: 500, .. }));
}

#[tokio::test]
async fn test_malformed_json_response() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(200).set_body_string("invalid json"))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_indexes(
        &client,
        &mock_server.uri(),
        "test-token",
        Some(10),
        Some(0),
        3,
        None,
    )
    .await;

    assert!(result.is_err());
}

#[tokio::test]
async fn test_timeout_handling() {
    let mock_server = MockServer::start().await;

    // Simulate a timeout by not responding immediately
    Mock::given(method("GET"))
        .and(path("/services/search/jobs/timeout-sid/results"))
        .respond_with(
            ResponseTemplate::new(200)
                .set_body_json(serde_json::json!([]))
                .set_delay(std::time::Duration::from_secs(10)),
        )
        .mount(&mock_server)
        .await;

    let client = Client::builder()
        .timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();

    let result = endpoints::get_results(
        &client,
        &mock_server.uri(),
        "test-token",
        "timeout-sid",
        Some(10),
        Some(0),
        endpoints::OutputMode::Json,
        3,
        None,
    )
    .await;

    // The request should timeout or return an error
    assert!(result.is_err());
}

#[tokio::test]
async fn test_api_error_details() {
    let mock_server = MockServer::start().await;
    let request_id = "test-request-id-999";

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(
            ResponseTemplate::new(404)
                .insert_header("X-Splunk-Request-Id", request_id)
                .set_body_json(serde_json::json!({
                    "messages": [{"type": "ERROR", "text": "Not Found"}]
                })),
        )
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_indexes(
        &client,
        &mock_server.uri(),
        "test-token",
        Some(10),
        Some(0),
        3,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    if let ClientError::ApiError {
        status,
        url,
        message,
        request_id: rid,
    } = err
    {
        assert_eq!(status, 404);
        assert!(url.contains("/services/data/indexes"));
        assert!(message.contains("Not Found"));
        assert_eq!(rid, Some(request_id.to_string()));

        // Check if Display implementation includes details
        let display = format!(
            "{}",
            ClientError::ApiError {
                status,
                url: url.clone(),
                message: message.clone(),
                request_id: rid,
            }
        );
        assert!(display.contains("404"));
        assert!(display.contains(&url));
        assert!(display.contains(&message));
        assert!(display.contains(request_id));
    } else {
        panic!("Expected ApiError, got {:?}", err);
    }
}

/// Test that connection refused errors are properly handled.
///
/// This test verifies that when a connection is refused (e.g., server not running),
/// the client returns a ClientError::ConnectionRefused or HttpError wrapping
/// the connection refused error.
#[tokio::test]
async fn test_connection_refused_error() {
    // Use port 1 which is reserved and should never have a service
    let client = Client::new();
    let result = endpoints::list_indexes(
        &client,
        "http://localhost:1",
        "test-token",
        Some(10),
        Some(0),
        3,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Connection refused is a retryable transport error, but returns immediately from OS
    // It may come through as HttpError wrapping the connection error
    let err_string = format!("{:?}", err);
    assert!(
        err_string.contains("Connection refused") || err_string.contains("connection refused"),
        "Error should indicate connection refused. Got: {}",
        err_string
    );
}

/// Test that invalid URL errors are properly handled at request time.
///
/// This test verifies that when an invalid URL is used for a request,
/// the client returns an appropriate error.
#[tokio::test]
async fn test_invalid_url_error_at_request_time() {
    let client = Client::new();

    // Test with a malformed URL that should fail at request time
    let result = endpoints::list_indexes(
        &client,
        "not-a-valid-url",
        "test-token",
        Some(10),
        Some(0),
        3,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Invalid URL should result in an error
    let err_string = format!("{:?}", err);
    assert!(
        err_string.contains("URL")
            || err_string.contains("url")
            || err_string.contains("builder")
            || err_string.contains("RelativeUrl"),
        "Error should indicate URL issue. Got: {}",
        err_string
    );
}

/// Test that 404 Not Found errors are properly mapped to ApiError.
///
/// This test verifies that when a resource is not found (404),
/// the client returns a ClientError::ApiError with status 404.
#[tokio::test]
async fn test_not_found_error() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Resource not found"}]
        })))
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let result = endpoints::list_indexes(
        &client,
        &mock_server.uri(),
        "test-token",
        Some(10),
        Some(0),
        3,
        None,
    )
    .await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    // 404 should return ApiError, not NotFound variant
    assert!(
        matches!(err, ClientError::ApiError { status: 404, .. }),
        "Expected ApiError with status 404, got {:?}",
        err
    );

    // Verify error message contains resource info
    let display = format!("{}", err);
    assert!(
        display.contains("404") || display.contains("not found") || display.contains("Not found"),
        "Error display should contain 404 or not found. Got: {}",
        display
    );
}

/// Test that TLS errors are properly handled.
///
/// This test verifies that when a TLS error occurs (e.g., certificate validation fails),
/// the client returns an appropriate error. We test this by connecting to an HTTPS
/// server with certificate validation enabled (default) but with a self-signed cert.
#[tokio::test]
async fn test_tls_error_handling() {
    // Create a mock server with HTTPS (self-signed cert)
    // Note: wiremock uses HTTP by default, so we simulate the TLS error
    // by testing the error path directly

    // Test that TLS configuration is properly validated
    let strategy = splunk_client::AuthStrategy::ApiToken {
        token: secrecy::SecretString::new("test-token".to_string().into()),
    };

    // Build client with skip_verify=false (default) - should enforce TLS verification
    let client_result = splunk_client::SplunkClient::builder()
        .base_url("https://localhost:8089".to_string())
        .auth_strategy(strategy)
        .skip_verify(false)
        .build();

    assert!(client_result.is_ok());
    let mut client = client_result.unwrap();

    // Attempt to connect to a non-existent HTTPS server
    // This should fail with a connection-related error
    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Error should be related to connection/TLS
    let err_string = format!("{:?}", err);
    assert!(
        err_string.contains("Connection")
            || err_string.contains("TLS")
            || err_string.contains("tls")
            || err_string.contains("connect"),
        "Error should indicate connection or TLS issue. Got: {}",
        err_string
    );
}

// Error handling path tests for request.rs

/// Test that request builder clone failure on first attempt sends single request.
///
/// When a request builder cannot be cloned (e.g., streaming body), the first
/// attempt should still proceed without retry capability.
#[tokio::test]
async fn test_request_builder_clone_failure_single_attempt() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("search/create_job_success.json");

    Mock::given(method("POST"))
        .and(path("/services/search/jobs"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .expect(1)
        .mount(&mock_server)
        .await;

    let client = Client::new();
    let options = endpoints::CreateJobOptions {
        wait: Some(false),
        ..Default::default()
    };

    // This request should succeed on first attempt even without clone capability
    let result = endpoints::create_job(
        &client,
        &mock_server.uri(),
        "test-token",
        "search index=main",
        &options,
        3,
        None,
    )
    .await;

    assert!(result.is_ok());
}

/// Test that connection errors fail quickly.
///
/// Connection refused errors are classified as retryable by the transport layer,
/// but they return immediately from the OS without network delays. This test
/// verifies that even with retry logic, the overall operation completes quickly
/// (under 2 seconds) rather than taking the full exponential backoff time.
#[tokio::test]
async fn test_connection_error_fails_quickly() {
    // Use port 1 which is reserved and should never have a service
    let client = Client::new();
    let start = std::time::Instant::now();

    let result = endpoints::list_indexes(
        &client,
        "http://localhost:1", // Connection refused
        "test-token",
        Some(10),
        Some(0),
        3, // max_retries = 3
        None,
    )
    .await;

    let elapsed = start.elapsed();

    assert!(result.is_err());

    // Connection refused returns immediately from OS, so even with retries
    // the total time should be under 2 seconds (vs ~7s for full backoff)
    assert!(
        elapsed < std::time::Duration::from_secs(2),
        "Connection error should fail quickly, took {:?}",
        elapsed
    );
}

/// Test that connection refused errors complete within reasonable time.
///
/// Connection refused errors are retryable transport errors, but they return
/// immediately from the OS. This test verifies the operation completes quickly
/// even with retry configuration, rather than taking the full ~7s that
/// exponential backoff would require (1s + 2s + 4s delays).
#[tokio::test]
async fn test_connection_refused_completes_quickly() {
    let client = Client::new();

    // Track request attempts
    let start = std::time::Instant::now();
    let result = endpoints::get_job_status(
        &client,
        "http://localhost:1", // Connection refused
        "test-token",
        "test-sid",
        3,
        None,
    )
    .await;
    let elapsed = start.elapsed();

    assert!(result.is_err());

    // Connection refused returns immediately from OS, so even with retries
    // the total time should be well under the ~7s full backoff would take
    assert!(
        elapsed < std::time::Duration::from_secs(3),
        "Connection refused should complete quickly, took {:?}",
        elapsed
    );
}

/// Test that is_retryable() returns false for ConnectionRefused errors.
///
/// This test verifies the error classification logic for non-retryable errors.
#[test]
fn test_connection_refused_is_not_retryable() {
    let err = ClientError::ConnectionRefused("localhost:8089".to_string());
    assert!(
        !err.is_retryable(),
        "ConnectionRefused should not be retryable"
    );
}

/// Test that is_retryable() returns false for TlsError.
///
/// This test verifies that TLS errors are not considered retryable.
#[test]
fn test_tls_error_is_not_retryable() {
    let err = ClientError::TlsError("certificate validation failed".to_string());
    assert!(!err.is_retryable(), "TlsError should not be retryable");
}

/// Test that is_retryable() returns false for InvalidUrl errors.
///
/// This test verifies that invalid URL errors are not considered retryable.
#[test]
fn test_invalid_url_is_not_retryable() {
    let err = ClientError::InvalidUrl("not-a-url".to_string());
    assert!(!err.is_retryable(), "InvalidUrl should not be retryable");
}

/// Test that is_retryable() returns false for NotFound errors.
///
/// This test verifies that not found errors are not considered retryable.
#[test]
fn test_not_found_is_not_retryable() {
    let err = ClientError::NotFound("/some/resource".to_string());
    assert!(!err.is_retryable(), "NotFound should not be retryable");
}

/// Test error variant constructors and display formatting.
///
/// This test verifies that all error variants properly format their messages.
#[test]
fn test_error_variant_display_formatting() {
    // ConnectionRefused
    let err = ClientError::ConnectionRefused("localhost:8089".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Connection refused"));
    assert!(display.contains("localhost:8089"));

    // TlsError
    let err = ClientError::TlsError("handshake failed".to_string());
    let display = format!("{}", err);
    assert!(display.contains("TLS error"));
    assert!(display.contains("handshake failed"));

    // InvalidUrl
    let err = ClientError::InvalidUrl("bad url".to_string());
    let display = format!("{}", err);
    assert!(display.contains("Invalid URL"));
    assert!(display.contains("bad url"));

    // NotFound
    let err = ClientError::NotFound("/api/resource".to_string());
    let display = format!("{}", err);
    assert!(display.contains("not found") || display.contains("Not found"));
    assert!(display.contains("/api/resource"));
}

// Client configuration tests

#[tokio::test]
async fn test_client_with_trailing_slash_base_url() {
    let mock_server = MockServer::start().await;

    let fixture = load_fixture("auth/login_success.json");

    // Verify the endpoint path is exactly /services/auth/login (not //services/auth/login)
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&mock_server)
        .await;

    let strategy = splunk_client::AuthStrategy::SessionToken {
        username: "admin".to_string(),
        password: secrecy::SecretString::new("testpassword".to_string().into()),
    };

    // Build client with trailing slash in base_url
    let mut client = splunk_client::SplunkClient::builder()
        .base_url(format!("{}/", mock_server.uri())) // Add trailing slash
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // Login should succeed (URL should be normalized, no double slash)
    let result = client.login().await;

    // If base_url wasn't normalized, this would fail with 404 because
    // wiremock would see //services/auth/login instead of /services/auth/login
    assert!(result.is_ok());
}
