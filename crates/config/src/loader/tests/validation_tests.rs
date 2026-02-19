//! Validation tests for the configuration loader builder.
//!
//! Responsibilities:
//! - Test timeout configuration validation (zero, max boundary, valid values).
//! - Test session TTL validation (TTL vs buffer relationship, max boundary).
//! - Test health check interval validation (zero, max boundary, valid values).
//! - Test max retries validation (zero allowed, max boundary, valid values).
//! - Test validation via environment variables.

use crate::constants::{MAX_MAX_RETRIES, MAX_TIMEOUT_SECS};
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

    // Use values above minimums but TTL < buffer
    let mut loader = loader;
    loader.set_session_ttl_seconds(Some(130)); // Above 120 minimum
    loader.set_session_expiry_buffer_seconds(Some(200)); // Above 10 minimum, but > TTL

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
// Minimum Session TTL Validation Tests
// ============================================================================

#[test]
fn test_session_ttl_below_minimum_invalid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Set TTL below minimum (MIN_SESSION_TTL_SECS = 120)
    let mut loader = loader;
    loader.set_session_ttl_seconds(Some(60)); // Below 120 minimum
    loader.set_session_expiry_buffer_seconds(Some(10));

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidSessionTtl { message }) => {
            assert!(
                message.contains("must be at least"),
                "Expected message about minimum TTL, got: {}",
                message
            );
            assert!(
                message.contains("120"),
                "Expected minimum value in message, got: {}",
                message
            );
        }
        Ok(_) => panic!("Expected InvalidSessionTtl error when TTL below minimum, got Ok"),
        Err(ref e) => panic!(
            "Expected InvalidSessionTtl error when TTL below minimum, got {:?}",
            e
        ),
    }
}

#[test]
fn test_session_ttl_at_minimum_valid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Set TTL exactly at minimum (MIN_SESSION_TTL_SECS = 120)
    let mut loader = loader;
    loader.set_session_ttl_seconds(Some(120));
    loader.set_session_expiry_buffer_seconds(Some(10)); // Below TTL, at buffer minimum

    let config = loader.build().unwrap();
    assert_eq!(config.connection.session_ttl_seconds, 120);
}

// ============================================================================
// Minimum Expiry Buffer Validation Tests
// ============================================================================

#[test]
fn test_expiry_buffer_below_minimum_invalid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Set buffer below minimum (MIN_EXPIRY_BUFFER_SECS = 10)
    let mut loader = loader;
    loader.set_session_ttl_seconds(Some(3600));
    loader.set_session_expiry_buffer_seconds(Some(5)); // Below 10 minimum

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidExpiryBuffer { message }) => {
            assert!(
                message.contains("must be at least"),
                "Expected message about minimum buffer, got: {}",
                message
            );
            assert!(
                message.contains("10"),
                "Expected minimum value in message, got: {}",
                message
            );
        }
        Ok(_) => panic!("Expected InvalidExpiryBuffer error when buffer below minimum, got Ok"),
        Err(ref e) => panic!(
            "Expected InvalidExpiryBuffer error when buffer below minimum, got {:?}",
            e
        ),
    }
}

#[test]
fn test_expiry_buffer_at_minimum_valid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Set buffer exactly at minimum (MIN_EXPIRY_BUFFER_SECS = 10)
    let mut loader = loader;
    loader.set_session_ttl_seconds(Some(3600));
    loader.set_session_expiry_buffer_seconds(Some(10));

    let config = loader.build().unwrap();
    assert_eq!(config.connection.session_expiry_buffer_seconds, 10);
}

#[test]
fn test_session_values_above_minimum_valid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30));

    // Set values comfortably above minimums
    let mut loader = loader;
    loader.set_session_ttl_seconds(Some(1800)); // 30 min, well above 120s min
    loader.set_session_expiry_buffer_seconds(Some(60)); // 1 min, well above 10s min

    let config = loader.build().unwrap();
    assert_eq!(config.connection.session_ttl_seconds, 1800);
    assert_eq!(config.connection.session_expiry_buffer_seconds, 60);
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
            ("SPLUNK_SESSION_TTL", Some("130")), // Above 120 minimum
            ("SPLUNK_SESSION_EXPIRY_BUFFER", Some("200")), // Above 10 minimum, but > TTL
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

// ============================================================================
// Max Retries Validation Tests
// ============================================================================

#[test]
fn test_max_retries_zero_valid() {
    // Zero is explicitly allowed for "no retry" scenarios
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30))
        .with_max_retries(0);

    let config = loader.build().unwrap();
    assert_eq!(config.connection.max_retries, 0);
}

#[test]
fn test_max_retries_exceeds_max_invalid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30))
        .with_max_retries(11); // MAX_MAX_RETRIES + 1

    let result = loader.build();
    match result {
        Err(ConfigError::InvalidMaxRetries { message }) => {
            assert!(
                message.contains("exceeds maximum"),
                "Expected message about exceeding max, got: {}",
                message
            );
        }
        Ok(_) => panic!("Expected InvalidMaxRetries error for max_retries exceeding max, got Ok"),
        Err(ref e) => panic!(
            "Expected InvalidMaxRetries error for max_retries exceeding max, got {:?}",
            e
        ),
    }
}

#[test]
fn test_max_retries_at_max_boundary_valid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30))
        .with_max_retries(MAX_MAX_RETRIES); // Exactly MAX_MAX_RETRIES

    let config = loader.build().unwrap();
    assert_eq!(config.connection.max_retries, MAX_MAX_RETRIES);
}

#[test]
fn test_max_retries_valid() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(30))
        .with_max_retries(3);

    let config = loader.build().unwrap();
    assert_eq!(config.connection.max_retries, 3);
}

#[test]
#[serial]
fn test_session_ttl_below_minimum_via_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
            ("SPLUNK_SESSION_TTL", Some("30")), // Below 120 minimum, but > buffer
            ("SPLUNK_SESSION_EXPIRY_BUFFER", Some("10")), // At minimum, less than TTL
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let result = loader.build();

            match result {
                Err(ConfigError::InvalidSessionTtl { message }) => {
                    assert!(
                        message.contains("must be at least"),
                        "Expected minimum TTL error, got: {}",
                        message
                    );
                }
                Ok(_) => panic!("Expected InvalidSessionTtl error for TTL below minimum from env"),
                Err(ref e) => panic!("Expected InvalidSessionTtl, got {:?}", e),
            }
        },
    );
}

#[test]
#[serial]
fn test_expiry_buffer_below_minimum_via_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
            ("SPLUNK_SESSION_EXPIRY_BUFFER", Some("5")), // Below 10 minimum
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let result = loader.build();

            match result {
                Err(ConfigError::InvalidExpiryBuffer { message }) => {
                    assert!(
                        message.contains("must be at least"),
                        "Expected minimum buffer error, got: {}",
                        message
                    );
                }
                Ok(_) => {
                    panic!("Expected InvalidExpiryBuffer error for buffer below minimum from env")
                }
                Err(ref e) => panic!("Expected InvalidExpiryBuffer, got {:?}", e),
            }
        },
    );
}

#[test]
#[serial]
fn test_session_at_minimum_via_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
            ("SPLUNK_SESSION_TTL", Some("120")), // Exactly MIN_SESSION_TTL_SECS
            ("SPLUNK_SESSION_EXPIRY_BUFFER", Some("10")), // Exactly MIN_EXPIRY_BUFFER_SECS
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let config = loader.build().unwrap();

            assert_eq!(config.connection.session_ttl_seconds, 120);
            assert_eq!(config.connection.session_expiry_buffer_seconds, 10);
        },
    );
}

#[test]
#[serial]
fn test_max_retries_validation_via_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
            ("SPLUNK_MAX_RETRIES", Some("15")), // Invalid: exceeds max
        ],
        || {
            let result = ConfigLoader::new().from_env();

            match result {
                Err(ConfigError::InvalidMaxRetries { message }) => {
                    assert!(
                        message.contains("15") && message.contains(&format!("{}", MAX_MAX_RETRIES)),
                        "Expected message about max_retries bounds, got: {}",
                        message
                    );
                }
                Ok(_) => panic!(
                    "Expected InvalidMaxRetries error for max_retries exceeding max from env, got Ok"
                ),
                Err(ref e) => panic!(
                    "Expected InvalidMaxRetries error for max_retries exceeding max from env, got {:?}",
                    e
                ),
            }
        },
    );
}

#[test]
#[serial]
fn test_max_retries_valid_via_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
            ("SPLUNK_MAX_RETRIES", Some("5")), // Valid: within bounds
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let config = loader.build().unwrap();
            assert_eq!(config.connection.max_retries, 5);
        },
    );
}

#[test]
#[serial]
fn test_max_retries_zero_via_env_var() {
    let _lock = env_lock().lock().unwrap();

    temp_env::with_vars(
        [
            ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
            ("SPLUNK_API_TOKEN", Some("test-token")),
            ("SPLUNK_MAX_RETRIES", Some("0")), // Valid: zero is allowed
        ],
        || {
            let loader = ConfigLoader::new().from_env().unwrap();
            let config = loader.build().unwrap();
            assert_eq!(config.connection.max_retries, 0);
        },
    );
}

// ============================================================================
// Timeout Validation at Max Boundary
// ============================================================================

#[test]
fn test_timeout_at_max_boundary_constant() {
    let loader = ConfigLoader::new()
        .with_base_url("https://localhost:8089".to_string())
        .with_api_token("test-token".to_string())
        .with_timeout(Duration::from_secs(MAX_TIMEOUT_SECS));

    let config = loader.build().unwrap();
    assert_eq!(
        config.connection.timeout,
        Duration::from_secs(MAX_TIMEOUT_SECS)
    );
}
