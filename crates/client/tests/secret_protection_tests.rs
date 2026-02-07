//! Secret protection tests for AuthStrategy, SessionManager, and SecretString.
//!
//! This module verifies that sensitive credentials are properly protected from
//! accidental exposure through Debug output, logging, or error messages.
//!
//! What this module does NOT handle:
//! - Network-level secret transmission security (TLS handles this)
//! - Secret storage at rest (handled by OS keychain or environment)
//! - Memory-dumping attack resistance

use secrecy::{ExposeSecret, SecretString};
use splunk_client::{AuthStrategy, SessionManager};

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
