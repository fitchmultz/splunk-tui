//! Configuration loader builder implementation.
//!
//! Responsibilities:
//! - Provide a builder-pattern `ConfigLoader` for hierarchical configuration merging.
//! - Support loading from environment variables, profile files, and direct builder methods.
//! - Build the final `Config` and `SearchDefaultConfig` from loaded values.
//!
//! Does NOT handle:
//! - Direct environment variable parsing logic (delegated to env.rs).
//! - Profile file loading logic (delegated to profile.rs).
//! - Persisting configuration changes (see persistence.rs).
//!
//! Invariants / Assumptions:
//! - Environment variables take precedence over profile file values.
//! - Builder methods take precedence over environment variables.
//! - `load_dotenv()` must be called explicitly to enable `.env` file loading.
//! - The `DOTENV_DISABLED` variable is checked before `dotenvy::dotenv()` is called.

use secrecy::SecretString;
use std::path::PathBuf;
use std::time::Duration;

use super::defaults::SearchDefaultConfig;
use super::env::apply_env;
use super::error::ConfigError;
use super::profile::apply_profile;
use crate::constants::{
    DEFAULT_EXPIRY_BUFFER_SECS, DEFAULT_HEALTH_CHECK_INTERVAL_SECS, DEFAULT_MAX_RETRIES,
    DEFAULT_SESSION_TTL_SECS, DEFAULT_TIMEOUT_SECS, MAX_HEALTH_CHECK_INTERVAL_SECS,
    MAX_SESSION_TTL_SECS, MAX_TIMEOUT_SECS,
};
use crate::persistence::SearchDefaults;
use crate::types::{AuthConfig, AuthStrategy, Config, ConnectionConfig};

/// Configuration loader that builds config from environment variables and profiles.
pub struct ConfigLoader {
    base_url: Option<String>,
    username: Option<String>,
    password: Option<SecretString>,
    api_token: Option<SecretString>,
    skip_verify: Option<bool>,
    timeout: Option<Duration>,
    max_retries: Option<usize>,
    session_expiry_buffer_seconds: Option<u64>,
    session_ttl_seconds: Option<u64>,
    health_check_interval_seconds: Option<u64>,
    profile_name: Option<String>,
    profile_missing: Option<String>,
    config_path: Option<PathBuf>,
    earliest_time: Option<String>,
    latest_time: Option<String>,
    max_results: Option<u64>,
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
            session_expiry_buffer_seconds: None,
            session_ttl_seconds: None,
            health_check_interval_seconds: None,
            profile_name: None,
            profile_missing: None,
            config_path: None,
            earliest_time: None,
            latest_time: None,
            max_results: None,
        }
    }

    /// Check if dotenv loading is disabled via environment variable.
    fn dotenv_disabled() -> bool {
        matches!(
            std::env::var("DOTENV_DISABLED").ok().as_deref(),
            Some("true") | Some("1")
        )
    }

    /// Load environment variables from .env file if present.
    ///
    /// If `DOTENV_DISABLED` environment variable is set to "true" or "1",
    /// the .env file will not be loaded (useful for testing).
    ///
    /// # Errors
    ///
    /// Returns an error if:
    /// - The `.env` file exists but has invalid syntax (`ConfigError::DotenvParse`)
    /// - The `.env` file exists but cannot be read due to I/O errors (`ConfigError::DotenvIo`)
    ///
    /// Missing `.env` files are silently ignored (returns `Ok(self)`).
    ///
    /// SAFETY: Error messages never include raw .env line contents to prevent secret leakage.
    pub fn load_dotenv(self) -> Result<Self, ConfigError> {
        if Self::dotenv_disabled() {
            return Ok(self);
        }

        match dotenvy::dotenv() {
            Ok(_) => Ok(self),
            Err(e) if Self::is_not_found(&e) => Ok(self),
            Err(dotenvy::Error::LineParse(_, idx)) => {
                Err(ConfigError::DotenvParse { error_index: idx })
            }
            Err(dotenvy::Error::Io(io_err)) => Err(ConfigError::DotenvIo {
                kind: io_err.kind(),
            }),
            Err(_) => Err(ConfigError::DotenvUnknown),
        }
    }

    /// Check if a dotenv error indicates the file was not found.
    fn is_not_found(err: &dotenvy::Error) -> bool {
        matches!(
            err,
            dotenvy::Error::Io(io_err) if io_err.kind() == std::io::ErrorKind::NotFound
        )
    }

    /// Set the active profile name to load from the config file.
    pub fn with_profile_name(mut self, name: String) -> Self {
        self.profile_name = Some(name);
        self
    }

    /// Override the config file path (primarily for testing).
    pub fn with_config_path(mut self, path: PathBuf) -> Self {
        self.config_path = Some(path);
        self
    }

    /// Read configuration from a profile in the config file.
    ///
    /// If the profile is not found, this records the missing profile name
    /// for later error handling in `build()`.
    pub fn from_profile(mut self) -> Result<Self, ConfigError> {
        apply_profile(&mut self)?;
        Ok(self)
    }

    /// Read configuration from environment variables.
    ///
    /// Environment variables take precedence over profile settings.
    pub fn from_env(mut self) -> Result<Self, ConfigError> {
        apply_env(&mut self)?;
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

    /// Check if we have a complete configuration (base_url + auth).
    ///
    /// A complete configuration requires:
    /// - base_url is present
    /// - AND (api_token is present OR (username AND password are present))
    fn has_complete_config(&self) -> bool {
        // Must have base_url
        let has_base_url = self.base_url.is_some();

        // Must have complete auth: either api_token OR (username AND password)
        let has_complete_auth =
            self.api_token.is_some() || (self.username.is_some() && self.password.is_some());

        has_base_url && has_complete_auth
    }

    /// Build the final configuration.
    pub fn build(self) -> Result<Config, ConfigError> {
        // Check for missing profile first
        if let Some(ref profile_name) = self.profile_missing {
            // Only suppress ProfileNotFound if we have a complete config from env/CLI
            if !self.has_complete_config() {
                return Err(ConfigError::ProfileNotFound(profile_name.clone()));
            }
        }

        let base_url = self
            .base_url
            .as_deref()
            .map(validate_and_normalize_base_url)
            .transpose()?
            .ok_or(ConfigError::MissingBaseUrl)?;

        // Determine auth strategy - API token takes precedence
        let strategy = if let Some(token) = self.api_token {
            AuthStrategy::ApiToken { token }
        } else if let (Some(username), Some(password)) = (self.username, self.password) {
            AuthStrategy::SessionToken { username, password }
        } else {
            return Err(ConfigError::MissingAuth);
        };

        let connection = ConnectionConfig {
            base_url,
            skip_verify: self.skip_verify.unwrap_or(false),
            timeout: self
                .timeout
                .unwrap_or(Duration::from_secs(DEFAULT_TIMEOUT_SECS)),
            max_retries: self.max_retries.unwrap_or(DEFAULT_MAX_RETRIES),
            session_expiry_buffer_seconds: self
                .session_expiry_buffer_seconds
                .unwrap_or(DEFAULT_EXPIRY_BUFFER_SECS),
            session_ttl_seconds: self.session_ttl_seconds.unwrap_or(DEFAULT_SESSION_TTL_SECS),
            health_check_interval_seconds: self
                .health_check_interval_seconds
                .unwrap_or(DEFAULT_HEALTH_CHECK_INTERVAL_SECS),
        };

        // Validate timeout configuration
        Self::validate_timeout_config(&connection)?;

        Ok(Config {
            connection,
            auth: AuthConfig { strategy },
        })
    }

    /// Validates timeout-related configuration values.
    ///
    /// Checks:
    /// - timeout is greater than 0 and not exceeding MAX_TIMEOUT_SECS
    /// - session_ttl_seconds is greater than session_expiry_buffer_seconds
    /// - session_ttl_seconds does not exceed MAX_SESSION_TTL_SECS
    /// - health_check_interval_seconds is greater than 0 and not exceeding MAX_HEALTH_CHECK_INTERVAL_SECS
    fn validate_timeout_config(connection: &ConnectionConfig) -> Result<(), ConfigError> {
        let timeout_secs = connection.timeout.as_secs();

        // Validate timeout > 0
        if timeout_secs == 0 {
            return Err(ConfigError::InvalidTimeout {
                message: "timeout must be greater than 0 seconds".to_string(),
            });
        }

        // Validate timeout <= MAX_TIMEOUT_SECS
        if timeout_secs > MAX_TIMEOUT_SECS {
            return Err(ConfigError::InvalidTimeout {
                message: format!(
                    "timeout exceeds maximum allowed value of {} seconds",
                    MAX_TIMEOUT_SECS
                ),
            });
        }

        // Validate session_ttl_seconds > session_expiry_buffer_seconds
        if connection.session_ttl_seconds <= connection.session_expiry_buffer_seconds {
            return Err(ConfigError::InvalidSessionTtl {
                message: format!(
                    "session_ttl_seconds ({}) must be greater than session_expiry_buffer_seconds ({})",
                    connection.session_ttl_seconds, connection.session_expiry_buffer_seconds
                ),
            });
        }

        // Validate session_ttl_seconds <= MAX_SESSION_TTL_SECS
        if connection.session_ttl_seconds > MAX_SESSION_TTL_SECS {
            return Err(ConfigError::InvalidSessionTtl {
                message: format!(
                    "session_ttl_seconds exceeds maximum allowed value of {} seconds",
                    MAX_SESSION_TTL_SECS
                ),
            });
        }

        // Validate health_check_interval_seconds > 0
        if connection.health_check_interval_seconds == 0 {
            return Err(ConfigError::InvalidHealthCheckInterval {
                message: "health_check_interval_seconds must be greater than 0".to_string(),
            });
        }

        // Validate health_check_interval_seconds <= MAX_HEALTH_CHECK_INTERVAL_SECS
        if connection.health_check_interval_seconds > MAX_HEALTH_CHECK_INTERVAL_SECS {
            return Err(ConfigError::InvalidHealthCheckInterval {
                message: format!(
                    "health_check_interval_seconds exceeds maximum allowed value of {} seconds",
                    MAX_HEALTH_CHECK_INTERVAL_SECS
                ),
            });
        }

        Ok(())
    }

    /// Build the search default configuration from loaded values.
    ///
    /// This uses environment variable values if set, otherwise falls back
    /// to the provided persisted defaults.
    pub fn build_search_defaults(&self, persisted: Option<SearchDefaults>) -> SearchDefaultConfig {
        let defaults = persisted.unwrap_or_default();
        SearchDefaultConfig {
            earliest_time: self.earliest_time.clone().unwrap_or(defaults.earliest_time),
            latest_time: self.latest_time.clone().unwrap_or(defaults.latest_time),
            max_results: self.max_results.unwrap_or(defaults.max_results),
        }
    }

    /// Get the earliest time if set via environment variable.
    pub fn earliest_time(&self) -> Option<&String> {
        self.earliest_time.as_ref()
    }

    /// Get the latest time if set via environment variable.
    pub fn latest_time(&self) -> Option<&String> {
        self.latest_time.as_ref()
    }

    /// Get the max results if set via environment variable.
    pub fn max_results(&self) -> Option<u64> {
        self.max_results
    }

    // Internal accessor methods for use by other loader modules

    pub(crate) fn profile_name(&self) -> Option<&String> {
        self.profile_name.as_ref()
    }

    pub(crate) fn config_path(&self) -> Option<&PathBuf> {
        self.config_path.as_ref()
    }

    pub(crate) fn set_profile_missing(&mut self, name: Option<String>) {
        self.profile_missing = name;
    }

    pub(crate) fn set_base_url(&mut self, url: Option<String>) {
        self.base_url = url;
    }

    pub(crate) fn set_username(&mut self, username: Option<String>) {
        self.username = username;
    }

    pub(crate) fn set_password(&mut self, password: Option<SecretString>) {
        self.password = password;
    }

    pub(crate) fn set_api_token(&mut self, token: Option<SecretString>) {
        self.api_token = token;
    }

    pub(crate) fn set_skip_verify(&mut self, skip: Option<bool>) {
        self.skip_verify = skip;
    }

    pub(crate) fn set_timeout(&mut self, timeout: Option<Duration>) {
        self.timeout = timeout;
    }

    pub(crate) fn set_max_retries(&mut self, retries: Option<usize>) {
        self.max_retries = retries;
    }

    pub(crate) fn set_session_expiry_buffer_seconds(&mut self, buffer: Option<u64>) {
        self.session_expiry_buffer_seconds = buffer;
    }

    pub(crate) fn set_session_ttl_seconds(&mut self, ttl: Option<u64>) {
        self.session_ttl_seconds = ttl;
    }

    pub(crate) fn set_health_check_interval_seconds(&mut self, interval: Option<u64>) {
        self.health_check_interval_seconds = interval;
    }

    pub(crate) fn set_earliest_time(&mut self, earliest: Option<String>) {
        self.earliest_time = earliest;
    }

    pub(crate) fn set_latest_time(&mut self, latest: Option<String>) {
        self.latest_time = latest;
    }

    pub(crate) fn set_max_results(&mut self, max_results: Option<u64>) {
        self.max_results = max_results;
    }
}

/// Validates and normalizes a base URL string.
///
/// Validation rules:
/// - Trim surrounding whitespace
/// - Treat blank/whitespace-only as missing (returns Err(ConfigError::MissingBaseUrl))
/// - Parse as an absolute URL
/// - Require scheme is http or https
/// - Require host is present
/// - Normalize by stripping trailing slash
fn validate_and_normalize_base_url(raw: &str) -> Result<String, ConfigError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(ConfigError::MissingBaseUrl);
    }

    let parsed = url::Url::parse(trimmed).map_err(|e| ConfigError::InvalidValue {
        var: "base_url".into(),
        message: format!(
            "must be an absolute http(s) URL with a host (e.g. https://localhost:8089): {e}"
        ),
    })?;

    // Validate scheme is http or https
    let scheme = parsed.scheme();
    if scheme != "http" && scheme != "https" {
        return Err(ConfigError::InvalidValue {
            var: "base_url".into(),
            message: format!(
                "scheme must be http or https (e.g. https://localhost:8089), got: {scheme}"
            ),
        });
    }

    // Validate host is present
    if parsed.host_str().is_none() {
        return Err(ConfigError::InvalidValue {
            var: "base_url".into(),
            message: "host is required (e.g. https://localhost:8089)".into(),
        });
    }

    // Normalize: strip trailing slash
    let normalized = parsed.as_str().trim_end_matches('/').to_string();

    Ok(normalized)
}
