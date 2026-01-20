//! Configuration types for Splunk TUI.

use secrecy::SecretString;
use std::time::Duration;

/// Strategy for authenticating with Splunk.
#[derive(Debug, Clone)]
pub enum AuthStrategy {
    /// Username and password authentication (creates session token)
    SessionToken {
        username: String,
        password: SecretString,
    },
    /// API token (bearer token authentication)
    ApiToken { token: SecretString },
}

/// Authentication configuration.
#[derive(Debug, Clone)]
pub struct AuthConfig {
    /// The authentication strategy to use.
    pub strategy: AuthStrategy,
}

/// Connection configuration for Splunk server.
#[derive(Debug, Clone)]
pub struct ConnectionConfig {
    /// Base URL of the Splunk server (e.g., https://localhost:8089)
    pub base_url: String,
    /// Whether to skip TLS verification (for self-signed certificates)
    pub skip_verify: bool,
    /// Connection timeout
    pub timeout: Duration,
    /// Maximum number of retries for failed requests
    pub max_retries: usize,
}

/// Main configuration structure.
#[derive(Debug, Clone)]
pub struct Config {
    /// Connection settings
    pub connection: ConnectionConfig,
    /// Authentication settings
    pub auth: AuthConfig,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            connection: ConnectionConfig {
                base_url: "https://localhost:8089".to_string(),
                skip_verify: false,
                timeout: Duration::from_secs(30),
                max_retries: 3,
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
    /// Create a new config with the specified base URL and API token.
    pub fn with_api_token(base_url: String, token: SecretString) -> Self {
        Self {
            connection: ConnectionConfig {
                base_url,
                skip_verify: false,
                timeout: Duration::from_secs(30),
                max_retries: 3,
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
                timeout: Duration::from_secs(30),
                max_retries: 3,
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
}
