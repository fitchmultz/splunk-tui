//! Basic loader tests for the configuration loader builder.
//!
//! Responsibilities:
//! - Test basic builder configuration with API token and session token auth.
//! - Test validation errors for missing base URL and auth.
//! - Test API token precedence over session credentials.

use crate::loader::builder::ConfigLoader;
use crate::loader::error::ConfigError;
use crate::types::AuthStrategy;

#[test]
fn test_loader_with_api_token() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string());

    let config = loader.build().unwrap();
    assert!(matches!(
        config.auth.strategy,
        AuthStrategy::ApiToken { .. }
    ));
}

#[test]
fn test_loader_with_session_token() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_username("admin".to_string())
        .with_password("password".to_string());

    let config = loader.build().unwrap();
    assert!(matches!(
        config.auth.strategy,
        AuthStrategy::SessionToken { .. }
    ));
}

#[test]
fn test_loader_missing_base_url() {
    let loader = ConfigLoader::new().with_api_token("test-token".to_string());
    let result = loader.build();
    assert!(matches!(result, Err(ConfigError::MissingBaseUrl)));
}

#[test]
fn test_loader_missing_auth() {
    let loader = ConfigLoader::new().with_base_url("https://localhost:8089".to_string());
    let result = loader.build();
    assert!(matches!(result, Err(ConfigError::MissingAuth)));
}

#[test]
fn test_api_token_takes_precedence() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_username("admin".to_string())
        .with_password("password".to_string())
        .with_api_token("api-token".to_string());

    let config = loader.build().unwrap();
    assert!(matches!(
        config.auth.strategy,
        AuthStrategy::ApiToken { .. }
    ));
}
