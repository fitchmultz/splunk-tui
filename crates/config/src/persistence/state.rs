//! State types and serialization for configuration persistence.
//!
//! Responsibilities:
//! - Define persisted state types (`PersistedState`, `SearchDefaults`).
//! - Define internal config file representation (`ConfigFile`).
//! - Define config file errors (`ConfigFileError`).
//! - Read and parse config files (supporting legacy format).
//!
//! Does NOT handle:
//! - Writing config files (handled by profiles.rs via atomic_save).
//! - Profile management operations.
//! - Keyring interactions.
//!
//! Invariants:
//! - `max_results` must be at least 1.
//! - Time strings are validated server-side.
//! - Legacy format (PersistedState only) is supported for backward compatibility.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::types::{ColorTheme, KeybindOverrides, ProfileConfig};

/// Default search parameters to avoid unbounded searches.
///
/// These values are used when submitting searches from the TUI to ensure
/// searches are bounded by time and result count, preventing performance
/// issues on Splunk servers.
///
/// # Default Values
///
/// - `earliest_time`: "-24h" (last 24 hours)
/// - `latest_time`: "now"
/// - `max_results`: 1000
///
/// # Invariants
///
/// - `max_results` must be at least 1
/// - Time strings should be valid Splunk time modifiers (validation is done server-side)
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct SearchDefaults {
    /// Earliest time for searches (e.g., "-24h", "2024-01-01T00:00:00").
    pub earliest_time: String,
    /// Latest time for searches (e.g., "now", "2024-01-02T00:00:00").
    pub latest_time: String,
    /// Maximum number of results to return per search.
    pub max_results: u64,
}

impl Default for SearchDefaults {
    fn default() -> Self {
        Self {
            earliest_time: "-24h".to_string(),
            latest_time: "now".to_string(),
            max_results: 1000,
        }
    }
}

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
    /// Persisted UI theme selection.
    #[serde(default)]
    pub selected_theme: ColorTheme,
    /// Default search parameters to avoid unbounded searches.
    #[serde(default)]
    pub search_defaults: SearchDefaults,
    /// User-defined keybinding overrides.
    #[serde(default)]
    pub keybind_overrides: KeybindOverrides,
}

impl Default for PersistedState {
    fn default() -> Self {
        Self {
            auto_refresh: false,
            sort_column: "sid".to_string(),
            sort_direction: "asc".to_string(),
            last_search_query: None,
            search_history: Vec::new(),
            selected_theme: ColorTheme::Default,
            search_defaults: SearchDefaults::default(),
            keybind_overrides: KeybindOverrides::default(),
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

#[cfg(test)]
mod tests {
    use super::*;
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
        assert_eq!(state.selected_theme, ColorTheme::Default);
    }

    #[test]
    fn test_serialize_deserialize() {
        let state = PersistedState {
            auto_refresh: true,
            sort_column: "status".to_string(),
            sort_direction: "desc".to_string(),
            last_search_query: Some("test query".to_string()),
            search_history: vec!["query1".to_string(), "query2".to_string()],
            selected_theme: ColorTheme::Dark,
            search_defaults: SearchDefaults {
                earliest_time: "-48h".to_string(),
                latest_time: "now".to_string(),
                max_results: 500,
            },
            keybind_overrides: KeybindOverrides::default(),
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
        assert_eq!(deserialized.selected_theme, ColorTheme::Dark);
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
            selected_theme: ColorTheme::Default,
            search_defaults: SearchDefaults::default(),
            keybind_overrides: KeybindOverrides::default(),
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
    fn test_search_defaults_default() {
        let defaults = SearchDefaults::default();
        assert_eq!(defaults.earliest_time, "-24h");
        assert_eq!(defaults.latest_time, "now");
        assert_eq!(defaults.max_results, 1000);
    }

    #[test]
    fn test_search_defaults_serialization() {
        let defaults = SearchDefaults {
            earliest_time: "-48h".to_string(),
            latest_time: "2024-01-01T00:00:00".to_string(),
            max_results: 500,
        };

        let json = serde_json::to_string(&defaults).unwrap();
        assert!(json.contains("-48h"));
        assert!(json.contains("2024-01-01T00:00:00"));
        assert!(json.contains("500"));

        let deserialized: SearchDefaults = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.earliest_time, "-48h");
        assert_eq!(deserialized.latest_time, "2024-01-01T00:00:00");
        assert_eq!(deserialized.max_results, 500);
    }

    #[test]
    fn test_search_defaults_deserialization_uses_defaults_for_missing_fields() {
        // Test that missing fields use default values
        let json = r#"{}"#;
        let deserialized: SearchDefaults = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.earliest_time, "-24h");
        assert_eq!(deserialized.latest_time, "now");
        assert_eq!(deserialized.max_results, 1000);
    }

    #[test]
    fn test_persisted_state_with_search_defaults_round_trip() {
        let state = PersistedState {
            auto_refresh: true,
            sort_column: "status".to_string(),
            sort_direction: "desc".to_string(),
            last_search_query: None,
            search_history: vec![],
            selected_theme: ColorTheme::Default,
            search_defaults: SearchDefaults {
                earliest_time: "-7d".to_string(),
                latest_time: "now".to_string(),
                max_results: 2000,
            },
            keybind_overrides: KeybindOverrides::default(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: PersistedState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.search_defaults.earliest_time, "-7d");
        assert_eq!(deserialized.search_defaults.latest_time, "now");
        assert_eq!(deserialized.search_defaults.max_results, 2000);
    }

    #[test]
    fn test_persisted_state_backward_compatibility_without_search_defaults() {
        // Simulate an old config file without search_defaults
        let json = r#"{
            "auto_refresh": false,
            "sort_column": "sid",
            "sort_direction": "asc",
            "last_search_query": null,
            "search_history": [],
            "selected_theme": "default"
        }"#;

        let deserialized: PersistedState = serde_json::from_str(json).unwrap();
        // Should use defaults for missing search_defaults
        assert_eq!(deserialized.search_defaults.earliest_time, "-24h");
        assert_eq!(deserialized.search_defaults.latest_time, "now");
        assert_eq!(deserialized.search_defaults.max_results, 1000);
    }

    #[test]
    fn test_persisted_state_with_keybind_overrides_round_trip() {
        use crate::types::{KeybindAction, KeybindOverrides};

        let mut overrides = BTreeMap::new();
        overrides.insert(KeybindAction::Quit, "Ctrl+x".to_string());
        overrides.insert(KeybindAction::Help, "F1".to_string());

        let state = PersistedState {
            auto_refresh: true,
            sort_column: "status".to_string(),
            sort_direction: "desc".to_string(),
            last_search_query: None,
            search_history: vec![],
            selected_theme: ColorTheme::Default,
            search_defaults: SearchDefaults::default(),
            keybind_overrides: KeybindOverrides { overrides },
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: PersistedState = serde_json::from_str(&json).unwrap();

        assert_eq!(
            deserialized.keybind_overrides.get(KeybindAction::Quit),
            Some("Ctrl+x")
        );
        assert_eq!(
            deserialized.keybind_overrides.get(KeybindAction::Help),
            Some("F1")
        );
    }

    #[test]
    fn test_persisted_state_backward_compatibility_without_keybind_overrides() {
        // Simulate an old config file without keybind_overrides
        let json = r#"{
            "auto_refresh": false,
            "sort_column": "sid",
            "sort_direction": "asc",
            "last_search_query": null,
            "search_history": [],
            "selected_theme": "default",
            "search_defaults": {
                "earliest_time": "-24h",
                "latest_time": "now",
                "max_results": 1000
            }
        }"#;

        let deserialized: PersistedState = serde_json::from_str(json).unwrap();
        // Should use defaults for missing keybind_overrides
        assert!(deserialized.keybind_overrides.is_empty());
    }
}
