//! Configuration persistence for user preferences.
//!
//! This module provides functionality to save and load user preferences
//! to disk using platform-standard configuration directories.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::types::{KEYRING_SERVICE, ProfileConfig, SecureValue};

/// User preferences that persist across application runs.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PersistedState {
    /// Whether auto-refresh is enabled for the jobs screen.
    pub auto_refresh: bool,
    /// Current sort column (maps to `SortColumn` enum variants).
    pub sort_column: String,
    /// Current sort direction (maps to `SortDirection` enum variants).
    pub sort_direction: String,
    /// Last search query used for filtering jobs.
    pub last_search_query: Option<String>,
    /// Search history for the search screen.
    #[serde(default)]
    pub search_history: Vec<String>,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            auto_refresh: false,
            sort_column: "sid".to_string(),
            sort_direction: "asc".to_string(),
            last_search_query: None,
            search_history: Vec::new(),
        }
    }
}

/// Internal representation of the config file on disk.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub(crate) struct ConfigFile {
    /// Named profiles for different Splunk environments.
    #[serde(default)]
    pub profiles: BTreeMap<String, ProfileConfig>,
    /// Persisted UI state.
    #[serde(default)]
    pub state: Option<PersistedState>,
}

/// Errors that can occur when reading the config file.
#[derive(Debug, thiserror::Error)]
pub enum ConfigFileError {
    #[error("Failed to read config file at {path}: {source}")]
    Read {
        path: PathBuf,
        source: std::io::Error,
    },

    #[error("Failed to parse config file at {path}: {source}")]
    Parse {
        path: PathBuf,
        source: serde_json::Error,
    },
}

/// Returns the default path to the configuration file.
pub(crate) fn default_config_path() -> Result<PathBuf, anyhow::Error> {
    let proj_dirs = directories::ProjectDirs::from("com", "splunk-tui", "splunk-tui")
        .context("Failed to determine project directories")?;

    Ok(proj_dirs.config_dir().join("config.json"))
}

/// Reads and parses the config file from disk.
///
/// This function supports legacy config files that only contain `PersistedState`
/// (without the `profiles` wrapper). It returns a `ConfigFile` with empty profiles
/// for legacy files.
pub(crate) fn read_config_file(path: &Path) -> Result<ConfigFile, ConfigFileError> {
    let content = std::fs::read_to_string(path).map_err(|e| ConfigFileError::Read {
        path: path.to_path_buf(),
        source: e,
    })?;

    // Try parsing as the new ConfigFile format first
    if let Ok(mut file) = serde_json::from_str::<ConfigFile>(&content) {
        // If we got a ConfigFile but it has no state, try legacy format
        if file.state.is_none()
            && let Ok(state) = serde_json::from_str::<PersistedState>(&content)
        {
            file.state = Some(state);
        }
        return Ok(file);
    }

    // Fall back to legacy format: try parsing as PersistedState directly
    match serde_json::from_str::<PersistedState>(&content) {
        Ok(state) => Ok(ConfigFile {
            profiles: BTreeMap::new(),
            state: Some(state),
        }),
        Err(e) => Err(ConfigFileError::Parse {
            path: path.to_path_buf(),
            source: e,
        }),
    }
}

/// Manages loading and saving user configuration to disk.
pub struct ConfigManager {
    /// Path to the configuration file.
    config_path: PathBuf,
    /// Cached config file data (profiles + state).
    config_file: ConfigFile,
}

impl ConfigManager {
    /// Creates a new `ConfigManager` using platform-standard config directories.
    ///
    /// # Errors
    /// Returns an error if `ProjectDirs::from` fails (should be rare).
    pub fn new() -> Result<Self> {
        let config_path = default_config_path()?;
        Self::new_with_path(config_path)
    }

    /// Creates a new `ConfigManager` with a specific config file path.
    pub fn new_with_path(config_path: PathBuf) -> Result<Self> {
        let config_file = if config_path.exists() {
            read_config_file(&config_path).unwrap_or_else(|e| {
                tracing::warn!(
                    path = %config_path.display(),
                    error = %e,
                    "Failed to load config file, using defaults"
                );
                ConfigFile::default()
            })
        } else {
            ConfigFile::default()
        };

        Ok(Self {
            config_path,
            config_file,
        })
    }

    /// Returns the path to the configuration file.
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// Loads persisted state from disk.
    ///
    /// Returns default state if the file doesn't exist or cannot be read.
    pub fn load(&self) -> PersistedState {
        self.config_file.state.clone().unwrap_or_default()
    }

    /// Saves persisted state to disk.
    ///
    /// This preserves any existing profiles in the config file.
    ///
    /// # Errors
    /// Returns an error if the parent directory cannot be created
    /// or the file cannot be written.
    pub fn save(&mut self, state: &PersistedState) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        // Update the state while preserving profiles
        self.config_file.state = Some(state.clone());

        let content = serde_json::to_string_pretty(&self.config_file)?;
        std::fs::write(&self.config_path, content).context("Failed to write config file")?;

        tracing::debug!(
            path = %self.config_path.display(),
            "Config saved successfully"
        );

        Ok(())
    }

    /// Moves a profile's password to the system keyring.
    pub fn move_password_to_keyring(&mut self, profile_name: &str) -> Result<()> {
        let profile = self
            .config_file
            .profiles
            .get_mut(profile_name)
            .context(format!("Profile '{}' not found", profile_name))?;

        if let Some(SecureValue::Plain(password)) = &profile.password {
            use secrecy::ExposeSecret;
            let username = profile.username.as_deref().unwrap_or("unknown");
            let keyring_account = format!("{}-{}", profile_name, username);

            let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account)?;
            entry.set_password(password.expose_secret())?;

            profile.password = Some(SecureValue::Keyring { keyring_account });
            self.save(&self.load())?;
        }

        Ok(())
    }

    /// Moves a profile's API token to the system keyring.
    pub fn move_token_to_keyring(&mut self, profile_name: &str) -> Result<()> {
        let profile = self
            .config_file
            .profiles
            .get_mut(profile_name)
            .context(format!("Profile '{}' not found", profile_name))?;

        if let Some(SecureValue::Plain(token)) = &profile.api_token {
            use secrecy::ExposeSecret;
            let keyring_account = format!("{}-token", profile_name);

            let entry = keyring::Entry::new(KEYRING_SERVICE, &keyring_account)?;
            entry.set_password(token.expose_secret())?;

            profile.api_token = Some(SecureValue::Keyring { keyring_account });
            self.save(&self.load())?;
        }

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;
    use std::io::Write;
    use tempfile::NamedTempFile;

    #[test]
    fn test_persisted_state_default() {
        let state = PersistedState::default();
        assert!(!state.auto_refresh);
        assert_eq!(state.sort_column, "sid");
        assert_eq!(state.sort_direction, "asc");
        assert!(state.last_search_query.is_none());
        assert!(state.search_history.is_empty());
    }

    #[test]
    fn test_serialize_deserialize() {
        let state = PersistedState {
            auto_refresh: true,
            sort_column: "status".to_string(),
            sort_direction: "desc".to_string(),
            last_search_query: Some("test query".to_string()),
            search_history: vec!["query1".to_string(), "query2".to_string()],
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: PersistedState = serde_json::from_str(&json).unwrap();

        assert!(deserialized.auto_refresh);
        assert_eq!(deserialized.sort_column, "status");
        assert_eq!(deserialized.sort_direction, "desc");
        assert_eq!(
            deserialized.last_search_query,
            Some("test query".to_string())
        );
        assert_eq!(deserialized.search_history, vec!["query1", "query2"]);
    }

    #[test]
    fn test_read_legacy_state_file() {
        let mut temp_file = NamedTempFile::new().unwrap();
        let legacy_state = PersistedState {
            auto_refresh: true,
            sort_column: "status".to_string(),
            sort_direction: "desc".to_string(),
            last_search_query: Some("legacy query".to_string()),
            search_history: Vec::new(),
        };

        writeln!(
            temp_file,
            "{}",
            serde_json::to_string(&legacy_state).unwrap()
        )
        .unwrap();

        let config_file = read_config_file(temp_file.path()).unwrap();

        // Legacy file should result in empty profiles
        assert!(config_file.profiles.is_empty());
        // But the state should be preserved
        assert_eq!(config_file.state.unwrap().sort_column, "status");
    }

    #[test]
    fn test_save_preserves_profiles() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        // Add a profile
        let password = SecretString::new("test-password".to_string().into());
        manager.config_file.profiles.insert(
            "test-profile".to_string(),
            ProfileConfig {
                base_url: Some("https://splunk.example.com:8089".to_string()),
                username: Some("admin".to_string()),
                password: Some(SecureValue::Plain(password)),
                api_token: None,
                skip_verify: Some(true),
                timeout_seconds: Some(60),
                max_retries: Some(5),
            },
        );

        // Save state
        let state = PersistedState {
            auto_refresh: true,
            ..Default::default()
        };
        manager.save(&state).unwrap();

        // Reload and verify profiles are preserved
        let reloaded = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();
        assert!(reloaded.config_file.profiles.contains_key("test-profile"));
        assert_eq!(
            reloaded.config_file.profiles["test-profile"]
                .base_url
                .as_ref()
                .unwrap(),
            "https://splunk.example.com:8089"
        );
    }

    #[test]
    fn test_move_password_to_keyring() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let profile_name = "test-password-profile-unique";
        let password_str = "keyring-password";
        let password = SecretString::new(password_str.to_string().into());

        manager.config_file.profiles.insert(
            profile_name.to_string(),
            ProfileConfig {
                username: Some("admin".to_string()),
                password: Some(SecureValue::Plain(password)),
                ..Default::default()
            },
        );

        // Attempt to move to keyring. We handle errors gracefully in case the test environment
        // doesn't have a functional keyring backend.
        match manager.move_password_to_keyring(profile_name) {
            Ok(_) => {
                let profile = &manager.config_file.profiles[profile_name];
                assert!(matches!(
                    profile.password,
                    Some(SecureValue::Keyring { .. })
                ));

                // Verify it can be resolved
                match profile.password.as_ref().unwrap().resolve() {
                    Ok(resolved) => {
                        use secrecy::ExposeSecret;
                        assert_eq!(resolved.expose_secret(), password_str);
                    }
                    Err(e) => {
                        eprintln!("Skipping resolve check: {}", e);
                    }
                }

                // Clean up
                if let Some(SecureValue::Keyring { keyring_account }) = &profile.password {
                    let entry = keyring::Entry::new(KEYRING_SERVICE, keyring_account).unwrap();
                    let _ = entry.delete_credential();
                }
            }
            Err(e) => {
                eprintln!("Skipping keyring test: {}", e);
            }
        }
    }

    #[test]
    fn test_move_token_to_keyring() {
        let temp_file = NamedTempFile::new().unwrap();
        let mut manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();

        let profile_name = "test-token-profile-unique";
        let token_str = "test-token-123";
        let token = SecretString::new(token_str.to_string().into());

        manager.config_file.profiles.insert(
            profile_name.to_string(),
            ProfileConfig {
                api_token: Some(SecureValue::Plain(token)),
                ..Default::default()
            },
        );

        match manager.move_token_to_keyring(profile_name) {
            Ok(_) => {
                let profile = &manager.config_file.profiles[profile_name];
                assert!(matches!(
                    profile.api_token,
                    Some(SecureValue::Keyring { .. })
                ));

                // Verify it can be resolved
                match profile.api_token.as_ref().unwrap().resolve() {
                    Ok(resolved) => {
                        use secrecy::ExposeSecret;
                        assert_eq!(resolved.expose_secret(), token_str);
                    }
                    Err(e) => {
                        eprintln!("Skipping token resolve check: {}", e);
                    }
                }

                // Clean up
                if let Some(SecureValue::Keyring { keyring_account }) = &profile.api_token {
                    let entry = keyring::Entry::new(KEYRING_SERVICE, keyring_account).unwrap();
                    let _ = entry.delete_credential();
                }
            }
            Err(e) => {
                eprintln!("Skipping keyring token test: {}", e);
            }
        }
    }
}
