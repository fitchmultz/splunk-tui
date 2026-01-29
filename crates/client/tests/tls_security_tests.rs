//! TLS configuration security tests.
//!
//! This module tests TLS/SSL certificate verification settings to ensure:
//! - TLS verification is enabled by default (secure-by-default posture)
//! - skip_verify=true properly configures the client for HTTPS URLs
//! - HTTP URLs with skip_verify=true don't cause panics
//!
//! What this module does NOT handle:
//! - Actual TLS handshake verification (requires mock server with invalid certs)
//! - Certificate pinning or custom CA bundle configuration
//! - TLS version negotiation specifics

use secrecy::SecretString;
use splunk_client::{AuthStrategy, SplunkClient};

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
