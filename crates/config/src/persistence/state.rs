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

use crate::constants::{
    DEFAULT_INTERNAL_LOGS_COUNT, DEFAULT_INTERNAL_LOGS_EARLIEST_TIME, DEFAULT_LIST_MAX_ITEMS,
    DEFAULT_LIST_PAGE_SIZE, DEFAULT_MAX_RESULTS,
};
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
            max_results: DEFAULT_MAX_RESULTS,
        }
    }
}

impl SearchDefaults {
    /// Sanitize search defaults to enforce invariants.
    ///
    /// Normalizes invalid values to their defaults:
    /// - Empty or whitespace-only `earliest_time` -> "-24h"
    /// - Empty or whitespace-only `latest_time` -> "now"
    /// - `max_results == 0` -> 1000
    ///
    /// Returns a new `SearchDefaults` with sanitized values.
    pub fn sanitize(&self) -> Self {
        let earliest_time = if self.earliest_time.trim().is_empty() {
            "-24h".to_string()
        } else {
            self.earliest_time.clone()
        };

        let latest_time = if self.latest_time.trim().is_empty() {
            "now".to_string()
        } else {
            self.latest_time.clone()
        };

        let max_results = if self.max_results == 0 {
            DEFAULT_MAX_RESULTS
        } else {
            self.max_results
        };

        Self {
            earliest_time,
            latest_time,
            max_results,
        }
    }
}

/// Default list pagination settings for TUI list screens.
///
/// These settings control how many items are fetched per page and the
/// maximum number of items to load for list screens (indexes, jobs, apps, users).
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct ListDefaults {
    /// Default page size for list screens.
    pub page_size: u64,
    /// Maximum number of items to load (safety limit).
    pub max_items: u64,
    /// Per-list override: indexes page size (None = use page_size).
    pub indexes_page_size: Option<u64>,
    /// Per-list override: jobs page size (None = use page_size).
    pub jobs_page_size: Option<u64>,
    /// Per-list override: apps page size (None = use page_size).
    pub apps_page_size: Option<u64>,
    /// Per-list override: users page size (None = use page_size).
    pub users_page_size: Option<u64>,
    /// Per-list override: roles page size (None = use page_size).
    pub roles_page_size: Option<u64>,
}

impl Default for ListDefaults {
    fn default() -> Self {
        Self {
            page_size: DEFAULT_LIST_PAGE_SIZE,
            max_items: DEFAULT_LIST_MAX_ITEMS,
            indexes_page_size: None,
            jobs_page_size: None,
            apps_page_size: None,
            users_page_size: None,
            roles_page_size: None,
        }
    }
}

impl ListDefaults {
    /// Get the effective page size for a specific list type.
    pub fn page_size_for(&self, list_type: ListType) -> u64 {
        let override_size = match list_type {
            ListType::Indexes => self.indexes_page_size,
            ListType::Jobs => self.jobs_page_size,
            ListType::Apps => self.apps_page_size,
            ListType::Users => self.users_page_size,
            ListType::Roles => self.roles_page_size,
        };
        override_size.unwrap_or(self.page_size)
    }
}

/// List type for retrieving per-list pagination settings.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ListType {
    Indexes,
    Jobs,
    Apps,
    Users,
    Roles,
}

/// Default parameters for internal logs queries.
///
/// These values control how many internal log entries are fetched
/// and the time range for the query.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(default)]
pub struct InternalLogsDefaults {
    /// Number of log entries to fetch per query.
    pub count: u64,
    /// Earliest time for log queries (e.g., "-15m", "-1h", "2024-01-01T00:00:00").
    pub earliest_time: String,
}

impl Default for InternalLogsDefaults {
    fn default() -> Self {
        Self {
            count: DEFAULT_INTERNAL_LOGS_COUNT,
            earliest_time: DEFAULT_INTERNAL_LOGS_EARLIEST_TIME.to_string(),
        }
    }
}

impl InternalLogsDefaults {
    /// Sanitize internal logs defaults to enforce invariants.
    ///
    /// Normalizes invalid values to their defaults:
    /// - `count == 0` -> 100
    /// - Empty or whitespace-only `earliest_time` -> "-15m"
    ///
    /// Returns a new `InternalLogsDefaults` with sanitized values.
    pub fn sanitize(&self) -> Self {
        let count = if self.count == 0 {
            DEFAULT_INTERNAL_LOGS_COUNT
        } else {
            self.count
        };

        let earliest_time = if self.earliest_time.trim().is_empty() {
            DEFAULT_INTERNAL_LOGS_EARLIEST_TIME.to_string()
        } else {
            self.earliest_time.clone()
        };

        Self {
            count,
            earliest_time,
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
    /// Default list pagination settings.
    #[serde(default)]
    pub list_defaults: ListDefaults,
    /// Default internal logs query parameters.
    #[serde(default)]
    pub internal_logs_defaults: InternalLogsDefaults,
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
            list_defaults: ListDefaults::default(),
            internal_logs_defaults: InternalLogsDefaults::default(),
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

impl ConfigFileError {
    /// Returns true if this error is a file read error (e.g., permissions, not found).
    pub fn is_read_error(&self) -> bool {
        matches!(self, ConfigFileError::Read { .. })
    }

    /// Returns true if this error is a parse error (invalid JSON).
    pub fn is_parse_error(&self) -> bool {
        matches!(self, ConfigFileError::Parse { .. })
    }

    /// Returns the path to the config file that caused the error.
    pub fn path(&self) -> &Path {
        match self {
            ConfigFileError::Read { path, .. } => path,
            ConfigFileError::Parse { path, .. } => path,
        }
    }
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
            list_defaults: ListDefaults::default(),
            internal_logs_defaults: InternalLogsDefaults::default(),
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
            list_defaults: ListDefaults::default(),
            internal_logs_defaults: InternalLogsDefaults::default(),
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
    fn test_search_defaults_sanitize_empty_earliest_time() {
        let defaults = SearchDefaults {
            earliest_time: "".to_string(),
            latest_time: "now".to_string(),
            max_results: 1000,
        };

        let sanitized = defaults.sanitize();
        assert_eq!(sanitized.earliest_time, "-24h");
        assert_eq!(sanitized.latest_time, "now");
        assert_eq!(sanitized.max_results, 1000);
    }

    #[test]
    fn test_search_defaults_sanitize_whitespace_earliest_time() {
        let defaults = SearchDefaults {
            earliest_time: "   ".to_string(),
            latest_time: "now".to_string(),
            max_results: 1000,
        };

        let sanitized = defaults.sanitize();
        assert_eq!(sanitized.earliest_time, "-24h");
        assert_eq!(sanitized.latest_time, "now");
        assert_eq!(sanitized.max_results, 1000);
    }

    #[test]
    fn test_search_defaults_sanitize_empty_latest_time() {
        let defaults = SearchDefaults {
            earliest_time: "-24h".to_string(),
            latest_time: "".to_string(),
            max_results: 1000,
        };

        let sanitized = defaults.sanitize();
        assert_eq!(sanitized.earliest_time, "-24h");
        assert_eq!(sanitized.latest_time, "now");
        assert_eq!(sanitized.max_results, 1000);
    }

    #[test]
    fn test_search_defaults_sanitize_whitespace_latest_time() {
        let defaults = SearchDefaults {
            earliest_time: "-24h".to_string(),
            latest_time: "   ".to_string(),
            max_results: 1000,
        };

        let sanitized = defaults.sanitize();
        assert_eq!(sanitized.earliest_time, "-24h");
        assert_eq!(sanitized.latest_time, "now");
        assert_eq!(sanitized.max_results, 1000);
    }

    #[test]
    fn test_search_defaults_sanitize_zero_max_results() {
        let defaults = SearchDefaults {
            earliest_time: "-24h".to_string(),
            latest_time: "now".to_string(),
            max_results: 0,
        };

        let sanitized = defaults.sanitize();
        assert_eq!(sanitized.earliest_time, "-24h");
        assert_eq!(sanitized.latest_time, "now");
        assert_eq!(sanitized.max_results, 1000);
    }

    #[test]
    fn test_search_defaults_sanitize_multiple_invalid() {
        let defaults = SearchDefaults {
            earliest_time: "".to_string(),
            latest_time: "   ".to_string(),
            max_results: 0,
        };

        let sanitized = defaults.sanitize();
        assert_eq!(sanitized.earliest_time, "-24h");
        assert_eq!(sanitized.latest_time, "now");
        assert_eq!(sanitized.max_results, 1000);
    }

    #[test]
    fn test_search_defaults_sanitize_valid_values_unchanged() {
        let defaults = SearchDefaults {
            earliest_time: "-7d".to_string(),
            latest_time: "2024-01-01T00:00:00".to_string(),
            max_results: 500,
        };

        let sanitized = defaults.sanitize();
        assert_eq!(sanitized.earliest_time, "-7d");
        assert_eq!(sanitized.latest_time, "2024-01-01T00:00:00");
        assert_eq!(sanitized.max_results, 500);
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
            list_defaults: ListDefaults::default(),
            internal_logs_defaults: InternalLogsDefaults::default(),
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
            list_defaults: ListDefaults::default(),
            internal_logs_defaults: InternalLogsDefaults::default(),
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

    #[test]
    fn test_list_defaults_default() {
        let defaults = ListDefaults::default();
        assert_eq!(defaults.page_size, DEFAULT_LIST_PAGE_SIZE);
        assert_eq!(defaults.max_items, DEFAULT_LIST_MAX_ITEMS);
        assert!(defaults.indexes_page_size.is_none());
        assert!(defaults.jobs_page_size.is_none());
        assert!(defaults.apps_page_size.is_none());
        assert!(defaults.users_page_size.is_none());
        assert!(defaults.roles_page_size.is_none());
    }

    #[test]
    fn test_list_defaults_page_size_for_with_overrides() {
        let defaults = ListDefaults {
            page_size: 100,
            max_items: 1000,
            indexes_page_size: Some(50),
            jobs_page_size: Some(200),
            apps_page_size: None,
            users_page_size: Some(75),
            roles_page_size: None,
        };

        assert_eq!(defaults.page_size_for(ListType::Indexes), 50);
        assert_eq!(defaults.page_size_for(ListType::Jobs), 200);
        assert_eq!(defaults.page_size_for(ListType::Apps), 100); // Falls back to default
        assert_eq!(defaults.page_size_for(ListType::Users), 75);
    }

    #[test]
    fn test_list_defaults_page_size_for_no_overrides() {
        let defaults = ListDefaults {
            page_size: 100,
            max_items: 1000,
            indexes_page_size: None,
            jobs_page_size: None,
            apps_page_size: None,
            users_page_size: None,
            roles_page_size: None,
        };

        assert_eq!(defaults.page_size_for(ListType::Indexes), 100);
        assert_eq!(defaults.page_size_for(ListType::Jobs), 100);
        assert_eq!(defaults.page_size_for(ListType::Apps), 100);
        assert_eq!(defaults.page_size_for(ListType::Users), 100);
        assert_eq!(defaults.page_size_for(ListType::Roles), 100);
    }

    #[test]
    fn test_list_defaults_serialization() {
        let defaults = ListDefaults {
            page_size: 50,
            max_items: 500,
            indexes_page_size: Some(25),
            jobs_page_size: Some(100),
            apps_page_size: None,
            users_page_size: None,
            roles_page_size: Some(30),
        };

        let json = serde_json::to_string(&defaults).unwrap();
        assert!(json.contains("50"));
        assert!(json.contains("500"));
        assert!(json.contains("25"));
        assert!(json.contains("100"));

        let deserialized: ListDefaults = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.page_size, 50);
        assert_eq!(deserialized.max_items, 500);
        assert_eq!(deserialized.indexes_page_size, Some(25));
        assert_eq!(deserialized.jobs_page_size, Some(100));
        assert_eq!(deserialized.apps_page_size, None);
    }

    #[test]
    fn test_list_defaults_deserialization_uses_defaults_for_missing_fields() {
        // Test that missing fields use default values
        let json = r#"{}"#;
        let deserialized: ListDefaults = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.page_size, DEFAULT_LIST_PAGE_SIZE);
        assert_eq!(deserialized.max_items, DEFAULT_LIST_MAX_ITEMS);
        assert!(deserialized.indexes_page_size.is_none());
    }

    #[test]
    fn test_persisted_state_backward_compatibility_without_list_defaults() {
        // Simulate an old config file without list_defaults
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
            },
            "keybind_overrides": {}
        }"#;

        let deserialized: PersistedState = serde_json::from_str(json).unwrap();
        // Should use defaults for missing list_defaults
        assert_eq!(deserialized.list_defaults.page_size, DEFAULT_LIST_PAGE_SIZE);
        assert_eq!(deserialized.list_defaults.max_items, DEFAULT_LIST_MAX_ITEMS);
    }

    #[test]
    fn test_persisted_state_with_list_defaults_round_trip() {
        let state = PersistedState {
            auto_refresh: true,
            sort_column: "status".to_string(),
            sort_direction: "desc".to_string(),
            last_search_query: None,
            search_history: vec![],
            selected_theme: ColorTheme::Default,
            search_defaults: SearchDefaults::default(),
            keybind_overrides: KeybindOverrides::default(),
            list_defaults: ListDefaults {
                page_size: 75,
                max_items: 750,
                indexes_page_size: Some(50),
                jobs_page_size: Some(100),
                apps_page_size: None,
                users_page_size: Some(25),
                roles_page_size: None,
            },
            internal_logs_defaults: InternalLogsDefaults::default(),
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: PersistedState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.list_defaults.page_size, 75);
        assert_eq!(deserialized.list_defaults.max_items, 750);
        assert_eq!(deserialized.list_defaults.indexes_page_size, Some(50));
        assert_eq!(deserialized.list_defaults.jobs_page_size, Some(100));
        assert_eq!(deserialized.list_defaults.apps_page_size, None);
        assert_eq!(deserialized.list_defaults.users_page_size, Some(25));
    }

    #[test]
    fn test_internal_logs_defaults_default() {
        let defaults = InternalLogsDefaults::default();
        assert_eq!(defaults.count, 100);
        assert_eq!(defaults.earliest_time, "-15m");
    }

    #[test]
    fn test_internal_logs_defaults_serialization() {
        let defaults = InternalLogsDefaults {
            count: 50,
            earliest_time: "-1h".to_string(),
        };

        let json = serde_json::to_string(&defaults).unwrap();
        assert!(json.contains("50"));
        assert!(json.contains("-1h"));

        let deserialized: InternalLogsDefaults = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.count, 50);
        assert_eq!(deserialized.earliest_time, "-1h");
    }

    #[test]
    fn test_internal_logs_defaults_deserialization_uses_defaults_for_missing_fields() {
        // Test that missing fields use default values
        let json = r#"{}"#;
        let deserialized: InternalLogsDefaults = serde_json::from_str(json).unwrap();
        assert_eq!(deserialized.count, 100);
        assert_eq!(deserialized.earliest_time, "-15m");
    }

    #[test]
    fn test_persisted_state_backward_compatibility_without_internal_logs_defaults() {
        // Simulate an old config file without internal_logs_defaults
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
            },
            "keybind_overrides": {},
            "list_defaults": {
                "page_size": 100,
                "max_items": 1000
            }
        }"#;

        let deserialized: PersistedState = serde_json::from_str(json).unwrap();
        // Should use defaults for missing internal_logs_defaults
        assert_eq!(deserialized.internal_logs_defaults.count, 100);
        assert_eq!(deserialized.internal_logs_defaults.earliest_time, "-15m");
    }

    #[test]
    fn test_persisted_state_with_internal_logs_defaults_round_trip() {
        let state = PersistedState {
            auto_refresh: true,
            sort_column: "status".to_string(),
            sort_direction: "desc".to_string(),
            last_search_query: None,
            search_history: vec![],
            selected_theme: ColorTheme::Default,
            search_defaults: SearchDefaults::default(),
            keybind_overrides: KeybindOverrides::default(),
            list_defaults: ListDefaults::default(),
            internal_logs_defaults: InternalLogsDefaults {
                count: 200,
                earliest_time: "-30m".to_string(),
            },
        };

        let json = serde_json::to_string(&state).unwrap();
        let deserialized: PersistedState = serde_json::from_str(&json).unwrap();

        assert_eq!(deserialized.internal_logs_defaults.count, 200);
        assert_eq!(deserialized.internal_logs_defaults.earliest_time, "-30m");
    }

    #[test]
    fn test_internal_logs_defaults_sanitize_zero_count() {
        let defaults = InternalLogsDefaults {
            count: 0,
            earliest_time: "-15m".to_string(),
        };

        let sanitized = defaults.sanitize();
        assert_eq!(sanitized.count, 100);
        assert_eq!(sanitized.earliest_time, "-15m");
    }

    #[test]
    fn test_internal_logs_defaults_sanitize_empty_earliest_time() {
        let defaults = InternalLogsDefaults {
            count: 50,
            earliest_time: "".to_string(),
        };

        let sanitized = defaults.sanitize();
        assert_eq!(sanitized.count, 50);
        assert_eq!(sanitized.earliest_time, "-15m");
    }

    #[test]
    fn test_internal_logs_defaults_sanitize_whitespace_earliest_time() {
        let defaults = InternalLogsDefaults {
            count: 50,
            earliest_time: "   ".to_string(),
        };

        let sanitized = defaults.sanitize();
        assert_eq!(sanitized.count, 50);
        assert_eq!(sanitized.earliest_time, "-15m");
    }

    #[test]
    fn test_internal_logs_defaults_sanitize_multiple_invalid() {
        let defaults = InternalLogsDefaults {
            count: 0,
            earliest_time: "".to_string(),
        };

        let sanitized = defaults.sanitize();
        assert_eq!(sanitized.count, 100);
        assert_eq!(sanitized.earliest_time, "-15m");
    }

    #[test]
    fn test_internal_logs_defaults_sanitize_valid_values_unchanged() {
        let defaults = InternalLogsDefaults {
            count: 200,
            earliest_time: "-1h".to_string(),
        };

        let sanitized = defaults.sanitize();
        assert_eq!(sanitized.count, 200);
        assert_eq!(sanitized.earliest_time, "-1h");
    }
}
