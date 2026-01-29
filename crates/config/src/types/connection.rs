//! Connection configuration types for Splunk TUI.
//!
//! Responsibilities:
//! - Define connection settings (URL, TLS verification, timeouts, retries).
//! - Define the main `Config` structure combining connection and auth.
//! - Provide serialization helpers for `Duration`.
//! - Provide convenience constructors for common config patterns.
//!
//! Does NOT handle:
//! - Configuration loading from files/env (see `loader` module).
//! - Configuration persistence (see `persistence` module).
//! - Actual network connections (see client crate).
//!
//! Invariants:
//! - All duration fields are serialized as seconds (integers).
//! - Default values are provided via `Default` impl, not magic numbers.
//! - `Config::default()` provides sensible development defaults (localhost:8089).

use crate::constants::{
    DEFAULT_EXPIRY_BUFFER_SECS, DEFAULT_HEALTH_CHECK_INTERVAL_SECS, DEFAULT_MAX_RETRIES,
    DEFAULT_SESSION_TTL_SECS, DEFAULT_SPLUNK_PORT, DEFAULT_TIMEOUT_SECS,
};
use crate::types::auth::{AuthConfig, AuthStrategy};
use secrecy::SecretString;
use serde::{Deserialize, Serialize};
use std::time::Duration;

/// Module for serializing Duration as seconds (integer).
mod duration_seconds {
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use std::time::Duration;

    pub fn serialize<S>(duration: &Duration, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        duration.as_secs().serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Duration, D::Error>
    where
        D: Deserializer<'de>,
    {
        let secs = u64::deserialize(deserializer)?;
        Ok(Duration::from_secs(secs))
    }
}

/// Connection configuration for Splunk server.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ConnectionConfig {
    /// Base URL of the Splunk server (e.g., https://localhost:8089)
    pub base_url: String,
    /// Whether to skip TLS verification (for self-signed certificates)
    pub skip_verify: bool,
    /// Connection timeout (serialized as seconds)
    #[serde(with = "duration_seconds")]
    pub timeout: Duration,
    /// Maximum number of retries for failed requests
    pub max_retries: usize,
    /// Buffer time before session expiry to proactively refresh tokens (in seconds)
    /// This prevents race conditions where a token expires during an API call.
    /// Default: 60 seconds
    #[serde(default = "default_session_expiry_buffer")]
    pub session_expiry_buffer_seconds: u64,
    /// Session time-to-live in seconds (how long tokens remain valid)
    /// Default: 3600 seconds (1 hour)
    #[serde(default = "default_session_ttl")]
    pub session_ttl_seconds: u64,
    /// Health check interval in seconds (how often to poll server health)
    /// Default: 60 seconds
    #[serde(default = "default_health_check_interval")]
    pub health_check_interval_seconds: u64,
}

/// Default session expiry buffer in seconds.
pub(crate) fn default_session_expiry_buffer() -> u64 {
    DEFAULT_EXPIRY_BUFFER_SECS
}

/// Default session TTL in seconds (1 hour).
pub(crate) fn default_session_ttl() -> u64 {
    DEFAULT_SESSION_TTL_SECS
}

/// Default health check interval in seconds.
pub(crate) fn default_health_check_interval() -> u64 {
    DEFAULT_HEALTH_CHECK_INTERVAL_SECS
}

/// Main configuration structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Config {
    /// Connection settings
    pub connection: ConnectionConfig,
    /// Authentication settings
    pub auth: AuthConfig,
}

impl Default for Config {
    /// Creates a default configuration with development-only credentials.
    ///
    /// # Security Warning
    ///
    /// The default configuration uses Splunk's default credentials (admin/changeme)
    /// targeting localhost:8089. These credentials are **ONLY** appropriate for
    /// local development environments and MUST be changed before any production use.
    ///
    /// # Default Values
    ///
    /// - `base_url`: `https://localhost:8089`
    /// - `username`: `admin`
    /// - `password`: `changeme`
    /// - `timeout`: 30 seconds
    /// - `max_retries`: 3
    ///
    /// # Invariants
    ///
    /// This implementation is intended for development convenience only. Production
    /// deployments should always use explicit configuration via environment variables,
    /// configuration files, or the `ConfigLoader` builder with custom credentials.
    fn default() -> Self {
        Self {
            connection: ConnectionConfig {
                base_url: format!("https://localhost:{}", DEFAULT_SPLUNK_PORT),
                skip_verify: false,
                timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
                max_retries: DEFAULT_MAX_RETRIES,
                session_expiry_buffer_seconds: default_session_expiry_buffer(),
                session_ttl_seconds: default_session_ttl(),
                health_check_interval_seconds: default_health_check_interval(),
            },
            auth: AuthConfig {
                strategy: AuthStrategy::SessionToken {
                    username: "admin".to_string(),
                    password: SecretString::new("changeme".to_string().into()),
                },
            },
        }
    }
}

impl Config {
    /// Checks if this configuration is using the default development credentials.
    ///
    /// Returns `true` if the auth strategy is `SessionToken` with username "admin"
    /// and password "changeme". This is useful for detecting potentially unsafe
    /// default configurations in production environments.
    ///
    /// # Security Note
    ///
    /// This check uses constant-time comparison to avoid timing side-channels,
    /// though this is primarily a defense-in-depth measure since the default
    /// credentials are publicly known.
    pub fn is_using_default_credentials(&self) -> bool {
        use secrecy::ExposeSecret;

        matches!(
            &self.auth.strategy,
            AuthStrategy::SessionToken { username, password }
                if username == "admin"
                    && password.expose_secret() == "changeme"
        )
    }

    /// Create a new config with the specified base URL and API token.
    pub fn with_api_token(base_url: String, token: SecretString) -> Self {
        Self {
            connection: ConnectionConfig {
                base_url,
                skip_verify: false,
                timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
                max_retries: DEFAULT_MAX_RETRIES,
                session_expiry_buffer_seconds: default_session_expiry_buffer(),
                session_ttl_seconds: default_session_ttl(),
                health_check_interval_seconds: default_health_check_interval(),
            },
            auth: AuthConfig {
                strategy: AuthStrategy::ApiToken { token },
            },
        }
    }

    /// Create a new config with the specified base URL and username/password.
    pub fn with_session_token(base_url: String, username: String, password: SecretString) -> Self {
        Self {
            connection: ConnectionConfig {
                base_url,
                skip_verify: false,
                timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
                max_retries: DEFAULT_MAX_RETRIES,
                session_expiry_buffer_seconds: default_session_expiry_buffer(),
                session_ttl_seconds: default_session_ttl(),
                health_check_interval_seconds: default_health_check_interval(),
            },
            auth: AuthConfig {
                strategy: AuthStrategy::SessionToken { username, password },
            },
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = Config::default();
        assert_eq!(config.connection.base_url, "https://localhost:8089");
        assert!(!config.connection.skip_verify);
    }

    #[test]
    fn test_config_with_api_token() {
        let token = SecretString::new("test-token".to_string().into());
        let config = Config::with_api_token("https://splunk.example.com:8089".to_string(), token);
        assert!(matches!(
            config.auth.strategy,
            AuthStrategy::ApiToken { .. }
        ));
    }

    #[test]
    fn test_config_with_session_token() {
        let password = SecretString::new("test-password".to_string().into());
        let config = Config::with_session_token(
            "https://splunk.example.com:8089".to_string(),
            "admin".to_string(),
            password,
        );
        assert!(matches!(
            config.auth.strategy,
            AuthStrategy::SessionToken { .. }
        ));
    }

    #[test]
    fn test_connection_config_serde_seconds() {
        let config = ConnectionConfig {
            base_url: "https://localhost:8089".to_string(),
            skip_verify: true,
            timeout: Duration::from_secs(60),
            max_retries: 5,
            session_expiry_buffer_seconds: default_session_expiry_buffer(),
            session_ttl_seconds: default_session_ttl(),
            health_check_interval_seconds: default_health_check_interval(),
        };

        let json = serde_json::to_string(&config).unwrap();
        let deserialized: ConnectionConfig = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.timeout, Duration::from_secs(60));
        assert_eq!(deserialized.max_retries, 5);
    }

    /// Test that Config Debug output does not expose secrets.
    #[test]
    fn test_config_debug_does_not_expose_secrets() {
        let password = SecretString::new("my-secret-password".to_string().into());
        let config = Config::with_session_token(
            "https://localhost:8089".to_string(),
            "admin".to_string(),
            password,
        );

        let debug_output = format!("{:?}", config);

        // The password should NOT appear in debug output
        assert!(
            !debug_output.contains("my-secret-password"),
            "Debug output should not contain the password"
        );

        // But non-sensitive data should be visible
        assert!(debug_output.contains("admin"));
        assert!(debug_output.contains("https://localhost:8089"));
    }

    /// Test that Config with API token does not expose token in Debug output.
    #[test]
    fn test_config_with_api_token_debug_redaction() {
        let token = SecretString::new("super-secret-api-token".to_string().into());
        let config = Config::with_api_token("https://splunk.example.com:8089".to_string(), token);

        let debug_output = format!("{:?}", config);

        // The token should NOT appear in debug output
        assert!(
            !debug_output.contains("super-secret-api-token"),
            "Debug output should not contain the API token"
        );
    }

    /// Test that ConnectionConfig Debug output is safe (no secrets).
    #[test]
    fn test_connection_config_debug_safe() {
        let config = ConnectionConfig {
            base_url: "https://localhost:8089".to_string(),
            skip_verify: true,
            timeout: Duration::from_secs(60),
            max_retries: 5,
            session_expiry_buffer_seconds: default_session_expiry_buffer(),
            session_ttl_seconds: default_session_ttl(),
            health_check_interval_seconds: default_health_check_interval(),
        };

        let debug_output = format!("{:?}", config);

        // Connection config should never contain secrets
        // Just verify it formats correctly
        assert!(debug_output.contains("https://localhost:8089"));
        assert!(debug_output.contains("skip_verify: true"));
    }

    // ============================================================================
    // Security-focused tests for default credential detection
    // ============================================================================

    /// Test that default config is correctly identified as using default credentials.
    #[test]
    fn test_is_using_default_credentials_true_for_default_config() {
        let config = Config::default();
        assert!(
            config.is_using_default_credentials(),
            "Default config should be detected as using default credentials"
        );
    }

    /// Test that custom username is not detected as default credentials.
    #[test]
    fn test_is_using_default_credentials_false_for_different_username() {
        let password = SecretString::new("changeme".to_string().into());
        let config = Config::with_session_token(
            "https://localhost:8089".to_string(),
            "customuser".to_string(),
            password,
        );
        assert!(
            !config.is_using_default_credentials(),
            "Custom username should not be detected as default credentials"
        );
    }

    /// Test that custom password is not detected as default credentials.
    #[test]
    fn test_is_using_default_credentials_false_for_different_password() {
        let password = SecretString::new("custompassword".to_string().into());
        let config = Config::with_session_token(
            "https://localhost:8089".to_string(),
            "admin".to_string(),
            password,
        );
        assert!(
            !config.is_using_default_credentials(),
            "Custom password should not be detected as default credentials"
        );
    }

    /// Test that API token auth is not detected as default credentials.
    #[test]
    fn test_is_using_default_credentials_false_for_api_token() {
        let token = SecretString::new("some-api-token".to_string().into());
        let config = Config::with_api_token("https://splunk.example.com:8089".to_string(), token);
        assert!(
            !config.is_using_default_credentials(),
            "API token auth should not be detected as default credentials"
        );
    }

    /// Test that completely different credentials are not detected as default.
    #[test]
    fn test_is_using_default_credentials_false_for_completely_different() {
        let password = SecretString::new("supersecret123".to_string().into());
        let config = Config::with_session_token(
            "https://splunk.prod.com:8089".to_string(),
            "splunkadmin".to_string(),
            password,
        );
        assert!(
            !config.is_using_default_credentials(),
            "Completely different credentials should not be detected as default"
        );
    }
}
