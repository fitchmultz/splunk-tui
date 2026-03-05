//! Profile loading tests for the configuration loader builder.
//!
//! Responsibilities:
//! - Test loading configuration from profile files.
//! - Test error handling for missing profiles.
//! - Test profile override behavior with builder methods.

use crate::loader::builder::ConfigLoader;
use crate::loader::error::ConfigError;
use crate::types::AuthStrategy;
use secrecy::ExposeSecret;
use secrecy::SecretString;
use std::io::Write;
use std::path::PathBuf;
use tempfile::TempDir;

/// Creates a test configuration file with predefined profiles.
pub fn create_test_config_file(dir: &std::path::Path) -> PathBuf {
    let config_path = dir.join("config.json");
    let password = SecretString::new("test-password".to_string().into());

    let config = serde_json::json!({
        "profiles": {
            "dev": {
                "base_url": "https://dev.splunk.com:8089",
                "username": "dev_user",
                "password": password.expose_secret(),
                "skip_verify": true,
                "timeout_seconds": 60,
                "max_retries": 5
            },
            "prod": {
                "base_url": "https://prod.splunk.com:8089",
                "api_token": "prod-token-123"
            }
        },
        "state": {
            "auto_refresh": true,
            "sort_column": "sid",
            "sort_direction": "asc"
        }
    });

    let mut file = std::fs::File::create(&config_path).unwrap();
    writeln!(file, "{}", config).unwrap();

    config_path
}

#[test]
fn test_loader_from_profile_file() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    let loader = ConfigLoader::new()
        .with_profile_name("dev".to_string())
        .with_config_path(config_path)
        .from_profile()
        .unwrap();

    let config = loader.build().unwrap();
    assert_eq!(config.connection.base_url, "https://dev.splunk.com:8089");
    assert!(config.connection.skip_verify);
    assert_eq!(
        config.connection.timeout,
        std::time::Duration::from_secs(60)
    );
    assert_eq!(config.connection.max_retries, 5);
}

#[test]
fn test_profile_missing_errors_without_overrides() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    let loader = ConfigLoader::new()
        .with_profile_name("nonexistent".to_string())
        .with_config_path(config_path)
        .from_profile()
        .unwrap();

    let result = loader.build();
    assert!(matches!(result, Err(ConfigError::ProfileNotFound(_))));
}

#[test]
fn test_profile_missing_with_only_username_override() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    // Profile missing, only username provided - should get ProfileNotFound
    let loader = ConfigLoader::new()
        .with_profile_name("nonexistent".to_string())
        .with_config_path(config_path)
        .from_profile()
        .unwrap()
        .with_username("admin".to_string());

    let result = loader.build();
    assert!(
        matches!(result, Err(ConfigError::ProfileNotFound(_))),
        "Expected ProfileNotFound when only username is provided, got {:?}",
        result
    );
}

#[test]
fn test_profile_missing_with_only_password_override() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    // Profile missing, only password provided - should get ProfileNotFound
    let loader = ConfigLoader::new()
        .with_profile_name("nonexistent".to_string())
        .with_config_path(config_path)
        .from_profile()
        .unwrap()
        .with_password("secret".to_string());

    let result = loader.build();
    assert!(
        matches!(result, Err(ConfigError::ProfileNotFound(_))),
        "Expected ProfileNotFound when only password is provided, got {:?}",
        result
    );
}

#[test]
fn test_profile_missing_with_only_base_url_override() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    // Profile missing, only base_url provided - should get ProfileNotFound
    let loader = ConfigLoader::new()
        .with_profile_name("nonexistent".to_string())
        .with_config_path(config_path)
        .from_profile()
        .unwrap()
        .with_base_url("https://splunk.com:8089".to_string());

    let result = loader.build();
    assert!(
        matches!(result, Err(ConfigError::ProfileNotFound(_))),
        "Expected ProfileNotFound when only base_url is provided, got {:?}",
        result
    );
}

#[test]
fn test_profile_missing_with_partial_session_auth() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    // Profile missing, username+password provided but no base_url - should get ProfileNotFound
    let loader = ConfigLoader::new()
        .with_profile_name("nonexistent".to_string())
        .with_config_path(config_path)
        .from_profile()
        .unwrap()
        .with_username("admin".to_string())
        .with_password("secret".to_string());

    let result = loader.build();
    assert!(
        matches!(result, Err(ConfigError::ProfileNotFound(_))),
        "Expected ProfileNotFound when username+password provided but no base_url, got {:?}",
        result
    );
}

#[test]
fn test_profile_missing_with_partial_api_token_auth() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    // Profile missing, api_token provided but no base_url - should get ProfileNotFound
    let loader = ConfigLoader::new()
        .with_profile_name("nonexistent".to_string())
        .with_config_path(config_path)
        .from_profile()
        .unwrap()
        .with_api_token("my-token".to_string());

    let result = loader.build();
    assert!(
        matches!(result, Err(ConfigError::ProfileNotFound(_))),
        "Expected ProfileNotFound when api_token provided but no base_url, got {:?}",
        result
    );
}

#[test]
fn test_profile_missing_with_complete_config_override() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    // Profile missing, but complete config provided via builder - should succeed
    let loader = ConfigLoader::new()
        .with_profile_name("nonexistent".to_string())
        .with_config_path(config_path)
        .from_profile()
        .unwrap()
        .with_base_url("https://splunk.com:8089".to_string())
        .with_api_token("my-token".to_string());

    let result = loader.build();
    assert!(
        result.is_ok(),
        "Expected success when complete config is provided, got {:?}",
        result
    );
    assert_eq!(
        result.unwrap().connection.base_url,
        "https://splunk.com:8089"
    );
}

#[test]
fn test_profile_missing_with_complete_session_auth_override() {
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    // Profile missing, but complete session auth config provided - should succeed
    let loader = ConfigLoader::new()
        .with_profile_name("nonexistent".to_string())
        .with_config_path(config_path)
        .from_profile()
        .unwrap()
        .with_base_url("https://splunk.com:8089".to_string())
        .with_username("admin".to_string())
        .with_password("secret".to_string());

    let result = loader.build();
    assert!(
        result.is_ok(),
        "Expected success when complete session auth config is provided, got {:?}",
        result
    );
    assert!(matches!(
        result.unwrap().auth.strategy,
        AuthStrategy::SessionToken { .. }
    ));
}
