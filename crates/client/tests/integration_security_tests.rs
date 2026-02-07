//! Integration security tests for comprehensive secret protection verification.
//!
//! This module contains end-to-end security tests that verify secrets are not
//! exposed across various error scenarios, session rotations, and connection
//! failures.
//!
//! What this module does NOT handle:
//! - Performance testing under load
//! - Fuzzing or penetration testing
//! - Memory forensics analysis

use secrecy::SecretString;
use splunk_client::{AuthStrategy, SplunkClient};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

/// Test that client operations don't expose secrets in error chains.
///
/// This test verifies that when multiple errors occur in a chain,
/// no secrets are exposed at any level.
#[tokio::test]
async fn test_error_chain_no_secret_exposure() {
    let mock_server = MockServer::start().await;

    // Return various error status codes
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(500).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Internal server error"}]
        })))
        .mount(&mock_server)
        .await;

    let secret_token = "chain-test-token-secret";
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new(secret_token.to_string().into()),
    };

    let client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Check various error representations
    let debug_str = format!("{:?}", err);
    let display_str = format!("{}", err);

    assert!(
        !debug_str.contains(secret_token),
        "Debug error should not contain token"
    );
    assert!(
        !display_str.contains(secret_token),
        "Display error should not contain token"
    );
}

/// Test that session token rotation doesn't expose old tokens.
///
/// When a session is refreshed (token rotation), the old token should
/// not be exposed in any way.
#[tokio::test]
async fn test_session_rotation_no_token_exposure() {
    let mock_server = MockServer::start().await;

    let first_session_key = "first-session-key-111";
    let second_session_key = "second-session-key-222";

    let first_login = serde_json::json!({
        "sessionKey": first_session_key
    });
    let second_login = serde_json::json!({
        "sessionKey": second_session_key
    });

    let login_count = Arc::new(AtomicUsize::new(0));
    let login_count_clone = login_count.clone();

    // Return different session keys on each login
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .and(query_param("output_mode", "json"))
        .respond_with(move |_: &wiremock::Request| {
            let count = login_count_clone.fetch_add(1, Ordering::SeqCst);
            if count == 0 {
                ResponseTemplate::new(200).set_body_json(&first_login)
            } else {
                ResponseTemplate::new(200).set_body_json(&second_login)
            }
        })
        .mount(&mock_server)
        .await;

    // First call returns 401, second returns success
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

    let client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // First login - gets first_session_key
    client.login().await.unwrap();

    // This triggers a retry - gets second_session_key
    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_ok());

    // The client debug output should not contain either session key
    let client_debug = format!("{:?}", client);
    assert!(
        !client_debug.contains(first_session_key),
        "Client debug should not contain first session key"
    );
    assert!(
        !client_debug.contains(second_session_key),
        "Client debug should not contain second session key"
    );
}

/// Test that ConnectionRefused errors don't expose secrets.
///
/// When a connection is refused, the error message should not contain
/// any authentication tokens or credentials.
#[tokio::test]
async fn test_connection_refused_no_secret_exposure() {
    let secret_token = "secret-token-connection-test-123";
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new(secret_token.to_string().into()),
    };

    let client = SplunkClient::builder()
        .base_url("http://localhost:1".to_string()) // Connection refused
        .auth_strategy(strategy)
        .timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();

    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Check both Debug and Display representations
    let debug_str = format!("{:?}", err);
    let display_str = format!("{}", err);

    assert!(
        !debug_str.contains(secret_token),
        "Debug error should not contain API token. Error: {}",
        debug_str
    );
    assert!(
        !display_str.contains(secret_token),
        "Display error should not contain API token. Error: {}",
        display_str
    );
}

/// Test that TLS errors don't expose secrets.
///
/// When a TLS error occurs, the error message should not contain
/// any authentication tokens or credentials.
#[tokio::test]
async fn test_tls_error_no_secret_exposure() {
    let secret_token = "secret-token-tls-test-456";
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new(secret_token.to_string().into()),
    };

    // Build client with TLS verification enabled
    let client = SplunkClient::builder()
        .base_url("https://localhost:1".to_string()) // Will fail TLS/connect
        .auth_strategy(strategy)
        .skip_verify(false)
        .timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();

    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Check both Debug and Display representations
    let debug_str = format!("{:?}", err);
    let display_str = format!("{}", err);

    assert!(
        !debug_str.contains(secret_token),
        "Debug error should not contain API token. Error: {}",
        debug_str
    );
    assert!(
        !display_str.contains(secret_token),
        "Display error should not contain API token. Error: {}",
        display_str
    );
}

/// Test that NotFound errors don't expose secrets.
///
/// When a resource is not found (404), the error message should not
/// contain any authentication tokens.
#[tokio::test]
async fn test_not_found_no_secret_exposure() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Not Found"}]
        })))
        .mount(&mock_server)
        .await;

    let secret_token = "secret-token-notfound-test-789";
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new(secret_token.to_string().into()),
    };

    let client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Check both Debug and Display representations
    let debug_str = format!("{:?}", err);
    let display_str = format!("{}", err);

    assert!(
        !debug_str.contains(secret_token),
        "Debug error should not contain API token. Error: {}",
        debug_str
    );
    assert!(
        !display_str.contains(secret_token),
        "Display error should not contain API token. Error: {}",
        display_str
    );
}

/// Test that InvalidUrl errors don't expose secrets.
///
/// When an invalid URL is used (e.g., missing base_url), the error message
/// should not contain any authentication tokens.
#[test]
fn test_invalid_url_no_secret_exposure() {
    let secret_token = "secret-token-invalidurl-test-abc";
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new(secret_token.to_string().into()),
    };

    // Attempt to build client without base_url (triggers InvalidUrl error)
    let result = SplunkClient::builder().auth_strategy(strategy).build();

    assert!(result.is_err());
    let err = result.unwrap_err();

    // Verify it's an InvalidUrl error
    let err_string = format!("{:?}", err);
    assert!(
        err_string.contains("InvalidUrl") || err_string.contains("base_url"),
        "Expected InvalidUrl error, got: {}",
        err_string
    );

    // Check both Debug and Display representations
    let debug_str = format!("{:?}", err);
    let display_str = format!("{}", err);

    assert!(
        !debug_str.contains(secret_token),
        "Debug error should not contain API token. Error: {}",
        debug_str
    );
    assert!(
        !display_str.contains(secret_token),
        "Display error should not contain API token. Error: {}",
        display_str
    );
}

/// Test that all error variants properly protect secrets in Debug output.
///
/// This test creates various error scenarios and verifies that secrets
/// are not exposed in any error representation.
#[tokio::test]
async fn test_all_error_paths_no_secret_exposure() {
    let mock_server = MockServer::start().await;

    // Mock various error responses
    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(404).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Not found"}]
        })))
        .mount(&mock_server)
        .await;

    let secret_token = "comprehensive-test-token-xyz789";
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new(secret_token.to_string().into()),
    };

    let client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    // Generate various errors
    let result = client.list_indexes(Some(10), Some(0)).await;
    assert!(result.is_err());

    let err = result.unwrap_err();

    // Comprehensive check - neither Debug nor Display should contain the token
    let representations = [format!("{:?}", err), format!("{}", err)];

    for repr in &representations {
        assert!(
            !repr.contains(secret_token),
            "Error representation should not contain secret token: {}",
            repr
        );
    }
}
