//! Tests for the configuration loader builder.
//!
//! Responsibilities:
//! - Test builder methods for configuration loading.
//! - Test profile loading from files.
//! - Test environment variable handling and precedence.
//! - Test search defaults, session TTL, and buffer settings.
//!
//! Does NOT handle:
//! - Direct environment variable parsing logic (tested in env.rs).
//! - Profile file loading logic (tested in profile.rs).
//! - Persisting configuration changes (tested in persistence.rs).
//!
//! Invariants / Assumptions:
//! - Tests use `serial_test` to prevent environment variable pollution.
//! - Tests use `global_test_lock()` for additional synchronization.
//! - Temporary directories are cleaned up automatically via `tempfile`.

use crate::loader::builder::ConfigLoader;
use crate::loader::env::env_var_or_none;
use crate::loader::error::ConfigError;
use crate::persistence::SearchDefaults;
use crate::types::AuthStrategy;
use secrecy::ExposeSecret;
use secrecy::SecretString;
use serial_test::serial;
use std::io::Write;
use std::path::PathBuf;
use std::sync::Mutex;
use std::time::Duration;
use tempfile::TempDir;

fn create_test_config_file(dir: &std::path::Path) -> PathBuf {
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

fn env_lock() -> &'static Mutex<()> {
    crate::test_util::global_test_lock()
}

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
    assert_eq!(config.connection.timeout, Duration::from_secs(60));
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

#[test]
#[serial]
fn test_env_overrides_profile() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    temp_env::with_vars(
        [("SPLUNK_BASE_URL", Some("https://override.splunk.com:8089"))],
        || {
            let loader = ConfigLoader::new()
                .with_profile_name("dev".to_string())
                .with_config_path(config_path.clone())
                .from_profile()
                .unwrap()
                .from_env()
                .unwrap();

            let config = loader.build().unwrap();
            // Env var should take precedence over profile
            assert_eq!(
                config.connection.base_url,
                "https://override.splunk.com:8089"
            );
        },
    );
}

#[test]
#[serial]
fn test_empty_env_vars_ignored() {
    let _lock = env_lock().lock().unwrap();

    // Set empty env vars - they should be treated as None
    temp_env::with_vars(
        [
            ("SPLUNK_API_TOKEN", Some("")),
            ("SPLUNK_USERNAME", Some("")),
            ("SPLUNK_PASSWORD", Some("")),
        ],
        || {
            let loader = ConfigLoader::new()
                .with_base_url("https://localhost:8089".to_string())
                .with_username("admin".to_string()) // Set via builder
                .with_password("password".to_string())
                .from_env()
                .unwrap();

            let config = loader.build().unwrap();
            // Should use session auth since API token env is empty
            assert!(matches!(
                config.auth.strategy,
                AuthStrategy::SessionToken { .. }
            ));
        },
    );
}

#[test]
#[serial]
fn test_whitespace_only_env_var_treated_as_unset() {
    let _lock = env_lock().lock().unwrap();

    // Whitespace-only is filtered as empty/unset
    temp_env::with_vars(
        [
            ("SPLUNK_API_TOKEN", Some("   ")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_USERNAME", Some("admin")),
            ("SPLUNK_PASSWORD", Some("password")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            let config = loader.build().unwrap();
            // Whitespace API token should be ignored, falling back to session auth
            assert!(matches!(
                config.auth.strategy,
                AuthStrategy::SessionToken { .. }
            ));
        },
    );
}

#[test]
#[serial]
fn test_empty_and_whitespace_env_vars_ignored_for_non_string_fields() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("token")),
            ("SPLUNK_SKIP_VERIFY", Some("   ")),
            ("SPLUNK_TIMEOUT", Some("")),
            ("SPLUNK_MAX_RETRIES", Some(" ")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let config = loader.build().unwrap();

            // Should use defaults for bool/number fields instead of erroring on parse
            assert!(!config.connection.skip_verify);
            assert_eq!(config.connection.timeout, Duration::from_secs(30));
            assert_eq!(config.connection.max_retries, 3);
        },
    );
}

#[test]
#[serial]
fn test_splunk_config_path_env_var() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    temp_env::with_vars(
        [("SPLUNK_CONFIG_PATH", Some(config_path.to_str().unwrap()))],
        || {
            // Verify that with_config_path would use the environment variable's path
            let env_path = env_var_or_none("SPLUNK_CONFIG_PATH").unwrap();
            let path_from_env = std::path::PathBuf::from(env_path);

            let loader = ConfigLoader::new()
                .with_config_path(path_from_env)
                .with_profile_name("prod".to_string())
                .from_profile()
                .unwrap();

            let config = loader.build().unwrap();
            assert_eq!(config.connection.base_url, "https://prod.splunk.com:8089");
        },
    );
}

#[test]
#[serial]
fn test_empty_splunk_config_path_ignored() {
    let _lock = env_lock().lock().unwrap();

    // Empty string in SPLUNK_CONFIG_PATH should be ignored
    temp_env::with_vars([("SPLUNK_CONFIG_PATH", Some(""))], || {
        let result = env_var_or_none("SPLUNK_CONFIG_PATH");
        assert!(
            result.is_none(),
            "Empty env var should be filtered by env_var_or_none"
        );
    });

    // Test with whitespace
    temp_env::with_vars([("SPLUNK_CONFIG_PATH", Some("   "))], || {
        let result_ws = env_var_or_none("SPLUNK_CONFIG_PATH");
        assert!(
            result_ws.is_none(),
            "Whitespace env var should be filtered by env_var_or_none"
        );
    });
}

#[test]
#[serial]
fn test_search_defaults_env_vars() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_EARLIEST_TIME", Some("-48h")),
            ("SPLUNK_LATEST_TIME", Some("2024-01-01T00:00:00")),
            ("SPLUNK_MAX_RESULTS", Some("500")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            assert_eq!(loader.earliest_time(), Some(&"-48h".to_string()));
            assert_eq!(
                loader.latest_time(),
                Some(&"2024-01-01T00:00:00".to_string())
            );
            assert_eq!(loader.max_results(), Some(500));
        },
    );
}

#[test]
#[serial]
fn test_search_defaults_env_vars_empty_ignored() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_EARLIEST_TIME", Some("")),
            ("SPLUNK_LATEST_TIME", Some("   ")),
            ("SPLUNK_MAX_RESULTS", Some("")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            // Empty/whitespace values should be treated as None
            assert_eq!(loader.earliest_time(), None);
            assert_eq!(loader.latest_time(), None);
            assert_eq!(loader.max_results(), None);
        },
    );
}

#[test]
#[serial]
fn test_build_search_defaults_with_persisted() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_EARLIEST_TIME", Some("-7d")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            let persisted = SearchDefaults {
                earliest_time: "-24h".to_string(),
                latest_time: "now".to_string(),
                max_results: 1000,
            };

            let defaults = loader.build_search_defaults(Some(persisted));

            // Env var should override persisted
            assert_eq!(defaults.earliest_time, "-7d");
            // Non-env values should use persisted
            assert_eq!(defaults.latest_time, "now");
            assert_eq!(defaults.max_results, 1000);
        },
    );
}

#[test]
#[serial]
fn test_build_search_defaults_without_persisted() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            // Build without persisted defaults - should use hardcoded defaults
            let defaults = loader.build_search_defaults(None);

            assert_eq!(defaults.earliest_time, "-24h");
            assert_eq!(defaults.latest_time, "now");
            assert_eq!(defaults.max_results, 1000);
        },
    );
}

#[test]
#[serial]
fn test_search_defaults_env_vars_override_persisted() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_EARLIEST_TIME", Some("-1h")),
            ("SPLUNK_LATEST_TIME", Some("-5m")),
            ("SPLUNK_MAX_RESULTS", Some("100")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            let persisted = SearchDefaults {
                earliest_time: "-48h".to_string(),
                latest_time: "2024-01-01T00:00:00".to_string(),
                max_results: 5000,
            };

            let defaults = loader.build_search_defaults(Some(persisted));

            // All env vars should override persisted values
            assert_eq!(defaults.earliest_time, "-1h");
            assert_eq!(defaults.latest_time, "-5m");
            assert_eq!(defaults.max_results, 100);
        },
    );
}

#[test]
#[serial]
fn test_invalid_max_results_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_MAX_RESULTS", Some("not-a-number")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let result = ConfigLoader::new().from_env();

            match result {
                Err(ConfigError::InvalidValue { var, .. }) => {
                    assert_eq!(var, "SPLUNK_MAX_RESULTS");
                }
                Ok(_) => panic!("Expected an error for invalid SPLUNK_MAX_RESULTS"),
                Err(_) => panic!("Expected InvalidValue error for SPLUNK_MAX_RESULTS"),
            }
        },
    );
}

#[test]
#[serial]
fn test_session_ttl_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_SESSION_TTL", Some("7200")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let config = loader.build().unwrap();

            assert_eq!(config.connection.session_ttl_seconds, 7200);
            // Default buffer should still be 60
            assert_eq!(config.connection.session_expiry_buffer_seconds, 60);
        },
    );
}

#[test]
#[serial]
fn test_session_expiry_buffer_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_SESSION_EXPIRY_BUFFER", Some("120")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let config = loader.build().unwrap();

            assert_eq!(config.connection.session_expiry_buffer_seconds, 120);
            // Default TTL should still be 3600
            assert_eq!(config.connection.session_ttl_seconds, 3600);
        },
    );
}

#[test]
#[serial]
fn test_session_ttl_and_buffer_env_vars_together() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_SESSION_TTL", Some("7200")),
            ("SPLUNK_SESSION_EXPIRY_BUFFER", Some("120")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let config = loader.build().unwrap();

            assert_eq!(config.connection.session_ttl_seconds, 7200);
            assert_eq!(config.connection.session_expiry_buffer_seconds, 120);
        },
    );
}

#[test]
#[serial]
fn test_invalid_session_ttl_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_SESSION_TTL", Some("not-a-number")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let result = ConfigLoader::new().from_env();

            match result {
                Err(ConfigError::InvalidValue { var, .. }) => {
                    assert_eq!(var, "SPLUNK_SESSION_TTL");
                }
                Ok(_) => panic!("Expected an error for invalid SPLUNK_SESSION_TTL"),
                Err(_) => panic!("Expected InvalidValue error for SPLUNK_SESSION_TTL"),
            }
        },
    );
}

#[test]
#[serial]
fn test_session_ttl_default_values() {
    let _lock = env_lock().lock().unwrap();

    // Don't set any session env vars
    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let config = loader.build().unwrap();

            // Should use defaults
            assert_eq!(config.connection.session_ttl_seconds, 3600);
            assert_eq!(config.connection.session_expiry_buffer_seconds, 60);
        },
    );
}

#[test]
#[serial]
fn test_health_check_interval_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_HEALTH_CHECK_INTERVAL", Some("120")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let config = loader.build().unwrap();

            assert_eq!(config.connection.health_check_interval_seconds, 120);
        },
    );
}

#[test]
#[serial]
fn test_health_check_interval_default() {
    let _lock = env_lock().lock().unwrap();

    // Don't set health check interval env var
    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let config = loader.build().unwrap();

            // Should use default of 60 seconds
            assert_eq!(config.connection.health_check_interval_seconds, 60);
        },
    );
}

#[test]
#[serial]
fn test_invalid_health_check_interval_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_HEALTH_CHECK_INTERVAL", Some("not-a-number")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let result = ConfigLoader::new().from_env();

            match result {
                Err(ConfigError::InvalidValue { var, .. }) => {
                    assert_eq!(var, "SPLUNK_HEALTH_CHECK_INTERVAL");
                }
                Ok(_) => panic!("Expected an error for invalid SPLUNK_HEALTH_CHECK_INTERVAL"),
                Err(_) => {
                    panic!("Expected InvalidValue error for SPLUNK_HEALTH_CHECK_INTERVAL")
                }
            }
        },
    );
}

// ============================================================================
// Timeout Configuration Validation Tests
// ============================================================================

#[test]
fn test_timeout_zero_invalid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(0));

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidTimeout { message }) => {
            assert!(
                message.contains("must be greater than 0"),
                "Expected message about timeout > 0, got: {}",
                message
            );
        }
        Ok(_) => panic!("Expected InvalidTimeout error for zero timeout, got Ok"),
        Err(ref e) => panic!(
            "Expected InvalidTimeout error for zero timeout, got {:?}",
            e
        ),
    }
}

#[test]
fn test_timeout_exceeds_max_invalid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(3601)); // MAX_TIMEOUT_SECS + 1

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidTimeout { message }) => {
            assert!(
                message.contains("exceeds maximum"),
                "Expected message about exceeding max, got: {}",
                message
            );
        }
        Ok(_) => panic!("Expected InvalidTimeout error for timeout exceeding max, got Ok"),
        Err(ref e) => panic!(
            "Expected InvalidTimeout error for timeout exceeding max, got {:?}",
            e
        ),
    }
}

#[test]
fn test_timeout_valid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(300)); // 5 minutes

    let config = loader.build().unwrap();
    assert_eq!(config.connection.timeout, Duration::from_secs(300));
}

#[test]
fn test_timeout_at_max_boundary_valid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(3600)); // Exactly MAX_TIMEOUT_SECS

    let config = loader.build().unwrap();
    assert_eq!(config.connection.timeout, Duration::from_secs(3600));
}

#[test]
fn test_session_ttl_less_than_buffer_invalid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Manually set invalid values using internal setters
    let mut loader = loader;
    loader.set_session_ttl_seconds(Some(60));
    loader.set_session_expiry_buffer_seconds(Some(120)); // TTL < buffer

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidSessionTtl { message }) => {
            assert!(
                message.contains("must be greater than"),
                "Expected message about TTL > buffer, got: {}",
                message
            );
        }
        Ok(_) => panic!("Expected InvalidSessionTtl error when TTL < buffer, got Ok"),
        Err(ref e) => panic!(
            "Expected InvalidSessionTtl error when TTL < buffer, got {:?}",
            e
        ),
    }
}

#[test]
fn test_session_ttl_equals_buffer_invalid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Manually set invalid values using internal setters
    let mut loader = loader;
    loader.set_session_ttl_seconds(Some(120));
    loader.set_session_expiry_buffer_seconds(Some(120)); // TTL == buffer

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidSessionTtl { message }) => {
            assert!(
                message.contains("must be greater than"),
                "Expected message about TTL > buffer, got: {}",
                message
            );
        }
        Ok(_) => panic!("Expected InvalidSessionTtl error when TTL == buffer, got Ok"),
        Err(ref e) => panic!(
            "Expected InvalidSessionTtl error when TTL == buffer, got {:?}",
            e
        ),
    }
}

#[test]
fn test_session_ttl_valid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Manually set valid values using internal setters
    let mut loader = loader;
    loader.set_session_ttl_seconds(Some(3600));
    loader.set_session_expiry_buffer_seconds(Some(60)); // TTL > buffer

    let config = loader.build().unwrap();
    assert_eq!(config.connection.session_ttl_seconds, 3600);
    assert_eq!(config.connection.session_expiry_buffer_seconds, 60);
}

#[test]
fn test_session_ttl_exceeds_max_invalid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Manually set invalid values using internal setters
    let mut loader = loader;
    loader.set_session_ttl_seconds(Some(86401)); // MAX_SESSION_TTL_SECS + 1
    loader.set_session_expiry_buffer_seconds(Some(60));

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidSessionTtl { message }) => {
            assert!(
                message.contains("exceeds maximum"),
                "Expected message about exceeding max, got: {}",
                message
            );
        }
        Ok(_) => panic!("Expected InvalidSessionTtl error when TTL exceeds max, got Ok"),
        Err(ref e) => panic!(
            "Expected InvalidSessionTtl error when TTL exceeds max, got {:?}",
            e
        ),
    }
}

#[test]
fn test_session_ttl_at_max_boundary_valid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Manually set valid values using internal setters
    let mut loader = loader;
    loader.set_session_ttl_seconds(Some(86400)); // Exactly MAX_SESSION_TTL_SECS
    loader.set_session_expiry_buffer_seconds(Some(60));

    let config = loader.build().unwrap();
    assert_eq!(config.connection.session_ttl_seconds, 86400);
}

#[test]
fn test_health_check_interval_zero_invalid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Manually set invalid value using internal setter
    let mut loader = loader;
    loader.set_health_check_interval_seconds(Some(0));

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidHealthCheckInterval { message }) => {
            assert!(
                message.contains("must be greater than 0"),
                "Expected message about interval > 0, got: {}",
                message
            );
        }
        Ok(_) => panic!("Expected InvalidHealthCheckInterval error for zero interval, got Ok"),
        Err(ref e) => panic!(
            "Expected InvalidHealthCheckInterval error for zero interval, got {:?}",
            e
        ),
    }
}

#[test]
fn test_health_check_interval_exceeds_max_invalid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Manually set invalid value using internal setter
    let mut loader = loader;
    loader.set_health_check_interval_seconds(Some(3601)); // MAX_HEALTH_CHECK_INTERVAL_SECS + 1

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidHealthCheckInterval { message }) => {
            assert!(
                message.contains("exceeds maximum"),
                "Expected message about exceeding max, got: {}",
                message
            );
        }
        Ok(_) => {
            panic!("Expected InvalidHealthCheckInterval error for interval exceeding max, got Ok")
        }
        Err(ref e) => panic!(
            "Expected InvalidHealthCheckInterval error for interval exceeding max, got {:?}",
            e
        ),
    }
}

#[test]
fn test_health_check_interval_valid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Manually set valid value using internal setter
    let mut loader = loader;
    loader.set_health_check_interval_seconds(Some(120));

    let config = loader.build().unwrap();
    assert_eq!(config.connection.health_check_interval_seconds, 120);
}

#[test]
fn test_health_check_interval_at_max_boundary_valid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Manually set valid value using internal setter
    let mut loader = loader;
    loader.set_health_check_interval_seconds(Some(3600)); // Exactly MAX_HEALTH_CHECK_INTERVAL_SECS

    let config = loader.build().unwrap();
    assert_eq!(config.connection.health_check_interval_seconds, 3600);
}

#[test]
#[serial]
fn test_timeout_validation_via_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
            ("SPLUNK_TIMEOUT", Some("0")), // Invalid: zero timeout
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let result = loader.build();

            match result {
                Err(ConfigError::InvalidTimeout { message }) => {
                    assert!(
                        message.contains("must be greater than 0"),
                        "Expected message about timeout > 0, got: {}",
                        message
                    );
                }
                Ok(_) => panic!("Expected InvalidTimeout error for zero timeout from env, got Ok"),
                Err(ref e) => panic!(
                    "Expected InvalidTimeout error for zero timeout from env, got {:?}",
                    e
                ),
            }
        },
    );
}

#[test]
#[serial]
fn test_session_ttl_validation_via_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
            ("SPLUNK_SESSION_TTL", Some("30")), // 30 seconds TTL
            ("SPLUNK_SESSION_EXPIRY_BUFFER", Some("60")), // 60 seconds buffer (TTL < buffer)
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let result = loader.build();

            match result {
                Err(ConfigError::InvalidSessionTtl { message }) => {
                    assert!(
                        message.contains("must be greater than"),
                        "Expected message about TTL > buffer, got: {}",
                        message
                    );
                }
                Ok(_) => {
                    panic!("Expected InvalidSessionTtl error when TTL < buffer from env, got Ok")
                }
                Err(ref e) => panic!(
                    "Expected InvalidSessionTtl error when TTL < buffer from env, got {:?}",
                    e
                ),
            }
        },
    );
}

#[test]
#[serial]
fn test_health_check_interval_validation_via_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
            ("SPLUNK_HEALTH_CHECK_INTERVAL", Some("0")), // Invalid: zero interval
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let result = loader.build();

            match result {
                Err(ConfigError::InvalidHealthCheckInterval { message }) => {
                    assert!(
                        message.contains("must be greater than 0"),
                        "Expected message about interval > 0, got: {}",
                        message
                    );
                }
                Ok(_) => panic!(
                    "Expected InvalidHealthCheckInterval error for zero interval from env, got Ok"
                ),
                Err(ref e) => panic!(
                    "Expected InvalidHealthCheckInterval error for zero interval from env, got {:?}",
                    e
                ),
            }
        },
    );
}
