//! Client builder for constructing [`SplunkClient`] instances.
//!
//! This module is responsible for:
//! - Providing a fluent builder API for client configuration
//! - Validating required configuration (base_url, auth_strategy)
//! - Normalizing the base URL (removing trailing slashes)
//! - Configuring the underlying HTTP client (timeouts, TLS verification)
//!
//! # What this module does NOT handle:
//! - Actual API calls (handled by [`SplunkClient`] methods in `mod.rs`)
//! - Session token management (handled by [`SessionManager`] in `auth.rs`)
//! - Retry logic for failed requests (handled by the `retry_call!` macro)
//!
//! # Invariants
//! - `base_url` and `auth_strategy` are required fields and must be provided before calling `build()`
//! - The base URL is always normalized to have no trailing slashes
//! - `skip_verify` only affects HTTPS connections; HTTP connections log a warning

use std::time::Duration;

use crate::auth::{AuthStrategy, SessionManager};
use crate::client::SplunkClient;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use splunk_config::{
    AuthStrategy as ConfigAuthStrategy, Config,
    constants::{
        DEFAULT_EXPIRY_BUFFER_SECS, DEFAULT_MAX_REDIRECTS, DEFAULT_MAX_RETRIES,
        DEFAULT_SESSION_TTL_SECS, DEFAULT_TIMEOUT_SECS,
    },
};

/// Builder for creating a new [`SplunkClient`].
///
/// This builder provides a fluent API for configuring the Splunk client
/// before instantiation. All configuration options have sensible defaults
/// except for `base_url` and `auth_strategy`, which are required.
///
/// # Example
///
/// ```rust,ignore
/// use splunk_client::{SplunkClient, AuthStrategy};
/// use secrecy::SecretString;
///
/// let client = SplunkClient::builder()
///     .base_url("https://localhost:8089".to_string())
///     .auth_strategy(AuthStrategy::ApiToken {
///         token: SecretString::new("my-token".to_string().into()),
///     })
///     .timeout(Duration::from_secs(60))
///     .build()?;
/// ```
pub struct SplunkClientBuilder {
    base_url: Option<String>,
    auth_strategy: Option<AuthStrategy>,
    skip_verify: bool,
    timeout: Duration,
    max_retries: usize,
    session_ttl_seconds: u64,
    session_expiry_buffer_seconds: u64,
    metrics: Option<MetricsCollector>,
}

impl Default for SplunkClientBuilder {
    fn default() -> Self {
        Self {
            base_url: None,
            auth_strategy: None,
            skip_verify: false,
            timeout: Duration::from_secs(DEFAULT_TIMEOUT_SECS),
            max_retries: DEFAULT_MAX_RETRIES,
            session_ttl_seconds: DEFAULT_SESSION_TTL_SECS,
            session_expiry_buffer_seconds: DEFAULT_EXPIRY_BUFFER_SECS,
            metrics: None,
        }
    }
}

impl SplunkClientBuilder {
    /// Create a new builder with default settings.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base URL of the Splunk server.
    ///
    /// This should include the protocol and port, e.g., `https://localhost:8089`.
    /// Trailing slashes will be automatically removed.
    pub fn base_url(mut self, url: String) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Set the authentication strategy.
    ///
    /// See [`AuthStrategy`] for available options.
    pub fn auth_strategy(mut self, strategy: AuthStrategy) -> Self {
        self.auth_strategy = Some(strategy);
        self
    }

    /// Set whether to skip TLS certificate verification.
    ///
    /// # Security Warning
    /// Only use this in development or testing environments. Disabling TLS
    /// verification makes the connection vulnerable to man-in-the-middle attacks.
    ///
    /// # Note
    /// This only affects HTTPS connections. For HTTP URLs, a warning is logged
    /// but no error occurs.
    pub fn skip_verify(mut self, skip: bool) -> Self {
        self.skip_verify = skip;
        self
    }

    /// Set the request timeout.
    ///
    /// Default is 30 seconds.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries for failed requests.
    ///
    /// Default is 3 retries with exponential backoff (1s, 2s, 4s delays).
    pub fn max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set the session TTL in seconds.
    ///
    /// This determines how long session tokens are considered valid before
    /// proactive refresh. Default is 3600 seconds (1 hour).
    pub fn session_ttl_seconds(mut self, ttl: u64) -> Self {
        self.session_ttl_seconds = ttl;
        self
    }

    /// Set the session expiry buffer in seconds.
    ///
    /// Sessions will be proactively refreshed if they expire within this
    /// buffer window. This prevents race conditions where a token expires
    /// during an API call. Default is 60 seconds.
    pub fn session_expiry_buffer_seconds(mut self, buffer: u64) -> Self {
        self.session_expiry_buffer_seconds = buffer;
        self
    }

    /// Set the metrics collector for API call performance tracking.
    ///
    /// When set, the client will record metrics for:
    /// - Request latency histograms
    /// - Request counters (total, retries, errors)
    /// - Error categorization
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use splunk_client::{SplunkClient, MetricsCollector};
    ///
    /// let metrics = MetricsCollector::new();
    /// let client = SplunkClient::builder()
    ///     .base_url("https://localhost:8089".to_string())
    ///     .auth_strategy(auth_strategy)
    ///     .metrics(metrics)
    ///     .build()?;
    /// ```
    pub fn metrics(mut self, metrics: MetricsCollector) -> Self {
        self.metrics = Some(metrics);
        self
    }

    /// Create a client builder from configuration.
    ///
    /// This method centralizes the conversion from config crate types to client crate types,
    /// eliminating duplication between CLI and TUI.
    ///
    /// # Arguments
    ///
    /// * `config` - The loaded configuration containing connection and auth settings
    ///
    /// # Returns
    ///
    /// A `SplunkClientBuilder` pre-configured with settings from the config
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use splunk_client::SplunkClient;
    /// use splunk_config::Config;
    ///
    /// let config = Config::default();
    /// let client = SplunkClient::builder()
    ///     .from_config(&config)
    ///     .build()?;
    /// ```
    pub fn from_config(mut self, config: &Config) -> Self {
        let auth_strategy = match &config.auth.strategy {
            ConfigAuthStrategy::SessionToken { username, password } => AuthStrategy::SessionToken {
                username: username.clone(),
                password: password.clone(),
            },
            ConfigAuthStrategy::ApiToken { token } => AuthStrategy::ApiToken {
                token: token.clone(),
            },
        };

        self.base_url = Some(config.connection.base_url.clone());
        self.auth_strategy = Some(auth_strategy);
        self.skip_verify = config.connection.skip_verify;
        self.timeout = config.connection.timeout;
        self.session_ttl_seconds = config.connection.session_ttl_seconds;
        self.session_expiry_buffer_seconds = config.connection.session_expiry_buffer_seconds;
        self
    }

    /// Normalize a base URL by removing trailing slashes.
    ///
    /// This prevents double slashes when concatenating with endpoint paths.
    ///
    /// # Examples
    ///
    /// - `"https://localhost:8089/"` -> `"https://localhost:8089"`
    /// - `"https://localhost:8089"` -> `"https://localhost:8089"`
    /// - `"https://example.com:8089//"` -> `"https://example.com:8089"`
    fn normalize_base_url(url: String) -> String {
        url.trim_end_matches('/').to_string()
    }

    /// Build the [`SplunkClient`] with the configured options.
    ///
    /// # Errors
    ///
    /// Returns [`ClientError::InvalidUrl`] if `base_url` was not provided.
    /// Returns [`ClientError::AuthFailed`] if `auth_strategy` was not provided.
    /// Returns `ClientError::HttpError` if the HTTP client fails to build.
    pub fn build(self) -> Result<SplunkClient> {
        let base_url = self
            .base_url
            .ok_or_else(|| ClientError::InvalidUrl("base_url is required".to_string()))?;
        let base_url = Self::normalize_base_url(base_url);

        let auth_strategy = self
            .auth_strategy
            .ok_or_else(|| ClientError::AuthFailed("auth_strategy is required".to_string()))?;

        let mut http_builder = reqwest::Client::builder()
            .timeout(self.timeout)
            .redirect(reqwest::redirect::Policy::limited(DEFAULT_MAX_REDIRECTS));

        if self.skip_verify {
            let is_https = base_url.starts_with("https://");
            if is_https {
                http_builder = http_builder.danger_accept_invalid_certs(true);
            } else {
                // skip_verify only affects TLS certificate verification.
                // It has no effect on HTTP connections since there is no TLS layer.
                tracing::warn!(
                    "skip_verify=true has no effect on HTTP URLs. TLS verification only applies to HTTPS connections."
                );
            }
        }

        let http = http_builder.build()?;

        Ok(SplunkClient {
            http,
            base_url,
            session_manager: SessionManager::new(auth_strategy),
            max_retries: self.max_retries,
            session_ttl_seconds: self.session_ttl_seconds,
            session_expiry_buffer_seconds: self.session_expiry_buffer_seconds,
            metrics: self.metrics,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn test_from_config_with_api_token() {
        let config = Config::with_api_token(
            "https://splunk.example.com:8089".to_string(),
            SecretString::new("test-token".to_string().into()),
        );

        let client = SplunkClient::builder().from_config(&config).build();

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url(), "https://splunk.example.com:8089");
        assert!(client.is_api_token_auth());
    }

    #[test]
    fn test_from_config_with_session_token() {
        let config = Config::with_session_token(
            "https://splunk.example.com:8089".to_string(),
            "admin".to_string(),
            SecretString::new("test-password".to_string().into()),
        );

        let client = SplunkClient::builder().from_config(&config).build();

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url(), "https://splunk.example.com:8089");
        assert!(!client.is_api_token_auth());
    }

    #[test]
    fn test_from_config_preserves_settings() {
        let mut config = Config::with_api_token(
            "https://splunk.example.com:8089".to_string(),
            SecretString::new("test-token".to_string().into()),
        );
        config.connection.skip_verify = true;
        config.connection.timeout = std::time::Duration::from_secs(120);
        config.connection.session_ttl_seconds = 7200;
        config.connection.session_expiry_buffer_seconds = 120;

        let builder = SplunkClient::builder().from_config(&config);

        assert_eq!(
            builder.base_url,
            Some("https://splunk.example.com:8089".to_string())
        );
        assert!(builder.skip_verify);
        assert_eq!(builder.timeout, std::time::Duration::from_secs(120));
        assert_eq!(builder.session_ttl_seconds, 7200);
        assert_eq!(builder.session_expiry_buffer_seconds, 120);
    }

    #[test]
    fn test_normalize_base_url_trailing_slash() {
        let input = "https://localhost:8089/".to_string();
        let expected = "https://localhost:8089";
        assert_eq!(SplunkClientBuilder::normalize_base_url(input), expected);
    }

    #[test]
    fn test_normalize_base_url_no_trailing_slash() {
        let input = "https://localhost:8089".to_string();
        let expected = "https://localhost:8089";
        assert_eq!(SplunkClientBuilder::normalize_base_url(input), expected);
    }

    #[test]
    fn test_normalize_base_url_multiple_trailing_slashes() {
        let input = "https://example.com:8089//".to_string();
        let expected = "https://example.com:8089";
        assert_eq!(SplunkClientBuilder::normalize_base_url(input), expected);
    }
}
