//! Session TTL and expiry buffer tests for the configuration loader builder.
//!
//! Responsibilities:
//! - Test session TTL environment variable handling.
//! - Test session expiry buffer environment variable handling.
//! - Test default values for session settings.
//! - Test validation of invalid session TTL values.

use crate::loader::builder::ConfigLoader;
use crate::loader::error::ConfigError;
use serial_test::serial;

use super::env_lock;

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
