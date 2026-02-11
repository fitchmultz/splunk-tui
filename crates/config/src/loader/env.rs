//! Environment variable parsing for configuration.
//!
//! Responsibilities:
//! - Read and parse environment variables for Splunk configuration.
//! - Apply environment variable values to a ConfigLoader instance.
//! - Provide helper functions for reading env vars with empty/whitespace filtering.
//!
//! Does NOT handle:
//! - Loading from profile files (see profile.rs).
//! - Building the final Config (see builder.rs).
//! - .env file loading (handled by ConfigLoader::load_dotenv).
//!
//! Invariants:
//! - Environment variables take precedence over profile settings.
//! - Empty or whitespace-only environment variables are treated as unset.
//! - Returned values are trimmed (leading/trailing whitespace removed).
//! - Invalid numeric values return ConfigError::InvalidValue.

use secrecy::SecretString;
use std::time::Duration;

use super::builder::ConfigLoader;
use super::error::ConfigError;
use crate::constants::MAX_MAX_RETRIES;

/// Read an environment variable, returning None if unset, empty, or whitespace-only.
/// Returns the trimmed value (leading/trailing whitespace removed) if present.
pub fn env_var_or_none(key: &str) -> Option<String> {
    std::env::var(key).ok().and_then(|s| {
        let trimmed = s.trim();
        if trimmed.is_empty() {
            None
        } else if trimmed.len() == s.len() {
            // No trimming needed, return original to avoid allocation
            Some(s)
        } else {
            // Trimming was needed, allocate new String
            Some(trimmed.to_string())
        }
    })
}

/// Apply environment variable configuration to the loader.
///
/// Environment variables take precedence over profile settings.
pub fn apply_env(loader: &mut ConfigLoader) -> Result<(), ConfigError> {
    if let Some(url) = env_var_or_none("SPLUNK_BASE_URL") {
        loader.set_base_url(Some(url));
    }
    if let Some(username) = env_var_or_none("SPLUNK_USERNAME") {
        loader.set_username(Some(username));
    }
    if let Some(password) = env_var_or_none("SPLUNK_PASSWORD") {
        loader.set_password(Some(SecretString::new(password.into())));
    }
    if let Some(token) = env_var_or_none("SPLUNK_API_TOKEN") {
        loader.set_api_token(Some(SecretString::new(token.into())));
    }
    if let Some(skip) = env_var_or_none("SPLUNK_SKIP_VERIFY") {
        loader.set_skip_verify(Some(skip.parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_SKIP_VERIFY".to_string(),
                message: "must be true or false".to_string(),
            }
        })?));
    }
    if let Some(timeout) = env_var_or_none("SPLUNK_TIMEOUT") {
        let secs: u64 = timeout.parse().map_err(|_| ConfigError::InvalidValue {
            var: "SPLUNK_TIMEOUT".to_string(),
            message: "must be a number".to_string(),
        })?;
        loader.set_timeout(Some(Duration::from_secs(secs)));
    }
    if let Some(retries) = env_var_or_none("SPLUNK_MAX_RETRIES") {
        let value: usize = retries.parse().map_err(|_| ConfigError::InvalidValue {
            var: "SPLUNK_MAX_RETRIES".to_string(),
            message: "must be a non-negative integer".to_string(),
        })?;
        if value > MAX_MAX_RETRIES {
            return Err(ConfigError::InvalidMaxRetries {
                message: format!("must be between 0 and {} (got {})", MAX_MAX_RETRIES, value),
            });
        }
        loader.set_max_retries(Some(value));
    }
    if let Some(buffer) = env_var_or_none("SPLUNK_SESSION_EXPIRY_BUFFER") {
        loader.set_session_expiry_buffer_seconds(Some(buffer.parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_SESSION_EXPIRY_BUFFER".to_string(),
                message: "must be a number".to_string(),
            }
        })?));
    }
    if let Some(ttl) = env_var_or_none("SPLUNK_SESSION_TTL") {
        loader.set_session_ttl_seconds(Some(ttl.parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_SESSION_TTL".to_string(),
                message: "must be a number".to_string(),
            }
        })?));
    }
    if let Some(interval) = env_var_or_none("SPLUNK_HEALTH_CHECK_INTERVAL") {
        loader.set_health_check_interval_seconds(Some(interval.parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_HEALTH_CHECK_INTERVAL".to_string(),
                message: "must be a number".to_string(),
            }
        })?));
    }
    if let Some(enabled) = env_var_or_none("SPLUNK_CIRCUIT_BREAKER_ENABLED") {
        loader.set_circuit_breaker_enabled(Some(enabled.parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_CIRCUIT_BREAKER_ENABLED".to_string(),
                message: "must be true or false".to_string(),
            }
        })?));
    }
    if let Some(threshold) = env_var_or_none("SPLUNK_CIRCUIT_FAILURE_THRESHOLD") {
        loader.set_circuit_failure_threshold(Some(threshold.parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_CIRCUIT_FAILURE_THRESHOLD".to_string(),
                message: "must be a number".to_string(),
            }
        })?));
    }
    if let Some(window) = env_var_or_none("SPLUNK_CIRCUIT_FAILURE_WINDOW") {
        loader.set_circuit_failure_window_seconds(Some(window.parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_CIRCUIT_FAILURE_WINDOW".to_string(),
                message: "must be a number".to_string(),
            }
        })?));
    }
    if let Some(timeout) = env_var_or_none("SPLUNK_CIRCUIT_RESET_TIMEOUT") {
        loader.set_circuit_reset_timeout_seconds(Some(timeout.parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_CIRCUIT_RESET_TIMEOUT".to_string(),
                message: "must be a number".to_string(),
            }
        })?));
    }
    if let Some(requests) = env_var_or_none("SPLUNK_CIRCUIT_HALF_OPEN_REQUESTS") {
        loader.set_circuit_half_open_requests(Some(requests.parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_CIRCUIT_HALF_OPEN_REQUESTS".to_string(),
                message: "must be a number".to_string(),
            }
        })?));
    }
    // Search defaults
    if let Some(earliest) = env_var_or_none("SPLUNK_EARLIEST_TIME") {
        loader.set_earliest_time(Some(earliest));
    }
    if let Some(latest) = env_var_or_none("SPLUNK_LATEST_TIME") {
        loader.set_latest_time(Some(latest));
    }
    if let Some(max_results) = env_var_or_none("SPLUNK_MAX_RESULTS") {
        loader.set_max_results(Some(max_results.parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_MAX_RESULTS".to_string(),
                message: "must be a positive number".to_string(),
            }
        })?));
    }
    // Internal logs defaults
    if let Some(count) = env_var_or_none("SPLUNK_INTERNAL_LOGS_COUNT") {
        loader.set_internal_logs_count(Some(count.parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_INTERNAL_LOGS_COUNT".to_string(),
                message: "must be a positive number".to_string(),
            }
        })?));
    }
    if let Some(earliest) = env_var_or_none("SPLUNK_INTERNAL_LOGS_EARLIEST") {
        loader.set_internal_logs_earliest(Some(earliest));
    }

    // Config path and profile name from environment (only if not already set via CLI)
    if loader.config_path().is_none() {
        if let Some(config_path) = env_var_or_none("SPLUNK_CONFIG_PATH") {
            loader.set_config_path(Some(std::path::PathBuf::from(config_path)));
        }
    }
    if loader.profile_name().is_none() {
        if let Some(profile) = env_var_or_none("SPLUNK_PROFILE") {
            loader.set_profile_name(Some(profile));
        }
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serial_test::serial;

    #[test]
    #[serial]
    fn test_env_var_or_none_filters_empty_and_whitespace_strings() {
        // Test 1: Unset env var returns None
        let key1 = "_SPLUNK_TEST_UNSET_VAR";
        let result1 = env_var_or_none(key1);
        assert!(result1.is_none(), "Unset env var should return None");

        // Test 2: Empty string env var returns None
        temp_env::with_vars([(key1, Some(""))], || {
            let result2 = env_var_or_none(key1);
            assert!(result2.is_none(), "Empty string env var should return None");
        });

        // Test 3: Whitespace-only string env var returns None
        temp_env::with_vars([(key1, Some("   "))], || {
            let result3 = env_var_or_none(key1);
            assert!(
                result3.is_none(),
                "Whitespace-only env var should return None"
            );
        });

        // Test 4: Non-empty string env var returns Some(trimmed value)
        let key2 = "_SPLUNK_TEST_SET_VAR";
        temp_env::with_vars([(key2, Some(" test-value "))], || {
            let result4 = env_var_or_none(key2);
            assert_eq!(
                result4,
                Some("test-value".to_string()), // Value is now trimmed
                "Non-empty env var should return Some(trimmed value)"
            );
        });
    }
}
