//! Error path secret protection tests.
//!
//! This module verifies that authentication tokens, passwords, and session
//! credentials are not exposed in error messages when network, authentication,
//! or API errors occur.
//!
//! What this module does NOT handle:
//! - Error classification or categorization
//! - Error retry logic
//! - Error logging (only that secrets don't appear in error representations)

use secrecy::SecretString;
use splunk_client::{AuthStrategy, SplunkClient};
use wiremock::{
    Mock, MockServer, ResponseTemplate,
    matchers::{method, path},
};

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

    let client = SplunkClient::builder()
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

    let client = SplunkClient::builder()
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

    let client = SplunkClient::builder()
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
