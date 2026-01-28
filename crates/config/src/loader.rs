//! Configuration loader for environment variables and files.
//!
//! Responsibilities:
//! - Load configuration from `.env` files, environment variables, and JSON profile files.
//! - Provide a builder-pattern `ConfigLoader` for hierarchical configuration merging.
//! - Enforce `DOTENV_DISABLED` gate to prevent accidental dotenv loading in tests.
//!
//! Does NOT handle:
//! - Persisting configuration changes back to disk (see `persistence.rs`).
//! - Interaction with system keyrings directly (delegated to `types.rs` via `resolve()`).
//!
//! Invariants / Assumptions:
//! - Environment variables take precedence over profile file values.
//! - `load_dotenv()` must be called explicitly to enable `.env` file loading.
//! - The `DOTENV_DISABLED` variable is checked before `dotenvy::dotenv()` is called.

use secrecy::SecretString;
use std::path::PathBuf;
use std::time::Duration;
use thiserror::Error;

use crate::persistence::{
    ConfigFileError, SearchDefaults, default_config_path, legacy_config_path,
    migrate_config_file_if_needed, read_config_file,
};
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
    earliest_time: Option<String>,
    latest_time: Option<String>,
    max_results: Option<u64>,
}

impl Default for ConfigLoader {
    fn default() -> Self {
        Self::new()
    }
}

/// Search default configuration values.
///
/// This is separate from the main `Config` because search defaults
/// are persisted to disk and managed through the TUI settings.
#[derive(Debug, Clone)]
pub struct SearchDefaultConfig {
    /// Earliest time for searches (e.g., "-24h").
    pub earliest_time: String,
    /// Latest time for searches (e.g., "now").
    pub latest_time: String,
    /// Maximum number of results to return per search.
    pub max_results: u64,
}

impl Default for SearchDefaultConfig {
    fn default() -> Self {
        Self {
            earliest_time: "-24h".to_string(),
            latest_time: "now".to_string(),
            max_results: 1000,
        }
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
            earliest_time: None,
            latest_time: None,
            max_results: None,
        }
    }

    /// Load environment variables from .env file if present.
    ///
    /// If `DOTENV_DISABLED` environment variable is set to "true" or "1",
    /// the .env file will not be loaded (useful for testing).
    pub fn load_dotenv(self) -> Result<Self, ConfigError> {
        if std::env::var("DOTENV_DISABLED").ok().as_deref() != Some("true")
            && std::env::var("DOTENV_DISABLED").ok().as_deref() != Some("1")
        {
            dotenvy::dotenv().ok();
        }
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

        // If we're using the default config path, attempt a best-effort migration from the
        // legacy path before checking existence. This prevents TUI startup failures when
        // users rely on profiles stored at the legacy location.
        if let (None, Ok(legacy_path)) = (&self.config_path, legacy_config_path()) {
            migrate_config_file_if_needed(&legacy_path, &config_path);
        }

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

    /// Read an environment variable, returning None if unset, empty, or whitespace-only.
    pub fn env_var_or_none(key: &str) -> Option<String> {
        std::env::var(key).ok().filter(|s| !s.trim().is_empty())
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
        if let Some(skip) = Self::env_var_or_none("SPLUNK_SKIP_VERIFY") {
            self.skip_verify =
                Some(skip.trim().parse().map_err(|_| ConfigError::InvalidValue {
                    var: "SPLUNK_SKIP_VERIFY".to_string(),
                    message: "must be true or false".to_string(),
                })?);
        }
        if let Some(timeout) = Self::env_var_or_none("SPLUNK_TIMEOUT") {
            let secs: u64 = timeout
                .trim()
                .parse()
                .map_err(|_| ConfigError::InvalidValue {
                    var: "SPLUNK_TIMEOUT".to_string(),
                    message: "must be a number".to_string(),
                })?;
            self.timeout = Some(Duration::from_secs(secs));
        }
        if let Some(retries) = Self::env_var_or_none("SPLUNK_MAX_RETRIES") {
            self.max_retries =
                Some(
                    retries
                        .trim()
                        .parse()
                        .map_err(|_| ConfigError::InvalidValue {
                            var: "SPLUNK_MAX_RETRIES".to_string(),
                            message: "must be a number".to_string(),
                        })?,
                );
        }
        // Search defaults
        if let Some(earliest) = Self::env_var_or_none("SPLUNK_EARLIEST_TIME") {
            self.earliest_time = Some(earliest);
        }
        if let Some(latest) = Self::env_var_or_none("SPLUNK_LATEST_TIME") {
            self.latest_time = Some(latest);
        }
        if let Some(max_results) = Self::env_var_or_none("SPLUNK_MAX_RESULTS") {
            self.max_results =
                Some(
                    max_results
                        .trim()
                        .parse()
                        .map_err(|_| ConfigError::InvalidValue {
                            var: "SPLUNK_MAX_RESULTS".to_string(),
                            message: "must be a positive number".to_string(),
                        })?,
                );
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;
    use serde_json::json;
    use serial_test::serial;
    use std::fs::File;
    use std::io::Write;
    use std::path::Path;
    use std::sync::Mutex;
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

    fn env_lock() -> &'static Mutex<()> {
        crate::test_util::global_test_lock()
    }

    fn cleanup_splunk_env() {
        unsafe {
            std::env::remove_var("SPLUNK_BASE_URL");
            std::env::remove_var("SPLUNK_API_TOKEN");
            std::env::remove_var("SPLUNK_USERNAME");
            std::env::remove_var("SPLUNK_PASSWORD");
            std::env::remove_var("SPLUNK_CONFIG_PATH");
            std::env::remove_var("SPLUNK_PROFILE");
            std::env::remove_var("SPLUNK_SKIP_VERIFY");
            std::env::remove_var("SPLUNK_TIMEOUT");
            std::env::remove_var("SPLUNK_MAX_RETRIES");
            std::env::remove_var("SPLUNK_EARLIEST_TIME");
            std::env::remove_var("SPLUNK_LATEST_TIME");
            std::env::remove_var("SPLUNK_MAX_RESULTS");
        }
    }

    /// Serializes process-global env-var mutations for this test module.
    struct EnvVarGuard {
        _lock: std::sync::MutexGuard<'static, ()>,
    }

    impl EnvVarGuard {
        fn new() -> Self {
            let lock = env_lock()
                .lock()
                .expect("Failed to acquire SPLUNK_* env var lock");
            cleanup_splunk_env();
            Self { _lock: lock }
        }
    }

    impl Drop for EnvVarGuard {
        fn drop(&mut self) {
            cleanup_splunk_env();
        }
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
    #[serial]
    fn test_env_overrides_profile() {
        let _env = EnvVarGuard::new();
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
    #[serial]
    fn test_empty_env_vars_ignored() {
        let _env = EnvVarGuard::new();
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
    #[serial]
    fn test_env_var_or_none_filters_empty_and_whitespace_strings() {
        let _env = EnvVarGuard::new();
        // Direct unit test for the env_var_or_none helper function

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

        // Test 3: Whitespace-only string env var returns None
        unsafe {
            std::env::set_var(key1, "   ");
        }
        let result3 = ConfigLoader::env_var_or_none(key1);
        assert!(
            result3.is_none(),
            "Whitespace-only env var should return None"
        );
        unsafe {
            std::env::remove_var(key1);
        }

        // Test 4: Non-empty string env var returns Some(trimmed)
        let key2 = "_SPLUNK_TEST_SET_VAR";
        unsafe {
            std::env::set_var(key2, " test-value ");
        }
        let result4 = ConfigLoader::env_var_or_none(key2);
        assert_eq!(
            result4,
            Some(" test-value ".to_string()), // Implementation doesn't trim the value, just checks if trimmed is empty
            "Non-empty env var should return Some(value)"
        );
        unsafe {
            std::env::remove_var(key2);
        }
    }

    #[test]
    #[serial]
    fn test_whitespace_only_env_var_treated_as_unset() {
        let _env = EnvVarGuard::new();
        // Whitespace-only is filtered as empty/unset
        unsafe {
            std::env::set_var("SPLUNK_API_TOKEN", "   ");
            std::env::set_var("SPLUNK_BASE_URL", "https://localhost:8089");
            std::env::set_var("SPLUNK_USERNAME", "admin");
            std::env::set_var("SPLUNK_PASSWORD", "password");
        }

        let loader = ConfigLoader::new().from_env().unwrap();

        let config = loader.build().unwrap();
        // Whitespace API token should be ignored, falling back to session auth
        assert!(matches!(
            config.auth.strategy,
            AuthStrategy::SessionToken { .. }
        ));

        unsafe {
            std::env::remove_var("SPLUNK_API_TOKEN");
            std::env::remove_var("SPLUNK_BASE_URL");
            std::env::remove_var("SPLUNK_USERNAME");
            std::env::remove_var("SPLUNK_PASSWORD");
        }
    }

    #[test]
    #[serial]
    fn test_empty_and_whitespace_env_vars_ignored_for_non_string_fields() {
        let _env = EnvVarGuard::new();
        unsafe {
            std::env::set_var("SPLUNK_BASE_URL", "https://localhost:8089");
            std::env::set_var("SPLUNK_API_TOKEN", "token");
            std::env::set_var("SPLUNK_SKIP_VERIFY", "   ");
            std::env::set_var("SPLUNK_TIMEOUT", "");
            std::env::set_var("SPLUNK_MAX_RETRIES", " ");
        }

        let loader = ConfigLoader::new().from_env().unwrap();
        let config = loader.build().unwrap();

        // Should use defaults for bool/number fields instead of erroring on parse
        assert!(!config.connection.skip_verify);
        assert_eq!(config.connection.timeout, Duration::from_secs(30));
        assert_eq!(config.connection.max_retries, 3);
    }

    #[test]
    #[serial]
    fn test_splunk_config_path_env_var() {
        let _env = EnvVarGuard::new();
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        // Set SPLUNK_CONFIG_PATH environment variable
        unsafe {
            std::env::set_var("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap());
        }

        // Verify that with_config_path would use the environment variable's path
        let env_path = ConfigLoader::env_var_or_none("SPLUNK_CONFIG_PATH").unwrap();
        let path_from_env = std::path::PathBuf::from(env_path);

        let loader = ConfigLoader::new()
            .with_config_path(path_from_env)
            .with_profile_name("prod".to_string())
            .from_profile()
            .unwrap();

        let config = loader.build().unwrap();
        assert_eq!(config.connection.base_url, "https://prod.splunk.com:8089");

        unsafe {
            std::env::remove_var("SPLUNK_CONFIG_PATH");
        }
    }

    #[test]
    #[serial]
    fn test_empty_splunk_config_path_ignored() {
        let _env = EnvVarGuard::new();
        // Empty string in SPLUNK_CONFIG_PATH should be ignored
        unsafe {
            std::env::set_var("SPLUNK_CONFIG_PATH", "");
        }

        let result = ConfigLoader::env_var_or_none("SPLUNK_CONFIG_PATH");
        assert!(
            result.is_none(),
            "Empty env var should be filtered by env_var_or_none"
        );

        // Test with whitespace
        unsafe {
            std::env::set_var("SPLUNK_CONFIG_PATH", "   ");
        }
        let result_ws = ConfigLoader::env_var_or_none("SPLUNK_CONFIG_PATH");
        assert!(
            result_ws.is_none(),
            "Whitespace env var should be filtered by env_var_or_none"
        );

        unsafe {
            std::env::remove_var("SPLUNK_CONFIG_PATH");
        }
    }

    #[test]
    #[serial]
    fn test_search_defaults_env_vars() {
        let _env = EnvVarGuard::new();

        // Set search default env vars
        unsafe {
            std::env::set_var("SPLUNK_EARLIEST_TIME", "-48h");
            std::env::set_var("SPLUNK_LATEST_TIME", "2024-01-01T00:00:00");
            std::env::set_var("SPLUNK_MAX_RESULTS", "500");
            std::env::set_var("SPLUNK_BASE_URL", "https://localhost:8089");
            std::env::set_var("SPLUNK_API_TOKEN", "test-token");
        }

        let loader = ConfigLoader::new().from_env().unwrap();

        assert_eq!(loader.earliest_time(), Some(&"-48h".to_string()));
        assert_eq!(
            loader.latest_time(),
            Some(&"2024-01-01T00:00:00".to_string())
        );
        assert_eq!(loader.max_results(), Some(500));
    }

    #[test]
    #[serial]
    fn test_search_defaults_env_vars_empty_ignored() {
        let _env = EnvVarGuard::new();

        // Set empty/whitespace search default env vars
        unsafe {
            std::env::set_var("SPLUNK_EARLIEST_TIME", "");
            std::env::set_var("SPLUNK_LATEST_TIME", "   ");
            std::env::set_var("SPLUNK_MAX_RESULTS", "");
            std::env::set_var("SPLUNK_BASE_URL", "https://localhost:8089");
            std::env::set_var("SPLUNK_API_TOKEN", "test-token");
        }

        let loader = ConfigLoader::new().from_env().unwrap();

        // Empty/whitespace values should be treated as None
        assert_eq!(loader.earliest_time(), None);
        assert_eq!(loader.latest_time(), None);
        assert_eq!(loader.max_results(), None);
    }

    #[test]
    #[serial]
    fn test_build_search_defaults_with_persisted() {
        let _env = EnvVarGuard::new();

        // Set only some env vars
        unsafe {
            std::env::set_var("SPLUNK_EARLIEST_TIME", "-7d");
            std::env::set_var("SPLUNK_BASE_URL", "https://localhost:8089");
            std::env::set_var("SPLUNK_API_TOKEN", "test-token");
        }

        let loader = ConfigLoader::new().from_env().unwrap();

        let persisted = SearchDefaults {
            earliest_time: "-24h".to_string(),
            latest_time: "now".to_string(),
            max_results: 1000,
        };

        let defaults = loader.build_search_defaults(Some(persisted));

        // Env var should override persisted
        assert_eq!(defaults.earliest_time, "-7d");
        // Non-env values should use persisted
        assert_eq!(defaults.latest_time, "now");
        assert_eq!(defaults.max_results, 1000);
    }

    #[test]
    #[serial]
    fn test_build_search_defaults_without_persisted() {
        let _env = EnvVarGuard::new();

        // Don't set any search default env vars
        unsafe {
            std::env::set_var("SPLUNK_BASE_URL", "https://localhost:8089");
            std::env::set_var("SPLUNK_API_TOKEN", "test-token");
        }

        let loader = ConfigLoader::new().from_env().unwrap();

        // Build without persisted defaults - should use hardcoded defaults
        let defaults = loader.build_search_defaults(None);

        assert_eq!(defaults.earliest_time, "-24h");
        assert_eq!(defaults.latest_time, "now");
        assert_eq!(defaults.max_results, 1000);
    }

    #[test]
    #[serial]
    fn test_search_defaults_env_vars_override_persisted() {
        let _env = EnvVarGuard::new();

        // Set all search default env vars
        unsafe {
            std::env::set_var("SPLUNK_EARLIEST_TIME", "-1h");
            std::env::set_var("SPLUNK_LATEST_TIME", "-5m");
            std::env::set_var("SPLUNK_MAX_RESULTS", "100");
            std::env::set_var("SPLUNK_BASE_URL", "https://localhost:8089");
            std::env::set_var("SPLUNK_API_TOKEN", "test-token");
        }

        let loader = ConfigLoader::new().from_env().unwrap();

        let persisted = SearchDefaults {
            earliest_time: "-48h".to_string(),
            latest_time: "2024-01-01T00:00:00".to_string(),
            max_results: 5000,
        };

        let defaults = loader.build_search_defaults(Some(persisted));

        // All env vars should override persisted values
        assert_eq!(defaults.earliest_time, "-1h");
        assert_eq!(defaults.latest_time, "-5m");
        assert_eq!(defaults.max_results, 100);
    }

    #[test]
    #[serial]
    fn test_invalid_max_results_env_var() {
        let _env = EnvVarGuard::new();

        unsafe {
            std::env::set_var("SPLUNK_MAX_RESULTS", "not-a-number");
            std::env::set_var("SPLUNK_BASE_URL", "https://localhost:8089");
            std::env::set_var("SPLUNK_API_TOKEN", "test-token");
        }

        let result = ConfigLoader::new().from_env();

        match result {
            Err(ConfigError::InvalidValue { var, .. }) => {
                assert_eq!(var, "SPLUNK_MAX_RESULTS");
            }
            Ok(_) => panic!("Expected an error for invalid SPLUNK_MAX_RESULTS"),
            Err(_) => panic!("Expected InvalidValue error for SPLUNK_MAX_RESULTS"),
        }
    }
}
