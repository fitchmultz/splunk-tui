//! Configuration persistence for user preferences.
//!
//! This module provides functionality to save and load user preferences
//! to disk using platform-standard configuration directories.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

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
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            auto_refresh: false,
            sort_column: "sid".to_string(),
            sort_direction: "asc".to_string(),
            last_search_query: None,
        }
    }
}

/// Manages loading and saving user configuration to disk.
pub struct ConfigManager {
    /// Path to the configuration file.
    config_path: PathBuf,
}

impl ConfigManager {
    /// Creates a new `ConfigManager` using platform-standard config directories.
    ///
    /// # Errors
    /// Returns an error if `ProjectDirs::from` fails (should be rare).
    pub fn new() -> Result<Self> {
        let proj_dirs = directories::ProjectDirs::from("com", "splunk-tui", "splunk-tui")
            .context("Failed to determine project directories")?;

        let config_dir = proj_dirs.config_dir();
        let config_path = config_dir.join("config.json");

        Ok(Self { config_path })
    }

    /// Returns the path to the configuration file.
    pub fn config_path(&self) -> &PathBuf {
        &self.config_path
    }

    /// Loads persisted state from disk.
    ///
    /// Returns default state if the file doesn't exist or cannot be read.
    pub fn load(&self) -> PersistedState {
        match self.load_inner() {
            Ok(state) => state,
            Err(e) => {
                tracing::warn!(
                    path = %self.config_path.display(),
                    error = %e,
                    "Failed to load config, using defaults"
                );
                PersistedState::default()
            }
        }
    }

    fn load_inner(&self) -> Result<PersistedState> {
        let content = std::fs::read_to_string(&self.config_path)?;
        let state: PersistedState = serde_json::from_str(&content)?;
        Ok(state)
    }

    /// Saves persisted state to disk.
    ///
    /// # Errors
    /// Returns an error if the parent directory cannot be created
    /// or the file cannot be written.
    pub fn save(&self, state: &PersistedState) -> Result<()> {
        // Create parent directory if it doesn't exist
        if let Some(parent) = self.config_path.parent() {
            std::fs::create_dir_all(parent).context("Failed to create config directory")?;
        }

        let content = serde_json::to_string_pretty(state)?;
        std::fs::write(&self.config_path, content).context("Failed to write config file")?;

        tracing::debug!(
            path = %self.config_path.display(),
            "Config saved successfully"
        );

        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_persisted_state_default() {
        let state = PersistedState::default();
        assert!(!state.auto_refresh);
        assert_eq!(state.sort_column, "sid");
        assert_eq!(state.sort_direction, "asc");
        assert!(state.last_search_query.is_none());
    }

    #[test]
    fn test_serialize_deserialize() {
        let state = PersistedState {
            auto_refresh: true,
            sort_column: "status".to_string(),
            sort_direction: "desc".to_string(),
            last_search_query: Some("test query".to_string()),
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
    }
}
