//! Validation tests for the configuration loader builder.
//!
//! Responsibilities:
//! - Test timeout configuration validation (zero, max boundary, valid values).
//! - Test session TTL validation (TTL vs buffer relationship, max boundary).
//! - Test health check interval validation (zero, max boundary, valid values).
//! - Test validation via environment variables.

use crate::loader::builder::ConfigLoader;
use crate::loader::error::ConfigError;
use serial_test::serial;
use std::time::Duration;

use super::env_lock;

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

// ============================================================================
// Session TTL Validation Tests
// ============================================================================

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

// ============================================================================
// Health Check Interval Validation Tests
// ============================================================================

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

// ============================================================================
// Base URL Validation Tests
// ============================================================================

#[test]
fn test_base_url_accepts_and_normalizes_whitespace() {
    let loader = ConfigLoader::new()
        .with_base_url("  https://localhost:8089  ".to_string())
        .with_api_token("test-token".to_string());

    let config = loader.build().unwrap();
    assert_eq!(config.connection.base_url, "https://localhost:8089");
}

#[test]
fn test_base_url_accepts_and_strips_trailing_slash() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089/".to_string())
        .with_api_token("test-token".to_string());

    let config = loader.build().unwrap();
    assert_eq!(config.connection.base_url, "https://localhost:8089");
}

#[test]
fn test_base_url_accepts_with_path_and_strips_trailing_slash() {
    let loader = ConfigLoader::new()
        .with_base_url("https://splunk.example.com:8089/custom/path/".to_string())
        .with_api_token("test-token".to_string());

    let config = loader.build().unwrap();
    assert_eq!(
        config.connection.base_url,
        "https://splunk.example.com:8089/custom/path"
    );
}

#[test]
fn test_base_url_rejects_missing_scheme() {
    let loader = ConfigLoader::new()
        .with_base_url("localhost:8089".to_string())
        .with_api_token("test-token".to_string());

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidValue { var, message }) => {
            assert_eq!(var, "base_url");
            assert!(
                message.contains("http") && message.contains("https"),
                "Expected message mentioning http/https scheme, got: {}",
                message
            );
        }
        Ok(_) => panic!("Expected InvalidValue error for missing scheme, got Ok"),
        Err(ref e) => panic!(
            "Expected InvalidValue error for missing scheme, got {:?}",
            e
        ),
    }
}

#[test]
fn test_base_url_rejects_unsupported_scheme() {
    let loader = ConfigLoader::new()
        .with_base_url("ftp://localhost:8089".to_string())
        .with_api_token("test-token".to_string());

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidValue { var, message }) => {
            assert_eq!(var, "base_url");
            assert!(
                message.contains("scheme must be http or https"),
                "Expected message about http/https scheme requirement, got: {}",
                message
            );
        }
        Ok(_) => panic!("Expected InvalidValue error for unsupported scheme, got Ok"),
        Err(ref e) => panic!(
            "Expected InvalidValue error for unsupported scheme, got {:?}",
            e
        ),
    }
}

#[test]
fn test_base_url_rejects_missing_host() {
    let loader = ConfigLoader::new()
        .with_base_url("https:///".to_string())
        .with_api_token("test-token".to_string());

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidValue { var, message }) => {
            assert_eq!(var, "base_url");
            // URL parser catches empty host before our custom check
            assert!(
                message.contains("host") || message.contains("empty host"),
                "Expected message about host requirement, got: {}",
                message
            );
        }
        Ok(_) => panic!("Expected InvalidValue error for missing host, got Ok"),
        Err(ref e) => panic!("Expected InvalidValue error for missing host, got {:?}", e),
    }
}

#[test]
fn test_base_url_rejects_blank_whitespace_only() {
    let loader = ConfigLoader::new()
        .with_base_url("   ".to_string())
        .with_api_token("test-token".to_string());

    let result = loader.build();
    assert!(
        matches!(result, Err(ConfigError::MissingBaseUrl)),
        "Expected MissingBaseUrl for whitespace-only base_url, got {:?}",
        result
    );
}

#[test]
fn test_base_url_rejects_empty_string() {
    let loader = ConfigLoader::new()
        .with_base_url("".to_string())
        .with_api_token("test-token".to_string());

    let result = loader.build();
    assert!(
        matches!(result, Err(ConfigError::MissingBaseUrl)),
        "Expected MissingBaseUrl for empty base_url, got {:?}",
        result
    );
}

// ============================================================================
// Environment Variable Validation Tests
// ============================================================================

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
