//! Environment variable tests for the configuration loader builder.
//!
//! Responsibilities:
//! - Test environment variable overrides for profile values.
//! - Test handling of empty and whitespace-only environment variables.
//! - Test SPLUNK_CONFIG_PATH environment variable handling.

use crate::loader::builder::ConfigLoader;
use crate::loader::env::env_var_or_none;
use crate::types::AuthStrategy;
use secrecy::ExposeSecret;
use serial_test::serial;
use std::path::PathBuf;

use super::env_lock;
use super::profile_tests::create_test_config_file;
use tempfile::TempDir;

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
            assert_eq!(
                config.connection.timeout,
                std::time::Duration::from_secs(30)
            );
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
            let path_from_env = PathBuf::from(env_path);

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
fn test_env_vars_trimmed_for_session_auth() {
    let _lock = env_lock().lock().unwrap();

    // Session auth path with whitespace-padded values
    // SPLUNK_API_TOKEN is explicitly unset (None) to ensure we use session auth
    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_USERNAME", Some(" admin ")),
            ("SPLUNK_PASSWORD", Some(" password ")),
            ("SPLUNK_API_TOKEN", None::<&str>),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let config = loader.build().unwrap();

            // Auth fields should be trimmed
            match config.auth.strategy {
                AuthStrategy::SessionToken { username, password } => {
                    assert_eq!(username, "admin", "Username should be trimmed");
                    assert_eq!(
                        password.expose_secret(),
                        "password",
                        "Password should be trimmed"
                    );
                }
                _ => panic!("Expected SessionToken auth strategy"),
            }
        },
    );
}

#[test]
#[serial]
fn test_env_vars_trimmed_for_api_token_auth() {
    let _lock = env_lock().lock().unwrap();

    // API token path with whitespace-padded value
    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some(" token ")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let config = loader.build().unwrap();

            // API token should be trimmed
            match config.auth.strategy {
                AuthStrategy::ApiToken { token } => {
                    assert_eq!(
                        token.expose_secret(),
                        "token",
                        "API token should be trimmed"
                    );
                }
                _ => panic!("Expected ApiToken auth strategy"),
            }
        },
    );
}

#[test]
#[serial]
fn test_splunk_profile_env_var() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    temp_env::with_vars(
        [
            ("SPLUNK_CONFIG_PATH", Some(config_path.to_str().unwrap())),
            ("SPLUNK_PROFILE", Some("prod")),
            ("SPLUNK_BASE_URL", None::<&str>),
            ("SPLUNK_USERNAME", None::<&str>),
            ("SPLUNK_PASSWORD", None::<&str>),
            ("SPLUNK_API_TOKEN", None::<&str>),
        ],
        || {
            let loader = ConfigLoader::new()
                .from_env()
                .unwrap()
                .from_profile()
                .unwrap();

            let config = loader.build().unwrap();
            // Should use the "prod" profile from the test config file
            assert_eq!(config.connection.base_url, "https://prod.splunk.com:8089");
        },
    );
}

#[test]
#[serial]
fn test_splunk_profile_env_var_blank_ignored() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    // Empty profile env var should be ignored
    temp_env::with_vars(
        [
            ("SPLUNK_CONFIG_PATH", Some(config_path.to_str().unwrap())),
            ("SPLUNK_PROFILE", Some("")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            // Profile should not be set (env var filtered as empty)
            assert!(loader.profile_name().is_none());
            // But config should still build using env vars
            let config = loader.build().unwrap();
            assert_eq!(config.connection.base_url, "https://localhost:8089");
        },
    );

    // Whitespace-only profile env var should also be ignored
    temp_env::with_vars(
        [
            ("SPLUNK_CONFIG_PATH", Some(config_path.to_str().unwrap())),
            ("SPLUNK_PROFILE", Some("   ")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            assert!(loader.profile_name().is_none());
            let config = loader.build().unwrap();
            assert_eq!(config.connection.base_url, "https://localhost:8089");
        },
    );
}

#[test]
#[serial]
fn test_splunk_config_path_env_var_via_apply_env() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    temp_env::with_vars(
        [
            ("SPLUNK_CONFIG_PATH", Some(config_path.to_str().unwrap())),
            ("SPLUNK_PROFILE", Some("prod")),
            ("SPLUNK_BASE_URL", None::<&str>),
            ("SPLUNK_USERNAME", None::<&str>),
            ("SPLUNK_PASSWORD", None::<&str>),
            ("SPLUNK_API_TOKEN", None::<&str>),
        ],
        || {
            // Test that apply_env() sets config_path from SPLUNK_CONFIG_PATH
            let loader = ConfigLoader::new()
                .from_env()
                .unwrap()
                .from_profile()
                .unwrap();

            let config = loader.build().unwrap();
            assert_eq!(config.connection.base_url, "https://prod.splunk.com:8089");
        },
    );
}

#[test]
#[serial]
fn test_cli_args_override_env_vars() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let config_path = create_test_config_file(temp_dir.path());

    temp_env::with_vars(
        [
            ("SPLUNK_CONFIG_PATH", Some(config_path.to_str().unwrap())),
            ("SPLUNK_PROFILE", Some("prod")), // env var says "prod"
            ("SPLUNK_BASE_URL", None::<&str>),
            ("SPLUNK_USERNAME", None::<&str>),
            ("SPLUNK_PASSWORD", None::<&str>),
            ("SPLUNK_API_TOKEN", None::<&str>),
        ],
        || {
            // CLI arg overrides env var - set profile_name BEFORE from_env()
            let loader = ConfigLoader::new()
                .with_profile_name("dev".to_string()) // CLI says "dev"
                .from_env()
                .unwrap()
                .from_profile()
                .unwrap();

            let config = loader.build().unwrap();
            // Should use "dev" profile (CLI arg takes precedence)
            assert_eq!(config.connection.base_url, "https://dev.splunk.com:8089");
        },
    );
}

#[test]
#[serial]
fn test_cli_config_path_override_env_var() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();

    // Create a config file with a prod profile
    let config_path2 = temp_dir.path().join("config2.json");
    std::fs::write(
        &config_path2,
        r#"{
            "profiles": {
                "prod": {
                    "base_url": "https://prod-config2.splunk.com:8089",
                    "auth": {
                        "type": "api_token",
                        "token": "prod_token_2"
                    }
                }
            }
        }"#,
    )
    .unwrap();

    // Verify file exists
    assert!(config_path2.exists(), "Config file should exist");

    // Test: CLI arg for config_path takes precedence over env var
    // Even if SPLUNK_CONFIG_PATH points elsewhere, CLI --config-path wins
    temp_env::with_vars(
        [
            // Set a non-existent path in env var - this should be ignored
            ("SPLUNK_CONFIG_PATH", Some("/nonexistent/path/config.json")),
            ("SPLUNK_PROFILE", None::<&str>),
        ],
        || {
            // CLI arg for config_path should be used, not the env var
            let loader = ConfigLoader::new()
                .with_config_path(config_path2.clone()) // CLI says config2.json
                .with_profile_name("prod".to_string())  // CLI also sets profile
                .from_env()  // This should NOT overwrite config_path since it's already set
                .unwrap()
                .from_profile()
                .unwrap();

            // Verify that the CLI config_path was preserved (not overwritten by env)
            assert_eq!(
                loader.config_path(),
                Some(&config_path2),
                "CLI config_path should take precedence over env var"
            );

            let config = loader.build().unwrap();
            // Should use "prod" profile from config2.json (CLI path takes precedence)
            assert_eq!(
                config.connection.base_url,
                "https://prod-config2.splunk.com:8089"
            );
        },
    );
}
