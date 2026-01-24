//! Configuration loader for environment variables and files.

use secrecy::SecretString;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

use crate::persistence::{ConfigFileError, default_config_path, read_config_file};
use crate::types::{AuthConfig, AuthStrategy, Config, ConnectionConfig, ProfileConfig};

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

    #[error("Unable to determine config directory: {0}")]
    ConfigDirUnavailable(String),

    #[error("Failed to read config file at {path}")]
    ConfigFileRead { path: PathBuf },

    #[error("Failed to parse config file at {path}")]
    ConfigFileParse { path: PathBuf },

    #[error("Profile '{0}' not found in config file")]
    ProfileNotFound(String),

    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<ConfigFileError> for ConfigError {
    fn from(error: ConfigFileError) -> Self {
        match error {
            ConfigFileError::Read { path, .. } => ConfigError::ConfigFileRead { path },
            ConfigFileError::Parse { path, .. } => ConfigError::ConfigFileParse { path },
        }
    }
}

/// Configuration loader that builds config from environment variables and profiles.
pub struct ConfigLoader {
    base_url: Option<String>,
    username: Option<String>,
    password: Option<SecretString>,
    api_token: Option<SecretString>,
    skip_verify: Option<bool>,
    timeout: Option<Duration>,
    max_retries: Option<usize>,
    profile_name: Option<String>,
    profile_missing: Option<String>,
    config_path: Option<PathBuf>,
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
            profile_name: None,
            profile_missing: None,
            config_path: None,
        }
    }

    /// Load environment variables from .env file if present.
    pub fn load_dotenv(self) -> Result<Self, ConfigError> {
        dotenvy::dotenv().ok();
        Ok(self)
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
        let profile_name = match &self.profile_name {
            Some(name) => name.clone(),
            None => return Ok(self),
        };

        let config_path = if let Some(path) = &self.config_path {
            path.clone()
        } else {
            default_config_path().map_err(|e| ConfigError::ConfigDirUnavailable(e.to_string()))?
        };

        if !config_path.exists() {
            self.profile_missing = Some(profile_name);
            return Ok(self);
        }

        let config_file = read_config_file(&config_path)?;

        let profile = match config_file.profiles.get(&profile_name) {
            Some(p) => p,
            None => {
                self.profile_missing = Some(profile_name);
                return Ok(self);
            }
        };

        self.apply_profile(profile)?;
        Ok(self)
    }

    /// Apply profile configuration to the loader.
    fn apply_profile(&mut self, profile: &ProfileConfig) -> Result<(), ConfigError> {
        if let Some(url) = &profile.base_url {
            self.base_url = Some(url.clone());
        }
        if let Some(username) = &profile.username {
            self.username = Some(username.clone());
        }
        if let Some(password) = &profile.password {
            self.password = Some(password.resolve()?);
        }
        if let Some(token) = &profile.api_token {
            self.api_token = Some(token.resolve()?);
        }
        if let Some(skip) = profile.skip_verify {
            self.skip_verify = Some(skip);
        }
        if let Some(secs) = profile.timeout_seconds {
            self.timeout = Some(Duration::from_secs(secs));
        }
        if let Some(retries) = profile.max_retries {
            self.max_retries = Some(retries);
        }
        Ok(())
    }

    /// Read an environment variable, returning None if unset or empty.
    fn env_var_or_none(key: &str) -> Option<String> {
        std::env::var(key).ok().filter(|s| !s.is_empty())
    }

    /// Read configuration from environment variables.
    ///
    /// Environment variables take precedence over profile settings.
    pub fn from_env(mut self) -> Result<Self, ConfigError> {
        if let Some(url) = Self::env_var_or_none("SPLUNK_BASE_URL") {
            self.base_url = Some(url);
        }
        if let Some(username) = Self::env_var_or_none("SPLUNK_USERNAME") {
            self.username = Some(username);
        }
        if let Some(password) = Self::env_var_or_none("SPLUNK_PASSWORD") {
            self.password = Some(SecretString::new(password.into()));
        }
        if let Some(token) = Self::env_var_or_none("SPLUNK_API_TOKEN") {
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
        // Check for missing profile first
        if let Some(profile_name) = self.profile_missing {
            // Only error if we don't have overrides from env/CLI
            if self.base_url.is_none()
                && self.username.is_none()
                && self.password.is_none()
                && self.api_token.is_none()
            {
                return Err(ConfigError::ProfileNotFound(profile_name));
            }
        }

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
    use secrecy::SecretString;
    use serde_json::json;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use tempfile::TempDir;

    fn create_test_config_file(dir: &Path) -> PathBuf {
        let config_path = dir.join("config.json");
        use secrecy::ExposeSecret;
        let password = SecretString::new("test-password".to_string().into());

        let config = json!({
            "profiles": {
                "dev": {
                    "base_url": "https://dev.splunk.com:8089",
                    "username": "dev_user",
                    "password": password.expose_secret(),
                    "skip_verify": true,
                    "timeout_seconds": 60,
                    "max_retries": 5
                },
                "prod": {
                    "base_url": "https://prod.splunk.com:8089",
                    "api_token": "prod-token-123"
                }
            },
            "state": {
                "auto_refresh": true,
                "sort_column": "sid",
                "sort_direction": "asc"
            }
        });

        let mut file = File::create(&config_path).unwrap();
        writeln!(file, "{}", config).unwrap();

        config_path
    }

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

    #[test]
    fn test_loader_from_profile_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        let loader = ConfigLoader::new()
            .with_profile_name("dev".to_string())
            .with_config_path(config_path)
            .from_profile()
            .unwrap();

        let config = loader.build().unwrap();
        assert_eq!(config.connection.base_url, "https://dev.splunk.com:8089");
        assert!(config.connection.skip_verify);
        assert_eq!(config.connection.timeout, Duration::from_secs(60));
        assert_eq!(config.connection.max_retries, 5);
    }

    #[test]
    fn test_profile_missing_errors_without_overrides() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        let loader = ConfigLoader::new()
            .with_profile_name("nonexistent".to_string())
            .with_config_path(config_path)
            .from_profile()
            .unwrap();

        let result = loader.build();
        assert!(matches!(result, Err(ConfigError::ProfileNotFound(_))));
    }

    #[test]
    fn test_env_overrides_profile() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        // Set env var to override profile
        unsafe {
            std::env::set_var("SPLUNK_BASE_URL", "https://override.splunk.com:8089");
        }

        let loader = ConfigLoader::new()
            .with_profile_name("dev".to_string())
            .with_config_path(config_path)
            .from_profile()
            .unwrap()
            .from_env()
            .unwrap();

        let config = loader.build().unwrap();
        // Env var should take precedence over profile
        assert_eq!(
            config.connection.base_url,
            "https://override.splunk.com:8089"
        );

        unsafe {
            std::env::remove_var("SPLUNK_BASE_URL");
        }
    }

    #[test]
    fn test_empty_env_vars_ignored() {
        // Clean up first to ensure test isolation
        unsafe {
            std::env::remove_var("SPLUNK_API_TOKEN");
            std::env::remove_var("SPLUNK_USERNAME");
            std::env::remove_var("SPLUNK_PASSWORD");
        }

        // Set empty env vars - they should be treated as None
        unsafe {
            std::env::set_var("SPLUNK_API_TOKEN", "");
            std::env::set_var("SPLUNK_USERNAME", "");
            std::env::set_var("SPLUNK_PASSWORD", "");
        }

        let loader = ConfigLoader::new()
            .with_base_url("https://localhost:8089".to_string())
            .with_username("admin".to_string()) // Set via builder
            .with_password("password".to_string())
            .from_env()
            .unwrap();

        let config = loader.build().unwrap();
        // Should use session auth since API token env is empty
        assert!(matches!(
            config.auth.strategy,
            AuthStrategy::SessionToken { .. }
        ));

        unsafe {
            std::env::remove_var("SPLUNK_API_TOKEN");
            std::env::remove_var("SPLUNK_USERNAME");
            std::env::remove_var("SPLUNK_PASSWORD");
        }
    }

    #[test]
    fn test_env_var_or_none_filters_empty_strings() {
        // Direct unit test for the env_var_or_none helper function
        // This tests the core functionality without relying on complex env var setup

        // Test 1: Unset env var returns None
        let key1 = "_SPLUNK_TEST_UNSET_VAR";
        let result1 = ConfigLoader::env_var_or_none(key1);
        assert!(result1.is_none(), "Unset env var should return None");

        // Test 2: Empty string env var returns None
        unsafe {
            std::env::set_var(key1, "");
        }
        let result2 = ConfigLoader::env_var_or_none(key1);
        assert!(result2.is_none(), "Empty string env var should return None");
        unsafe {
            std::env::remove_var(key1);
        }

        // Test 3: Non-empty string env var returns Some
        let key2 = "_SPLUNK_TEST_SET_VAR";
        unsafe {
            std::env::set_var(key2, "test-value");
        }
        let result3 = ConfigLoader::env_var_or_none(key2);
        assert_eq!(
            result3,
            Some("test-value".to_string()),
            "Non-empty env var should return Some(value)"
        );
        unsafe {
            std::env::remove_var(key2);
        }

        // Test 4: Whitespace-only string is NOT filtered
        let key3 = "_SPLUNK_TEST_WHITESPACE_VAR";
        unsafe {
            std::env::set_var(key3, "   ");
        }
        let result4 = ConfigLoader::env_var_or_none(key3);
        assert_eq!(
            result4,
            Some("   ".to_string()),
            "Whitespace-only env var should return Some(whitespace)"
        );
        unsafe {
            std::env::remove_var(key3);
        }
    }

    #[test]
    fn test_whitespace_only_env_var_treated_as_set() {
        // Whitespace-only is NOT filtered as empty (only empty string is)
        unsafe {
            std::env::set_var("SPLUNK_API_TOKEN", "   ");
            std::env::set_var("SPLUNK_BASE_URL", "https://localhost:8089");
        }

        let loader = ConfigLoader::new().from_env().unwrap();

        let config = loader.build().unwrap();
        // Whitespace is a valid (though invalid for auth) value, so API token is used
        assert!(matches!(
            config.auth.strategy,
            AuthStrategy::ApiToken { .. }
        ));

        unsafe {
            std::env::remove_var("SPLUNK_API_TOKEN");
            std::env::remove_var("SPLUNK_BASE_URL");
        }
    }
}
