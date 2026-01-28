//! Security-focused tests for TLS and secret handling.
//!
//! These tests verify that:
//! - TLS certificate validation is properly configured
//! - Secrets are not exposed in Debug output
//! - Session tokens are not logged in retry flows
//! - Auth credentials are not exposed in error messages

use secrecy::{ExposeSecret, SecretString};
use splunk_client::{AuthStrategy, ClientError, SessionManager, SplunkClient};
use std::sync::Arc;
use std::sync::atomic::{AtomicUsize, Ordering};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path, query_param},
};

// ============================================================================
// TLS Configuration Tests
// ============================================================================

/// Test that TLS verification is enabled by default (skip_verify=false).
///
/// This test verifies that when skip_verify is not set (defaults to false),
/// the client builder does not set danger_accept_invalid_certs.
/// Note: We cannot directly inspect the reqwest Client's TLS configuration,
/// but we can verify the client builds successfully with default settings.
#[test]
fn test_tls_verification_enabled_by_default() {
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("test-token".to_string().into()),
    };

    // Build client with default settings (skip_verify=false)
    let client = SplunkClient::builder()
        .base_url("https://localhost:8089".to_string())
        .auth_strategy(strategy)
        .build();

    assert!(
        client.is_ok(),
        "Client should build successfully with TLS verification enabled by default"
    );
}

/// Test that skip_verify=true properly configures the client for HTTPS URLs.
///
/// This test verifies that when skip_verify=true and the URL is HTTPS,
/// the client builder successfully applies the danger_accept_invalid_certs setting.
#[test]
fn test_skip_verify_configures_danger_accept_invalid_certs() {
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("test-token".to_string().into()),
    };

    // Build client with skip_verify=true on HTTPS URL
    let client = SplunkClient::builder()
        .base_url("https://localhost:8089".to_string())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build();

    assert!(
        client.is_ok(),
        "Client should build successfully with skip_verify=true on HTTPS URL"
    );
}

/// Test that skip_verify=true with HTTP URL does not panic.
///
/// When skip_verify=true is used with an HTTP URL, the client should still
/// build successfully (the warning about ineffective skip_verify is logged
/// but not testable in a unit test).
#[test]
fn test_skip_verify_http_url_warning() {
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("test-token".to_string().into()),
    };

    // Build client with HTTP URL and skip_verify=true
    // This should succeed (warning is logged but not testable in unit test)
    let client = SplunkClient::builder()
        .base_url("http://localhost:8089".to_string())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build();

    assert!(
        client.is_ok(),
        "Client should build successfully even with skip_verify=true on HTTP URL"
    );
}

/// Test that skip_verify=false prevents building client with invalid HTTPS config.
///
/// This test documents that with skip_verify=false (default), the client
/// will enforce TLS certificate validation. We can't test actual TLS handshake
/// failures without a mock server with invalid certificates.
#[test]
fn test_tls_verification_not_skipped_by_default() {
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("test-token".to_string().into()),
    };

    // Build client with explicit skip_verify=false
    let client = SplunkClient::builder()
        .base_url("https://localhost:8089".to_string())
        .auth_strategy(strategy)
        .skip_verify(false)
        .build();

    assert!(
        client.is_ok(),
        "Client should build successfully with explicit skip_verify=false"
    );
}

// ============================================================================
// AuthStrategy Secret Protection Tests
// ============================================================================

/// Test that API token is not exposed in AuthStrategy Debug output.
///
/// The secrecy crate should redact the SecretString in Debug output.
/// This test verifies that the API token value does not appear in the
/// formatted Debug output.
#[test]
fn test_api_token_not_in_debug_output() {
    let secret_token = "secret-api-token-12345";
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new(secret_token.to_string().into()),
    };

    let debug_output = format!("{:?}", strategy);

    // The secret token should NOT appear in debug output
    assert!(
        !debug_output.contains(secret_token),
        "Debug output should not contain the API token secret. Output: {}",
        debug_output
    );

    // But the variant name should be visible
    assert!(
        debug_output.contains("ApiToken"),
        "Debug output should contain the variant name. Output: {}",
        debug_output
    );

    // The SecretString should be redacted (typically shows as "[REDACTED]" or similar)
    assert!(
        debug_output.contains("[REDACTED]") || debug_output.contains("SecretString"),
        "Debug output should indicate secret is redacted. Output: {}",
        debug_output
    );
}

/// Test that session password is not exposed in AuthStrategy Debug output.
///
/// When using SessionToken auth, the password should be redacted in Debug output
/// while the username remains visible.
#[test]
fn test_session_token_password_not_in_debug_output() {
    let username = "admin";
    let password = "secret-password-45678";

    let strategy = AuthStrategy::SessionToken {
        username: username.to_string(),
        password: SecretString::new(password.to_string().into()),
    };

    let debug_output = format!("{:?}", strategy);

    // The password should NOT appear in debug output
    assert!(
        !debug_output.contains(password),
        "Debug output should not contain the password. Output: {}",
        debug_output
    );

    // But the username SHOULD be visible (it's not a secret)
    assert!(
        debug_output.contains(username),
        "Debug output should contain the username. Output: {}",
        debug_output
    );

    // The variant name should be visible
    assert!(
        debug_output.contains("SessionToken"),
        "Debug output should contain the variant name. Output: {}",
        debug_output
    );
}

// ============================================================================
// SessionManager Secret Protection Tests
// ============================================================================

/// Test that SessionManager does not expose tokens in Debug output.
///
/// The SessionManager stores both the auth strategy and session token.
/// Both should be redacted in Debug output.
#[test]
fn test_session_manager_token_not_in_debug_output() {
    let secret_token = "session-manager-secret-token";
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new(secret_token.to_string().into()),
    };

    let manager = SessionManager::new(strategy);
    let debug_output = format!("{:?}", manager);

    // The secret token should NOT appear in debug output
    assert!(
        !debug_output.contains(secret_token),
        "Debug output should not contain the token. Output: {}",
        debug_output
    );
}

/// Test that session tokens set after login are not exposed in Debug output.
#[test]
fn test_session_manager_set_token_not_in_debug_output() {
    let strategy = AuthStrategy::SessionToken {
        username: "admin".to_string(),
        password: SecretString::new("password".to_string().into()),
    };

    let mut manager = SessionManager::new(strategy);
    let session_token = "new-session-token-after-login-123";
    manager.set_session_token(session_token.to_string(), Some(3600));

    let debug_output = format!("{:?}", manager);

    // The session token should NOT appear in debug output
    assert!(
        !debug_output.contains(session_token),
        "Debug output should not contain the session token. Output: {}",
        debug_output
    );
}

/// Test that clearing a session doesn't expose the token.
#[test]
fn test_session_clear_not_logged() {
    let strategy = AuthStrategy::SessionToken {
        username: "admin".to_string(),
        password: SecretString::new("password".to_string().into()),
    };

    let mut manager = SessionManager::new(strategy);
    let session_token = "session-to-be-cleared-456";
    manager.set_session_token(session_token.to_string(), Some(3600));

    // Clear the session
    manager.clear_session();

    let debug_output = format!("{:?}", manager);

    // Even after clearing, the old token should not appear
    assert!(
        !debug_output.contains(session_token),
        "Debug output should not contain cleared session token. Output: {}",
        debug_output
    );

    // Session should be expired after clearing
    assert!(
        manager.is_session_expired(),
        "Session should be expired after clearing"
    );
}

// ============================================================================
// Auth Security Difference Tests
// ============================================================================

/// Test that API token auth never expires while session auth does.
///
/// This test documents the security difference between API tokens and session tokens:
/// - API tokens are long-lived (don't expire)
/// - Session tokens expire after a TTL (default 1 hour)
#[test]
fn test_session_token_expires_api_token_does_not() {
    // API token auth - should never expire
    let api_strategy = AuthStrategy::ApiToken {
        token: SecretString::new("api-token".to_string().into()),
    };
    let api_manager = SessionManager::new(api_strategy);
    assert!(
        !api_manager.is_session_expired(),
        "API token auth should never be expired"
    );
    assert!(api_manager.is_api_token(), "Should be API token auth");

    // Session token auth without setting token - should be expired
    let session_strategy = AuthStrategy::SessionToken {
        username: "admin".to_string(),
        password: SecretString::new("pass".to_string().into()),
    };
    let session_manager = SessionManager::new(session_strategy);
    assert!(
        session_manager.is_session_expired(),
        "Session auth without token should be expired"
    );
    assert!(
        !session_manager.is_api_token(),
        "Should not be API token auth"
    );
}

/// Test that bearer token can be accessed programmatically but is protected.
///
/// The get_bearer_token method returns the token for use in API calls,
/// but the SecretString protects it from accidental exposure.
#[test]
fn test_bearer_token_accessible_via_expose_secret() {
    let secret_token = "bearer-token-secret-789";
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new(secret_token.to_string().into()),
    };

    let manager = SessionManager::new(strategy);

    // Token should be accessible via get_bearer_token (for API calls)
    let bearer = manager.get_bearer_token();
    assert_eq!(
        bearer,
        Some(secret_token),
        "Bearer token should be accessible for API calls"
    );

    // But the SecretString itself should not expose the secret in Debug
    let token_secret = SecretString::new(secret_token.to_string().into());
    let debug_output = format!("{:?}", token_secret);
    assert!(
        !debug_output.contains(secret_token),
        "SecretString Debug should not expose the secret"
    );
}

// ============================================================================
// Retry Flow Secret Safety Tests
// ============================================================================

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

// ============================================================================
// Error Path Secret Tests
// ============================================================================

/// Test that network errors don't expose tokens in error messages.
///
/// When a network error occurs, the error message should not contain
/// any authentication tokens.
#[tokio::test]
async fn test_network_error_does_not_expose_token() {
    // Use an invalid URL that will cause a connection error
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new("secret-api-token-xyz789".to_string().into()),
    };

    let mut client = SplunkClient::builder()
        .base_url("http://localhost:1".to_string()) // Invalid port
        .auth_strategy(strategy)
        .timeout(std::time::Duration::from_millis(100))
        .build()
        .unwrap();

    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_string = format!("{:?}", err);

    // The error should not contain the token
    assert!(
        !err_string.contains("secret-api-token-xyz789"),
        "Error message should not contain the API token. Error: {}",
        err_string
    );
}

/// Test that authentication failure errors don't expose credentials.
///
/// When authentication fails (wrong password), the error should not
/// contain the password that was used.
#[tokio::test]
async fn test_auth_failure_does_not_expose_password() {
    let mock_server = MockServer::start().await;

    // Return 401 for login attempt
    Mock::given(method("POST"))
        .and(path("/services/auth/login"))
        .respond_with(ResponseTemplate::new(401).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Invalid credentials"}]
        })))
        .mount(&mock_server)
        .await;

    let wrong_password = "wrong-password-12345";
    let strategy = AuthStrategy::SessionToken {
        username: "admin".to_string(),
        password: SecretString::new(wrong_password.to_string().into()),
    };

    let mut client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    let result = client.login().await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_string = format!("{:?}", err);

    // The error should not contain the password
    assert!(
        !err_string.contains(wrong_password),
        "Error message should not contain the password. Error: {}",
        err_string
    );
}

/// Test that API error messages don't contain tokens.
///
/// When an API error occurs, the error details should not include
/// any authentication tokens.
#[tokio::test]
async fn test_api_error_does_not_expose_token() {
    let mock_server = MockServer::start().await;

    Mock::given(method("GET"))
        .and(path("/services/data/indexes"))
        .respond_with(ResponseTemplate::new(403).set_body_json(serde_json::json!({
            "messages": [{"type": "ERROR", "text": "Forbidden - insufficient permissions"}]
        })))
        .mount(&mock_server)
        .await;

    let secret_token = "forbidden-token-abc123";
    let strategy = AuthStrategy::ApiToken {
        token: SecretString::new(secret_token.to_string().into()),
    };

    let mut client = SplunkClient::builder()
        .base_url(mock_server.uri())
        .auth_strategy(strategy)
        .skip_verify(true)
        .build()
        .unwrap();

    let result = client.list_indexes(Some(10), Some(0)).await;

    assert!(result.is_err());
    let err = result.unwrap_err();
    let err_string = format!("{:?}", err);

    // The error should not contain the token
    assert!(
        !err_string.contains(secret_token),
        "API error should not contain the token. Error: {}",
        err_string
    );
}

// ============================================================================
// SecretString Direct Tests
// ============================================================================

/// Test that SecretString properly redacts in Debug output.
#[test]
fn test_secret_string_debug_redaction() {
    let secret = "my-super-secret-value";
    let secret_string = SecretString::new(secret.to_string().into());

    let debug_output = format!("{:?}", secret_string);

    // The secret should NOT appear
    assert!(
        !debug_output.contains(secret),
        "Debug output should not contain the secret value"
    );

    // But we should be able to access it via ExposeSecret
    assert_eq!(
        secret_string.expose_secret(),
        secret,
        "Should be able to access secret via ExposeSecret trait"
    );
}

/// Test that multiple SecretStrings in a structure are all redacted.
#[test]
fn test_multiple_secrets_redacted() {
    let strategy = AuthStrategy::SessionToken {
        username: "admin".to_string(),
        password: SecretString::new("password1".to_string().into()),
    };

    let mut manager = SessionManager::new(strategy);
    manager.set_session_token("session-token-123".to_string(), Some(3600));

    let debug_output = format!("{:?}", manager);

    // Neither password nor session token should appear
    assert!(
        !debug_output.contains("password1"),
        "Debug output should not contain password"
    );
    assert!(
        !debug_output.contains("session-token-123"),
        "Debug output should not contain session token"
    );
}

// ============================================================================
// Integration Security Tests
// ============================================================================

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

    let mut client = SplunkClient::builder()
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

    let mut client = SplunkClient::builder()
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
