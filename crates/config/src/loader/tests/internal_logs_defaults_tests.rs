//! Internal logs defaults tests for the configuration loader builder.
//!
//! Responsibilities:
//! - Test internal logs defaults environment variable handling.
//! - Test internal logs defaults merging with persisted values.
//! - Test validation of internal logs defaults values.

use crate::loader::builder::ConfigLoader;
use crate::loader::error::ConfigError;
use crate::persistence::InternalLogsDefaults;
use serial_test::serial;

use super::env_lock;

#[test]
#[serial]
fn test_internal_logs_defaults_env_vars() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_INTERNAL_LOGS_COUNT", Some("200")),
            ("SPLUNK_INTERNAL_LOGS_EARLIEST", Some("-1h")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            assert_eq!(loader.internal_logs_count(), Some(200));
            assert_eq!(loader.internal_logs_earliest(), Some(&"-1h".to_string()));
        },
    );
}

#[test]
#[serial]
fn test_internal_logs_defaults_env_vars_empty_ignored() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_INTERNAL_LOGS_COUNT", Some("")),
            ("SPLUNK_INTERNAL_LOGS_EARLIEST", Some("   ")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            // Empty/whitespace values should be treated as None
            assert_eq!(loader.internal_logs_count(), None);
            assert_eq!(loader.internal_logs_earliest(), None);
        },
    );
}

#[test]
#[serial]
fn test_build_internal_logs_defaults_with_persisted() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_INTERNAL_LOGS_COUNT", Some("50")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            let persisted = InternalLogsDefaults {
                count: 100,
                earliest_time: "-15m".to_string(),
            };

            let defaults = loader.build_internal_logs_defaults(Some(persisted));

            // Env var should override persisted
            assert_eq!(defaults.count, 50);
            // Non-env values should use persisted
            assert_eq!(defaults.earliest_time, "-15m");
        },
    );
}

#[test]
#[serial]
fn test_build_internal_logs_defaults_without_persisted() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            // Build without persisted defaults - should use hardcoded defaults
            let defaults = loader.build_internal_logs_defaults(None);

            assert_eq!(defaults.count, 100);
            assert_eq!(defaults.earliest_time, "-15m");
        },
    );
}

#[test]
#[serial]
fn test_internal_logs_defaults_env_vars_override_persisted() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_INTERNAL_LOGS_COUNT", Some("500")),
            ("SPLUNK_INTERNAL_LOGS_EARLIEST", Some("-30m")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();

            let persisted = InternalLogsDefaults {
                count: 200,
                earliest_time: "-1h".to_string(),
            };

            let defaults = loader.build_internal_logs_defaults(Some(persisted));

            // All env vars should override persisted values
            assert_eq!(defaults.count, 500);
            assert_eq!(defaults.earliest_time, "-30m");
        },
    );
}

#[test]
#[serial]
fn test_invalid_internal_logs_count_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_INTERNAL_LOGS_COUNT", Some("not-a-number")),
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
        ],
        || {
            let result = ConfigLoader::new().from_env();

            match result {
                Err(ConfigError::InvalidValue { var, .. }) => {
                    assert_eq!(var, "SPLUNK_INTERNAL_LOGS_COUNT");
                }
                Ok(_) => panic!("Expected an error for invalid SPLUNK_INTERNAL_LOGS_COUNT"),
                Err(_) => panic!("Expected InvalidValue error for SPLUNK_INTERNAL_LOGS_COUNT"),
            }
        },
    );
}
