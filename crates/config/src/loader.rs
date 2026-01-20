//! Configuration loader for environment variables and files.

use secrecy::SecretString;
use std::time::Duration;
use thiserror::Error;

use crate::types::{AuthConfig, AuthStrategy, Config, ConnectionConfig};

/// Errors that can occur during configuration loading.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid value for {var}: {message}")]
    InvalidValue { var: String, message: String },

    #[error("Base URL is required")]
    MissingBaseUrl,

    #[error("Authentication configuration is required (either username/password or API token)")]
    MissingAuth,

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

/// Configuration loader that builds config from environment variables.
pub struct ConfigLoader {
    base_url: Option<String>,
    username: Option<String>,
    password: Option<SecretString>,
    api_token: Option<SecretString>,
    skip_verify: Option<bool>,
    timeout: Option<Duration>,
    max_retries: Option<usize>,
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

impl ConfigLoader {
    /// Create a new configuration loader.
    pub fn new() -> Self {
        Self {
            base_url: None,
            username: None,
            password: None,
            api_token: None,
            skip_verify: None,
            timeout: None,
            max_retries: None,
        }
    }

    /// Load environment variables from .env file if present.
    pub fn load_dotenv(self) -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();
        Ok(self)
    }

    /// Read configuration from environment variables.
    pub fn from_env(mut self) -> Result<Self, ConfigError> {
        if let Ok(url) = std::env::var("SPLUNK_BASE_URL") {
            self.base_url = Some(url);
        }
        if let Ok(username) = std::env::var("SPLUNK_USERNAME") {
            self.username = Some(username);
        }
        if let Ok(password) = std::env::var("SPLUNK_PASSWORD") {
            self.password = Some(SecretString::new(password.into()));
        }
        if let Ok(token) = std::env::var("SPLUNK_API_TOKEN") {
            self.api_token = Some(SecretString::new(token.into()));
        }
        if let Ok(skip) = std::env::var("SPLUNK_SKIP_VERIFY") {
            self.skip_verify = Some(skip.parse().map_err(|_| ConfigError::InvalidValue {
                var: "SPLUNK_SKIP_VERIFY".to_string(),
                message: "must be true or false".to_string(),
            })?);
        }
        if let Ok(timeout) = std::env::var("SPLUNK_TIMEOUT") {
            let secs: u64 = timeout.parse().map_err(|_| ConfigError::InvalidValue {
                var: "SPLUNK_TIMEOUT".to_string(),
                message: "must be a number".to_string(),
            })?;
            self.timeout = Some(Duration::from_secs(secs));
        }
        if let Ok(retries) = std::env::var("SPLUNK_MAX_RETRIES") {
            self.max_retries = Some(retries.parse().map_err(|_| ConfigError::InvalidValue {
                var: "SPLUNK_MAX_RETRIES".to_string(),
                message: "must be a number".to_string(),
            })?);
        }
        Ok(self)
    }

    /// Set the base URL.
    pub fn with_base_url(mut self, url: String) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Set the username.
    pub fn with_username(mut self, username: String) -> Self {
        self.username = Some(username);
        self
    }

    /// Set the password.
    pub fn with_password(mut self, password: String) -> Self {
        self.password = Some(SecretString::new(password.into()));
        self
    }

    /// Set the API token.
    pub fn with_api_token(mut self, token: String) -> Self {
        self.api_token = Some(SecretString::new(token.into()));
        self
    }

    /// Set whether to skip TLS verification.
    pub fn with_skip_verify(mut self, skip: bool) -> Self {
        self.skip_verify = Some(skip);
        self
    }

    /// Set the connection timeout.
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = Some(timeout);
        self
    }

    /// Set the maximum number of retries.
    pub fn with_max_retries(mut self, retries: usize) -> Self {
        self.max_retries = Some(retries);
        self
    }

    /// Build the final configuration.
    pub fn build(self) -> Result<Config, ConfigError> {
        let base_url = self.base_url.ok_or(ConfigError::MissingBaseUrl)?;

        // Determine auth strategy - API token takes precedence
        let strategy = if let Some(token) = self.api_token {
            AuthStrategy::ApiToken { token }
        } else if let (Some(username), Some(password)) = (self.username, self.password) {
            AuthStrategy::SessionToken { username, password }
        } else {
            return Err(ConfigError::MissingAuth);
        };

        Ok(Config {
            connection: ConnectionConfig {
                base_url,
                skip_verify: self.skip_verify.unwrap_or(false),
                timeout: self.timeout.unwrap_or(Duration::from_secs(30)),
                max_retries: self.max_retries.unwrap_or(3),
            },
            auth: AuthConfig { strategy },
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
}
