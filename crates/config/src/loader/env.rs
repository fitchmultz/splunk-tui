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
//! Invariants / Assumptions:
//! - Environment variables take precedence over profile settings.
//! - Empty or whitespace-only environment variables are treated as unset.
//! - Invalid numeric values return ConfigError::InvalidValue.

use secrecy::SecretString;
use std::time::Duration;

use super::builder::ConfigLoader;
use super::error::ConfigError;

/// Read an environment variable, returning None if unset, empty, or whitespace-only.
pub fn env_var_or_none(key: &str) -> Option<String> {
    std::env::var(key).ok().filter(|s| !s.trim().is_empty())
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
        loader.set_skip_verify(Some(skip.trim().parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_SKIP_VERIFY".to_string(),
                message: "must be true or false".to_string(),
            }
        })?));
    }
    if let Some(timeout) = env_var_or_none("SPLUNK_TIMEOUT") {
        let secs: u64 = timeout
            .trim()
            .parse()
            .map_err(|_| ConfigError::InvalidValue {
                var: "SPLUNK_TIMEOUT".to_string(),
                message: "must be a number".to_string(),
            })?;
        loader.set_timeout(Some(Duration::from_secs(secs)));
    }
    if let Some(retries) = env_var_or_none("SPLUNK_MAX_RETRIES") {
        loader.set_max_retries(Some(retries.trim().parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_MAX_RETRIES".to_string(),
                message: "must be a number".to_string(),
            }
        })?));
    }
    if let Some(buffer) = env_var_or_none("SPLUNK_SESSION_EXPIRY_BUFFER") {
        loader.set_session_expiry_buffer_seconds(Some(buffer.trim().parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_SESSION_EXPIRY_BUFFER".to_string(),
                message: "must be a number".to_string(),
            }
        })?));
    }
    if let Some(ttl) = env_var_or_none("SPLUNK_SESSION_TTL") {
        loader.set_session_ttl_seconds(Some(ttl.trim().parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_SESSION_TTL".to_string(),
                message: "must be a number".to_string(),
            }
        })?));
    }
    if let Some(interval) = env_var_or_none("SPLUNK_HEALTH_CHECK_INTERVAL") {
        loader.set_health_check_interval_seconds(Some(interval.trim().parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_HEALTH_CHECK_INTERVAL".to_string(),
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
        loader.set_max_results(Some(max_results.trim().parse().map_err(|_| {
            ConfigError::InvalidValue {
                var: "SPLUNK_MAX_RESULTS".to_string(),
                message: "must be a positive number".to_string(),
            }
        })?));
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

        // Test 4: Non-empty string env var returns Some(value without trimming)
        let key2 = "_SPLUNK_TEST_SET_VAR";
        temp_env::with_vars([(key2, Some(" test-value "))], || {
            let result4 = env_var_or_none(key2);
            assert_eq!(
                result4,
                Some(" test-value ".to_string()), // Implementation doesn't trim the value, just checks if trimmed is empty
                "Non-empty env var should return Some(value)"
            );
        });
    }
}
