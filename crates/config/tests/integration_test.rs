//! Integration tests for configuration loading across CLI and TUI entrypoints.
//!
//! These tests verify end-to-end config loading behavior, ensuring that
//! the ConfigLoader builder chain works correctly from both frontends.

use splunk_config::{
    AuthStrategy, ConfigError, ConfigLoader, InternalLogsDefaults, SearchDefaultConfig,
    env_var_or_none,
};
use std::path::PathBuf;

/// Test that ConfigLoader respects the full precedence chain:
/// CLI args > env vars > profile config > defaults
#[test]
fn test_config_loader_cli_overrides() {
    // This test verifies that values set via builder methods
    // (simulating CLI args) take precedence over defaults
    let loader = ConfigLoader::new()
        .with_base_url("https://cli-override.com:8089".to_string())
        .with_api_token("cli-token".to_string());

    let config = loader.build().expect("should build with CLI overrides");
    assert_eq!(config.connection.base_url, "https://cli-override.com:8089");
    assert!(matches!(
        config.auth.strategy,
        AuthStrategy::ApiToken { .. }
    ));
}

/// Test that env_var_or_none is exported and works correctly
#[test]
fn test_env_var_or_none_exported() {
    // env_var_or_none should be available from splunk_config root
    let _result: Option<String> = env_var_or_none("SPLUNK_BASE_URL");
    // We don't care about the value, just that it's callable
}

/// Test building search defaults with no persisted defaults
#[test]
fn test_build_search_defaults_integration() {
    let loader = ConfigLoader::new();

    // Build with no persisted defaults (uses internal defaults)
    let defaults: SearchDefaultConfig = loader.build_search_defaults(None);

    // Verify we get reasonable defaults
    assert!(!defaults.earliest_time.is_empty());
    assert!(!defaults.latest_time.is_empty());
    assert!(defaults.max_results > 0);
}

/// Test building internal logs defaults with no persisted defaults
#[test]
fn test_build_internal_logs_defaults_integration() {
    let loader = ConfigLoader::new();

    // Build with no persisted defaults (uses internal defaults)
    let defaults: InternalLogsDefaults = loader.build_internal_logs_defaults(None);

    // Verify we get reasonable defaults
    assert!(defaults.count > 0);
    assert!(!defaults.earliest_time.is_empty());
}

/// Test that ConfigLoader can be used with custom config path
#[test]
fn test_config_loader_with_custom_path() {
    let loader = ConfigLoader::new().with_config_path(PathBuf::from("/tmp/test-config.json"));

    // Just verify it doesn't panic - actual profile loading is tested in unit tests
    let _ = loader;
}

/// Test that API token auth takes precedence over session credentials
#[test]
fn test_api_token_precedence_integration() {
    let loader = ConfigLoader::new()
        .with_base_url("https://example.com:8089".to_string())
        .with_username("admin".to_string())
        .with_password("password".to_string())
        .with_api_token("api-token".to_string());

    let config = loader.build().expect("should build with API token");
    assert!(matches!(
        config.auth.strategy,
        AuthStrategy::ApiToken { .. }
    ));
}

/// Test that session auth works when only username/password provided
#[test]
fn test_session_auth_integration() {
    let loader = ConfigLoader::new()
        .with_base_url("https://example.com:8089".to_string())
        .with_username("admin".to_string())
        .with_password("password".to_string());

    let config = loader.build().expect("should build with session auth");
    assert!(matches!(
        config.auth.strategy,
        AuthStrategy::SessionToken { .. }
    ));
}

/// Test that missing base URL returns correct error
#[test]
fn test_missing_base_url_error() {
    let loader = ConfigLoader::new().with_api_token("token".to_string());

    let result = loader.build();
    assert!(matches!(result, Err(ConfigError::MissingBaseUrl)));
}

/// Test that missing auth returns correct error
#[test]
fn test_missing_auth_error() {
    let loader = ConfigLoader::new().with_base_url("https://example.com:8089".to_string());

    let result = loader.build();
    assert!(matches!(result, Err(ConfigError::MissingAuth)));
}

/// Test that ConfigLoader::new() creates a valid loader that can be built
/// (after providing required fields)
#[test]
fn test_config_loader_new() {
    let loader = ConfigLoader::new();

    // Should be able to set required fields and build
    let result = loader
        .with_base_url("https://example.com:8089".to_string())
        .with_api_token("token".to_string())
        .build();

    assert!(result.is_ok());
}

/// Test that SearchDefaultConfig implements Clone
#[test]
fn test_search_defaults_implements_clone() {
    let loader = ConfigLoader::new();
    let defaults = loader.build_search_defaults(None);

    // This should compile if SearchDefaultConfig implements Clone
    let _cloned = defaults.clone();
}

/// Test that InternalLogsDefaults implements Clone
#[test]
fn test_internal_logs_defaults_implements_clone() {
    let loader = ConfigLoader::new();
    let defaults = loader.build_internal_logs_defaults(None);

    // This should compile if InternalLogsDefaults implements Clone
    let _cloned = defaults.clone();
}
