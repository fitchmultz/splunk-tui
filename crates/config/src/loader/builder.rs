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
                session_expiry_buffer_seconds: self.session_expiry_buffer_seconds.unwrap_or(60),
                session_ttl_seconds: self.session_ttl_seconds.unwrap_or(3600),
                health_check_interval_seconds: self.health_check_interval_seconds.unwrap_or(60),
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::loader::env::env_var_or_none;
    use secrecy::ExposeSecret;
    use serial_test::serial;
    use std::io::Write;
    use std::sync::Mutex;
    use tempfile::TempDir;

    fn create_test_config_file(dir: &std::path::Path) -> PathBuf {
        let config_path = dir.join("config.json");
        let password = SecretString::new("test-password".to_string().into());

        let config = serde_json::json!({
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

        let mut file = std::fs::File::create(&config_path).unwrap();
        writeln!(file, "{}", config).unwrap();

        config_path
    }

    fn env_lock() -> &'static Mutex<()> {
        crate::test_util::global_test_lock()
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
    fn test_profile_missing_with_only_username_override() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        // Profile missing, only username provided - should get ProfileNotFound
        let loader = ConfigLoader::new()
            .with_profile_name("nonexistent".to_string())
            .with_config_path(config_path)
            .from_profile()
            .unwrap()
            .with_username("admin".to_string());

        let result = loader.build();
        assert!(
            matches!(result, Err(ConfigError::ProfileNotFound(_))),
            "Expected ProfileNotFound when only username is provided, got {:?}",
            result
        );
    }

    #[test]
    fn test_profile_missing_with_only_password_override() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        // Profile missing, only password provided - should get ProfileNotFound
        let loader = ConfigLoader::new()
            .with_profile_name("nonexistent".to_string())
            .with_config_path(config_path)
            .from_profile()
            .unwrap()
            .with_password("secret".to_string());

        let result = loader.build();
        assert!(
            matches!(result, Err(ConfigError::ProfileNotFound(_))),
            "Expected ProfileNotFound when only password is provided, got {:?}",
            result
        );
    }

    #[test]
    fn test_profile_missing_with_only_base_url_override() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        // Profile missing, only base_url provided - should get ProfileNotFound
        let loader = ConfigLoader::new()
            .with_profile_name("nonexistent".to_string())
            .with_config_path(config_path)
            .from_profile()
            .unwrap()
            .with_base_url("https://splunk.com:8089".to_string());

        let result = loader.build();
        assert!(
            matches!(result, Err(ConfigError::ProfileNotFound(_))),
            "Expected ProfileNotFound when only base_url is provided, got {:?}",
            result
        );
    }

    #[test]
    fn test_profile_missing_with_partial_session_auth() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        // Profile missing, username+password provided but no base_url - should get ProfileNotFound
        let loader = ConfigLoader::new()
            .with_profile_name("nonexistent".to_string())
            .with_config_path(config_path)
            .from_profile()
            .unwrap()
            .with_username("admin".to_string())
            .with_password("secret".to_string());

        let result = loader.build();
        assert!(
            matches!(result, Err(ConfigError::ProfileNotFound(_))),
            "Expected ProfileNotFound when username+password provided but no base_url, got {:?}",
            result
        );
    }

    #[test]
    fn test_profile_missing_with_partial_api_token_auth() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        // Profile missing, api_token provided but no base_url - should get ProfileNotFound
        let loader = ConfigLoader::new()
            .with_profile_name("nonexistent".to_string())
            .with_config_path(config_path)
            .from_profile()
            .unwrap()
            .with_api_token("my-token".to_string());

        let result = loader.build();
        assert!(
            matches!(result, Err(ConfigError::ProfileNotFound(_))),
            "Expected ProfileNotFound when api_token provided but no base_url, got {:?}",
            result
        );
    }

    #[test]
    fn test_profile_missing_with_complete_config_override() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        // Profile missing, but complete config provided via builder - should succeed
        let loader = ConfigLoader::new()
            .with_profile_name("nonexistent".to_string())
            .with_config_path(config_path)
            .from_profile()
            .unwrap()
            .with_base_url("https://splunk.com:8089".to_string())
            .with_api_token("my-token".to_string());

        let result = loader.build();
        assert!(
            result.is_ok(),
            "Expected success when complete config is provided, got {:?}",
            result
        );
        assert_eq!(
            result.unwrap().connection.base_url,
            "https://splunk.com:8089"
        );
    }

    #[test]
    fn test_profile_missing_with_complete_session_auth_override() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        // Profile missing, but complete session auth config provided - should succeed
        let loader = ConfigLoader::new()
            .with_profile_name("nonexistent".to_string())
            .with_config_path(config_path)
            .from_profile()
            .unwrap()
            .with_base_url("https://splunk.com:8089".to_string())
            .with_username("admin".to_string())
            .with_password("secret".to_string());

        let result = loader.build();
        assert!(
            result.is_ok(),
            "Expected success when complete session auth config is provided, got {:?}",
            result
        );
        assert!(matches!(
            result.unwrap().auth.strategy,
            AuthStrategy::SessionToken { .. }
        ));
    }

    #[test]
    #[serial]
    fn test_env_overrides_profile() {
        let _lock = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        temp_env::with_vars(
            [("SPLUNK_BASE_URL", Some("https://override.splunk.com:8089"))],
            || {
                let loader = ConfigLoader::new()
                    .with_profile_name("dev".to_string())
                    .with_config_path(config_path.clone())
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
            },
        );
    }

    #[test]
    #[serial]
    fn test_empty_env_vars_ignored() {
        let _lock = env_lock().lock().unwrap();

        // Set empty env vars - they should be treated as None
        temp_env::with_vars(
            [
                ("SPLUNK_API_TOKEN", Some("")),
                ("SPLUNK_USERNAME", Some("")),
                ("SPLUNK_PASSWORD", Some("")),
            ],
            || {
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
            },
        );
    }

    #[test]
    #[serial]
    fn test_whitespace_only_env_var_treated_as_unset() {
        let _lock = env_lock().lock().unwrap();

        // Whitespace-only is filtered as empty/unset
        temp_env::with_vars(
            [
                ("SPLUNK_API_TOKEN", Some("   ")),
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_USERNAME", Some("admin")),
                ("SPLUNK_PASSWORD", Some("password")),
            ],
            || {
                let loader = ConfigLoader::new().from_env().unwrap();

                let config = loader.build().unwrap();
                // Whitespace API token should be ignored, falling back to session auth
                assert!(matches!(
                    config.auth.strategy,
                    AuthStrategy::SessionToken { .. }
                ));
            },
        );
    }

    #[test]
    #[serial]
    fn test_empty_and_whitespace_env_vars_ignored_for_non_string_fields() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("token")),
                ("SPLUNK_SKIP_VERIFY", Some("   ")),
                ("SPLUNK_TIMEOUT", Some("")),
                ("SPLUNK_MAX_RETRIES", Some(" ")),
            ],
            || {
                let loader = ConfigLoader::new().from_env().unwrap();
                let config = loader.build().unwrap();

                // Should use defaults for bool/number fields instead of erroring on parse
                assert!(!config.connection.skip_verify);
                assert_eq!(config.connection.timeout, Duration::from_secs(30));
                assert_eq!(config.connection.max_retries, 3);
            },
        );
    }

    #[test]
    #[serial]
    fn test_splunk_config_path_env_var() {
        let _lock = env_lock().lock().unwrap();
        let temp_dir = TempDir::new().unwrap();
        let config_path = create_test_config_file(temp_dir.path());

        temp_env::with_vars(
            [("SPLUNK_CONFIG_PATH", Some(config_path.to_str().unwrap()))],
            || {
                // Verify that with_config_path would use the environment variable's path
                let env_path = env_var_or_none("SPLUNK_CONFIG_PATH").unwrap();
                let path_from_env = std::path::PathBuf::from(env_path);

                let loader = ConfigLoader::new()
                    .with_config_path(path_from_env)
                    .with_profile_name("prod".to_string())
                    .from_profile()
                    .unwrap();

                let config = loader.build().unwrap();
                assert_eq!(config.connection.base_url, "https://prod.splunk.com:8089");
            },
        );
    }

    #[test]
    #[serial]
    fn test_empty_splunk_config_path_ignored() {
        let _lock = env_lock().lock().unwrap();

        // Empty string in SPLUNK_CONFIG_PATH should be ignored
        temp_env::with_vars([("SPLUNK_CONFIG_PATH", Some(""))], || {
            let result = env_var_or_none("SPLUNK_CONFIG_PATH");
            assert!(
                result.is_none(),
                "Empty env var should be filtered by env_var_or_none"
            );
        });

        // Test with whitespace
        temp_env::with_vars([("SPLUNK_CONFIG_PATH", Some("   "))], || {
            let result_ws = env_var_or_none("SPLUNK_CONFIG_PATH");
            assert!(
                result_ws.is_none(),
                "Whitespace env var should be filtered by env_var_or_none"
            );
        });
    }

    #[test]
    #[serial]
    fn test_search_defaults_env_vars() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_EARLIEST_TIME", Some("-48h")),
                ("SPLUNK_LATEST_TIME", Some("2024-01-01T00:00:00")),
                ("SPLUNK_MAX_RESULTS", Some("500")),
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
                let loader = ConfigLoader::new().from_env().unwrap();

                assert_eq!(loader.earliest_time(), Some(&"-48h".to_string()));
                assert_eq!(
                    loader.latest_time(),
                    Some(&"2024-01-01T00:00:00".to_string())
                );
                assert_eq!(loader.max_results(), Some(500));
            },
        );
    }

    #[test]
    #[serial]
    fn test_search_defaults_env_vars_empty_ignored() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_EARLIEST_TIME", Some("")),
                ("SPLUNK_LATEST_TIME", Some("   ")),
                ("SPLUNK_MAX_RESULTS", Some("")),
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
                let loader = ConfigLoader::new().from_env().unwrap();

                // Empty/whitespace values should be treated as None
                assert_eq!(loader.earliest_time(), None);
                assert_eq!(loader.latest_time(), None);
                assert_eq!(loader.max_results(), None);
            },
        );
    }

    #[test]
    #[serial]
    fn test_build_search_defaults_with_persisted() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_EARLIEST_TIME", Some("-7d")),
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
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
            },
        );
    }

    #[test]
    #[serial]
    fn test_build_search_defaults_without_persisted() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
                let loader = ConfigLoader::new().from_env().unwrap();

                // Build without persisted defaults - should use hardcoded defaults
                let defaults = loader.build_search_defaults(None);

                assert_eq!(defaults.earliest_time, "-24h");
                assert_eq!(defaults.latest_time, "now");
                assert_eq!(defaults.max_results, 1000);
            },
        );
    }

    #[test]
    #[serial]
    fn test_search_defaults_env_vars_override_persisted() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_EARLIEST_TIME", Some("-1h")),
                ("SPLUNK_LATEST_TIME", Some("-5m")),
                ("SPLUNK_MAX_RESULTS", Some("100")),
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
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
            },
        );
    }

    #[test]
    #[serial]
    fn test_invalid_max_results_env_var() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_MAX_RESULTS", Some("not-a-number")),
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
                let result = ConfigLoader::new().from_env();

                match result {
                    Err(ConfigError::InvalidValue { var, .. }) => {
                        assert_eq!(var, "SPLUNK_MAX_RESULTS");
                    }
                    Ok(_) => panic!("Expected an error for invalid SPLUNK_MAX_RESULTS"),
                    Err(_) => panic!("Expected InvalidValue error for SPLUNK_MAX_RESULTS"),
                }
            },
        );
    }

    #[test]
    #[serial]
    fn test_session_ttl_env_var() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_SESSION_TTL", Some("7200")),
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
                let loader = ConfigLoader::new().from_env().unwrap();
                let config = loader.build().unwrap();

                assert_eq!(config.connection.session_ttl_seconds, 7200);
                // Default buffer should still be 60
                assert_eq!(config.connection.session_expiry_buffer_seconds, 60);
            },
        );
    }

    #[test]
    #[serial]
    fn test_session_expiry_buffer_env_var() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_SESSION_EXPIRY_BUFFER", Some("120")),
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
                let loader = ConfigLoader::new().from_env().unwrap();
                let config = loader.build().unwrap();

                assert_eq!(config.connection.session_expiry_buffer_seconds, 120);
                // Default TTL should still be 3600
                assert_eq!(config.connection.session_ttl_seconds, 3600);
            },
        );
    }

    #[test]
    #[serial]
    fn test_session_ttl_and_buffer_env_vars_together() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_SESSION_TTL", Some("7200")),
                ("SPLUNK_SESSION_EXPIRY_BUFFER", Some("120")),
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
                let loader = ConfigLoader::new().from_env().unwrap();
                let config = loader.build().unwrap();

                assert_eq!(config.connection.session_ttl_seconds, 7200);
                assert_eq!(config.connection.session_expiry_buffer_seconds, 120);
            },
        );
    }

    #[test]
    #[serial]
    fn test_invalid_session_ttl_env_var() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_SESSION_TTL", Some("not-a-number")),
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
                let result = ConfigLoader::new().from_env();

                match result {
                    Err(ConfigError::InvalidValue { var, .. }) => {
                        assert_eq!(var, "SPLUNK_SESSION_TTL");
                    }
                    Ok(_) => panic!("Expected an error for invalid SPLUNK_SESSION_TTL"),
                    Err(_) => panic!("Expected InvalidValue error for SPLUNK_SESSION_TTL"),
                }
            },
        );
    }

    #[test]
    #[serial]
    fn test_session_ttl_default_values() {
        let _lock = env_lock().lock().unwrap();

        // Don't set any session env vars
        temp_env::with_vars(
            [
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
                let loader = ConfigLoader::new().from_env().unwrap();
                let config = loader.build().unwrap();

                // Should use defaults
                assert_eq!(config.connection.session_ttl_seconds, 3600);
                assert_eq!(config.connection.session_expiry_buffer_seconds, 60);
            },
        );
    }

    #[test]
    #[serial]
    fn test_health_check_interval_env_var() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_HEALTH_CHECK_INTERVAL", Some("120")),
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
                let loader = ConfigLoader::new().from_env().unwrap();
                let config = loader.build().unwrap();

                assert_eq!(config.connection.health_check_interval_seconds, 120);
            },
        );
    }

    #[test]
    #[serial]
    fn test_health_check_interval_default() {
        let _lock = env_lock().lock().unwrap();

        // Don't set health check interval env var
        temp_env::with_vars(
            [
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
                let loader = ConfigLoader::new().from_env().unwrap();
                let config = loader.build().unwrap();

                // Should use default of 60 seconds
                assert_eq!(config.connection.health_check_interval_seconds, 60);
            },
        );
    }

    #[test]
    #[serial]
    fn test_invalid_health_check_interval_env_var() {
        let _lock = env_lock().lock().unwrap();

        temp_env::with_vars(
            [
                ("SPLUNK_HEALTH_CHECK_INTERVAL", Some("not-a-number")),
                ("SPLUNK_BASE_URL", Some("https://localhost:8089")),
                ("SPLUNK_API_TOKEN", Some("test-token")),
            ],
            || {
                let result = ConfigLoader::new().from_env();

                match result {
                    Err(ConfigError::InvalidValue { var, .. }) => {
                        assert_eq!(var, "SPLUNK_HEALTH_CHECK_INTERVAL");
                    }
                    Ok(_) => panic!("Expected an error for invalid SPLUNK_HEALTH_CHECK_INTERVAL"),
                    Err(_) => {
                        panic!("Expected InvalidValue error for SPLUNK_HEALTH_CHECK_INTERVAL")
                    }
                }
            },
        );
    }
}
