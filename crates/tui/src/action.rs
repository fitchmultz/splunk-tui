//! Action protocol for async TUI event handling.
//!
//! This module defines the unified Action enum that replaces simple events.
//! Actions represent both user inputs and async API operation results.
//!
//! # Security Note
//!
//! When logging Actions, use `RedactedAction(&action)` wrapper instead of
//! `?action` Debug formatting to prevent sensitive payloads from being written
//! to log files. See `RedactedAction` documentation for details.

use crossterm::event::KeyEvent;
use serde_json::Value;
use splunk_client::models::{
    App as SplunkApp, ClusterInfo, ClusterPeer, HealthCheckOutput, Index, LogEntry, SavedSearch,
    SearchJobStatus, SplunkHealth, User,
};
use splunk_config::{PersistedState, SearchDefaults};
use std::path::PathBuf;
use tokio::sync::mpsc::UnboundedSender;

use crate::ConnectionContext;
use crate::ui::ToastLevel;

/// Supported export formats for search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
}

/// Redacted wrapper for Action that prevents sensitive payloads from being logged.
///
/// This wrapper implements `Debug` to replace sensitive string payloads with
/// size indicators (e.g., `<42 chars>`) while preserving non-sensitive
/// information for debugging purposes.
///
/// # Example
/// ```ignore
/// use splunk_config::SearchDefaults;
/// let action = Action::RunSearch {
///     query: "SELECT * FROM users WHERE password='secret'".to_string(),
///     search_defaults: SearchDefaults::default(),
/// };
/// tracing::info!("Handling action: {:?}", RedactedAction(&action));
/// // Logs: Handling action: RunSearch(<52 chars>)
/// ```
pub struct RedactedAction<'a>(pub &'a Action);

impl std::fmt::Debug for RedactedAction<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self.0 {
            Action::RunSearch { query, .. } => {
                write!(f, "RunSearch(<{} chars>)", query.len())
            }
            Action::CopyToClipboard(text) => {
                write!(f, "CopyToClipboard(<{} chars>)", text.len())
            }
            Action::ExportData(data, path, format) => {
                let data_size = data.to_string().len();
                write!(
                    f,
                    "ExportData(<{} bytes>, {:?}, {:?})",
                    data_size, path, format
                )
            }
            Action::Notify(level, message) => {
                write!(f, "Notify({:?}, <{} chars>)", level, message.len())
            }
            Action::CancelJob(sid) => write!(f, "CancelJob({})", sid),
            Action::DeleteJob(sid) => write!(f, "DeleteJob({})", sid),
            Action::CancelJobsBatch(sids) => {
                write!(f, "CancelJobsBatch([{} job(s)])", sids.len())
            }
            Action::DeleteJobsBatch(sids) => {
                write!(f, "DeleteJobsBatch([{} job(s)])", sids.len())
            }
            Action::EnableApp(name) => write!(f, "EnableApp({})", name),
            Action::DisableApp(name) => write!(f, "DisableApp({})", name),
            Action::SearchInput(c) => write!(f, "SearchInput({:?})", c),

            // Search-related actions with sensitive data
            Action::SearchStarted(query) => {
                write!(f, "SearchStarted(<{} chars>)", query.len())
            }
            Action::SearchComplete(result) => match result {
                Ok((results, sid, total)) => {
                    write!(
                        f,
                        "SearchComplete(<{} results>, sid={}, total={:?})",
                        results.len(),
                        sid,
                        total
                    )
                }
                Err(_) => write!(f, "SearchComplete(<error>)"),
            },
            Action::MoreSearchResultsLoaded(result) => match result {
                Ok((results, offset, total)) => {
                    write!(
                        f,
                        "MoreSearchResultsLoaded(<{} results>, offset={}, total={:?})",
                        results.len(),
                        offset,
                        total
                    )
                }
                Err(_) => write!(f, "MoreSearchResultsLoaded(<error>)"),
            },

            // Data-loaded actions - show item count, not content
            Action::IndexesLoaded(result) => match result {
                Ok(items) => write!(f, "IndexesLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "IndexesLoaded(<error>)"),
            },
            Action::JobsLoaded(result) => match result {
                Ok(items) => write!(f, "JobsLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "JobsLoaded(<error>)"),
            },
            Action::ClusterInfoLoaded(result) => match result {
                Ok(_) => write!(f, "ClusterInfoLoaded(<data>)"),
                Err(_) => write!(f, "ClusterInfoLoaded(<error>)"),
            },
            Action::HealthLoaded(result) => match result.as_ref() {
                Ok(_) => write!(f, "HealthLoaded(<data>)"),
                Err(_) => write!(f, "HealthLoaded(<error>)"),
            },
            Action::SavedSearchesLoaded(result) => match result {
                Ok(items) => write!(f, "SavedSearchesLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "SavedSearchesLoaded(<error>)"),
            },
            Action::InternalLogsLoaded(result) => match result {
                Ok(items) => write!(f, "InternalLogsLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "InternalLogsLoaded(<error>)"),
            },
            Action::AppsLoaded(result) => match result {
                Ok(items) => write!(f, "AppsLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "AppsLoaded(<error>)"),
            },
            Action::UsersLoaded(result) => match result {
                Ok(items) => write!(f, "UsersLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "UsersLoaded(<error>)"),
            },
            Action::ClusterPeersLoaded(result) => match result {
                Ok(items) => write!(f, "ClusterPeersLoaded(<{} items>)", items.len()),
                Err(_) => write!(f, "ClusterPeersLoaded(<error>)"),
            },
            Action::HealthStatusLoaded(result) => match result {
                Ok(_) => write!(f, "HealthStatusLoaded(<data>)"),
                Err(_) => write!(f, "HealthStatusLoaded(<error>)"),
            },

            // Profile-related actions
            Action::OpenProfileSelectorWithList(profiles) => {
                write!(
                    f,
                    "OpenProfileSelectorWithList(<{} profiles>)",
                    profiles.len()
                )
            }
            Action::ProfileSwitchResult(result) => match result {
                Ok(_) => write!(f, "ProfileSwitchResult(Ok)"),
                Err(_) => write!(f, "ProfileSwitchResult(Err)"),
            },
            Action::ProfileSelected(_) => write!(f, "ProfileSelected(<redacted>)"),

            // Settings-loaded action contains search history that may have sensitive queries
            Action::SettingsLoaded(_) => write!(f, "SettingsLoaded(<redacted>)"),

            // Error details may contain sensitive URLs, queries, or response data
            Action::ShowErrorDetails(_) => write!(f, "ShowErrorDetails(<redacted>)"),
            Action::ShowErrorDetailsFromCurrent => write!(f, "ShowErrorDetailsFromCurrent"),

            // Non-sensitive simple actions - fall through to default Debug
            other => write!(f, "{:?}", other),
        }
    }
}

/// Unified action type for async TUI event handling.
///
/// Actions flow through a channel from input handlers and async tasks
/// to the main app state, where they trigger state mutations.
#[derive(Debug, Clone)]
pub enum Action {
    // System
    /// Quit the application
    Quit,

    /// Open the help popup.
    OpenHelpPopup,
    /// Switch to the Search screen without triggering a load.
    SwitchToSearch,
    /// Switch to the Settings screen without reloading settings.
    SwitchToSettingsScreen,
    /// Navigate to the next screen in cyclic order.
    NextScreen,
    /// Navigate to the previous screen in cyclic order.
    PreviousScreen,

    // Input
    /// Raw keyboard input event
    Input(KeyEvent),
    /// Raw mouse input event
    Mouse(crossterm::event::MouseEvent),
    /// Navigate down in current list/table
    NavigateDown,
    /// Navigate up in current list/table
    NavigateUp,
    /// Page down in current view
    PageDown,
    /// Page up in current view
    PageUp,
    /// Jump to top of list
    GoToTop,
    /// Jump to bottom of list
    GoToBottom,
    /// Enter search/filter mode for jobs
    EnterSearchMode,
    /// Add a character to the search filter
    SearchInput(char),
    /// Clear the search filter
    ClearSearch,
    /// Cycle sort column for jobs
    CycleSortColumn,
    /// Toggle sort direction for jobs
    ToggleSortDirection,

    /// Cycle through the available color themes (Settings screen).
    CycleTheme,

    /// Copy the provided text to the system clipboard.
    ///
    /// This is emitted by per-screen input handlers (Ctrl+C) and executed by the app.
    CopyToClipboard(String),

    // API Triggers
    /// Load the list of indexes
    LoadIndexes,
    /// Load the list of search jobs
    LoadJobs,
    /// Load cluster information
    LoadClusterInfo,
    /// Load health check information
    LoadHealth,
    /// Load the list of saved searches
    LoadSavedSearches,
    /// Load internal logs from index=_internal
    LoadInternalLogs,
    /// Load the list of apps
    LoadApps,
    /// Load the list of users
    LoadUsers,
    /// Load cluster peers (detailed view)
    LoadClusterPeers,
    /// Switch to settings screen
    SwitchToSettings,
    /// Toggle cluster view mode (Summary <-> Peers)
    ToggleClusterViewMode,
    /// Run a search with the given query and search defaults.
    ///
    /// The search defaults (earliest_time, latest_time, max_results) are passed
    /// explicitly to ensure environment variable overrides are applied correctly.
    RunSearch {
        query: String,
        search_defaults: SearchDefaults,
    },
    /// Export data (pre-serialized as JSON) to a file.
    ///
    /// This payload is produced by the UI state machine so the main event loop
    /// can export without needing access to `App` state.
    ExportData(Value, PathBuf, ExportFormat),

    // API Results
    /// Result of loading indexes
    IndexesLoaded(Result<Vec<Index>, String>),
    /// Result of loading jobs
    JobsLoaded(Result<Vec<SearchJobStatus>, String>),
    /// Result of loading cluster info
    ClusterInfoLoaded(Result<ClusterInfo, String>),
    /// Result of loading health check
    HealthLoaded(Box<Result<HealthCheckOutput, String>>),
    /// Result of loading saved searches
    SavedSearchesLoaded(Result<Vec<SavedSearch>, String>),
    /// Result of loading internal logs
    InternalLogsLoaded(Result<Vec<LogEntry>, String>),
    /// Result of loading apps
    AppsLoaded(Result<Vec<SplunkApp>, String>),
    /// Result of loading users
    UsersLoaded(Result<Vec<User>, String>),
    /// Result of loading cluster peers
    ClusterPeersLoaded(Result<Vec<ClusterPeer>, String>),
    /// Result of loading persisted settings
    SettingsLoaded(PersistedState),
    /// Result of background health status check
    HealthStatusLoaded(Result<SplunkHealth, String>),
    /// Signals that a search has started with the given query.
    /// Stores the query for accurate status messaging even if search_input is edited.
    SearchStarted(String),
    /// Result of a search completion (results, sid, total_count) or (error_msg, error_details)
    #[allow(clippy::type_complexity)]
    SearchComplete(
        Result<(Vec<Value>, String, Option<u64>), (String, crate::error_details::ErrorDetails)>,
    ),
    /// Load more results for the current search (pagination)
    LoadMoreSearchResults {
        sid: String,
        offset: u64,
        count: u64,
    },
    /// Result of loading more results
    MoreSearchResultsLoaded(Result<(Vec<Value>, u64, Option<u64>), String>),

    // Job Operations
    /// Cancel a job by SID
    CancelJob(String),
    /// Delete a job by SID
    DeleteJob(String),
    /// Cancel multiple jobs by SID
    CancelJobsBatch(Vec<String>),
    /// Delete multiple jobs by SID
    DeleteJobsBatch(Vec<String>),
    /// Job operation completed successfully
    JobOperationComplete(String),

    // App Operations
    /// Enable an app by name
    EnableApp(String),
    /// Disable an app by name
    DisableApp(String),
    /// Inspect currently selected job
    InspectJob,
    /// Exit job inspection mode
    ExitInspectMode,

    // Progress
    /// Update progress indicator (0.0 - 1.0)
    Progress(f32),
    /// Set loading state
    Loading(bool),

    // Notifications
    /// Display a toast notification
    Notify(ToastLevel, String),
    /// Tick event for periodic updates (TTL pruning, animations)
    Tick,

    // Error handling
    /// Display error details popup
    ShowErrorDetails(crate::error_details::ErrorDetails),
    /// Show error details from current_error (when user presses 'e')
    ShowErrorDetailsFromCurrent,
    /// Clear current error details (when popup is dismissed)
    ClearErrorDetails,

    // Profile Switching
    /// Open the profile selector popup
    OpenProfileSwitcher,
    /// Open profile selector with list of profiles (sent from main.rs side effects)
    OpenProfileSelectorWithList(Vec<String>),
    /// Profile selected by user (contains profile name)
    ProfileSelected(String),
    /// Result of profile switch operation (contains new connection context or error)
    ProfileSwitchResult(Result<ConnectionContext, String>),
    /// Clear all cached data after profile switch
    ClearAllData,
}

/// Creates a progress callback that bridges the client's synchronous `FnMut(f64)`
/// to the TUI's async `UnboundedSender<Action>` channel.
///
/// This allows the client's `search_with_progress` method to send progress updates
/// to the TUI event loop without blocking. Progress values are clamped to [0.0, 1.0]
/// and sent as `Action::Progress` messages.
///
/// # Arguments
///
/// * `tx` - The action sender channel to send progress updates to
///
/// # Returns
///
/// A closure that can be passed to `client.search_with_progress()` as the progress callback.
///
/// # Example
///
/// ```ignore
/// let progress_tx = tx.clone();
/// let mut progress_callback = progress_callback_to_action_sender(progress_tx);
///
/// let (results, sid, total) = client
///     .search_with_progress(query, true, earliest, latest, max_results, Some(&mut progress_callback))
///     .await?;
/// ```
pub fn progress_callback_to_action_sender(tx: UnboundedSender<Action>) -> impl FnMut(f64) + Send {
    move |progress: f64| {
        // Clamp progress to valid range [0.0, 1.0]
        let clamped = progress.clamp(0.0, 1.0);
        // Send progress as f32 (TUI uses f32 for Progress action)
        let _ = tx.send(Action::Progress(clamped as f32));
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn redacted_debug(action: &Action) -> String {
        format!("{:?}", RedactedAction(action))
    }

    #[test]
    fn test_redact_run_search() {
        let action = Action::RunSearch {
            query: "SELECT * FROM users WHERE password='secret'".to_string(),
            search_defaults: SearchDefaults::default(),
        };
        let output = redacted_debug(&action);

        assert!(
            !output.contains("password"),
            "Should not contain sensitive password"
        );
        assert!(!output.contains("secret"), "Should not contain secret word");
        assert!(output.contains("RunSearch"), "Should contain action name");
        assert!(output.contains("43 chars"), "Should show size indicator");
    }

    #[test]
    fn test_redact_copy_to_clipboard() {
        let action =
            Action::CopyToClipboard("{\"user\":\"alice\",\"token\":\"abc123\"}".to_string());
        let output = redacted_debug(&action);

        assert!(!output.contains("alice"), "Should not contain user name");
        assert!(!output.contains("abc123"), "Should not contain token");
        assert!(
            output.contains("CopyToClipboard"),
            "Should contain action name"
        );
        assert!(output.contains("33 chars"), "Should show size indicator");
    }

    #[test]
    fn test_redact_export_data() {
        let data = serde_json::json!({"results": [{"id": 1, "password": "secret123"}]});
        let path = PathBuf::from("/tmp/export.json");
        let action = Action::ExportData(data.clone(), path, ExportFormat::Json);
        let output = redacted_debug(&action);

        assert!(
            !output.contains("secret123"),
            "Should not contain sensitive data"
        );
        assert!(output.contains("ExportData"), "Should contain action name");
        assert!(output.contains("bytes"), "Should show bytes indicator");
    }

    #[test]
    fn test_redact_notify() {
        let action = Action::Notify(
            ToastLevel::Error,
            "Failed to authenticate: invalid token xyz789".to_string(),
        );
        let output = redacted_debug(&action);

        assert!(!output.contains("xyz789"), "Should not contain token");
        assert!(output.contains("Notify"), "Should contain action name");
        assert!(output.contains("Error"), "Should contain toast level");
        assert!(output.contains("chars"), "Should show size indicator");
    }

    #[test]
    fn test_show_cancel_job_sid() {
        let action = Action::CancelJob("search_job_12345_789".to_string());
        let output = redacted_debug(&action);

        assert!(output.contains("CancelJob"), "Should contain action name");
        assert!(
            output.contains("search_job_12345_789"),
            "Should show SID for debugging"
        );
    }

    #[test]
    fn test_show_delete_job_sid() {
        let action = Action::DeleteJob("search_job_98765_4321".to_string());
        let output = redacted_debug(&action);

        assert!(output.contains("DeleteJob"), "Should contain action name");
        assert!(
            output.contains("search_job_98765_4321"),
            "Should show SID for debugging"
        );
    }

    #[test]
    fn test_show_batch_operation_counts() {
        let sids = vec!["job1".to_string(), "job2".to_string(), "job3".to_string()];
        let action = Action::CancelJobsBatch(sids);
        let output = redacted_debug(&action);

        assert!(
            output.contains("CancelJobsBatch"),
            "Should contain action name"
        );
        assert!(
            output.contains("3 job(s)"),
            "Should show count but not SIDs"
        );
        assert!(!output.contains("job1"), "Should not show individual SIDs");
    }

    #[test]
    fn test_show_search_input() {
        let action = Action::SearchInput('s');
        let output = redacted_debug(&action);

        assert!(output.contains("SearchInput"), "Should contain action name");
        assert!(
            output.contains("'s'"),
            "Should show character for input debugging"
        );
    }

    #[test]
    fn test_non_sensitive_action_shown_fully() {
        let action = Action::Quit;
        let output = redacted_debug(&action);

        assert!(output.contains("Quit"), "Should show simple action fully");
    }

    #[test]
    fn test_unicode_in_payload() {
        let action = Action::CopyToClipboard("日本語テスト 🇯🇵".to_string());
        let output = redacted_debug(&action);

        assert!(
            !output.contains("日本語"),
            "Should not contain Unicode content"
        );
        assert!(output.contains("chars"), "Should show character count");
    }

    #[test]
    fn test_redact_search_started() {
        let action =
            Action::SearchStarted("SELECT * FROM users WHERE password='secret'".to_string());
        let output = redacted_debug(&action);

        assert!(
            !output.contains("password"),
            "Should not contain sensitive password"
        );
        assert!(!output.contains("secret"), "Should not contain secret word");
        assert!(
            output.contains("SearchStarted"),
            "Should contain action name"
        );
        assert!(output.contains("43 chars"), "Should show size indicator");
    }

    #[test]
    fn test_redact_search_complete_ok() {
        let results = vec![
            serde_json::json!({"password": "secret123", "user": "admin"}),
            serde_json::json!({"token": "abc456", "user": "bob"}),
        ];
        let action =
            Action::SearchComplete(Ok((results, "search_job_12345".to_string(), Some(100))));
        let output = redacted_debug(&action);

        assert!(
            !output.contains("secret123"),
            "Should not contain sensitive data from results"
        );
        assert!(!output.contains("abc456"), "Should not contain token");
        assert!(
            !output.contains("admin"),
            "Should not contain user names from results"
        );
        assert!(
            output.contains("SearchComplete"),
            "Should contain action name"
        );
        assert!(output.contains("2 results"), "Should show result count");
        assert!(output.contains("sid=search_job_12345"), "Should show SID");
        assert!(
            output.contains("total=Some(100)"),
            "Should show total count"
        );
    }

    #[test]
    fn test_redact_search_complete_err() {
        let action = Action::SearchComplete(Err((
            "Authentication failed for user admin".to_string(),
            crate::error_details::ErrorDetails::from_error_string("auth failed"),
        )));
        let output = redacted_debug(&action);

        assert!(
            !output.contains("Authentication failed"),
            "Should not contain error message"
        );
        assert!(!output.contains("admin"), "Should not contain user name");
        assert!(
            output.contains("SearchComplete"),
            "Should contain action name"
        );
        assert!(output.contains("<error>"), "Should show error indicator");
    }

    #[test]
    fn test_redact_more_search_results_loaded_ok() {
        let results = vec![serde_json::json!({"password": "secret789", "data": "sensitive"})];
        let action = Action::MoreSearchResultsLoaded(Ok((results, 50, Some(200))));
        let output = redacted_debug(&action);

        assert!(
            !output.contains("secret789"),
            "Should not contain sensitive data"
        );
        assert!(
            !output.contains("sensitive"),
            "Should not contain data content"
        );
        assert!(
            output.contains("MoreSearchResultsLoaded"),
            "Should contain action name"
        );
        assert!(output.contains("1 results"), "Should show result count");
        assert!(output.contains("offset=50"), "Should show offset");
        assert!(output.contains("total=Some(200)"), "Should show total");
    }

    #[test]
    fn test_redact_more_search_results_loaded_err() {
        let action = Action::MoreSearchResultsLoaded(Err(
            "Failed to fetch results for user bob".to_string()
        ));
        let output = redacted_debug(&action);

        assert!(
            !output.contains("Failed to fetch"),
            "Should not contain error message"
        );
        assert!(!output.contains("bob"), "Should not contain user name");
        assert!(
            output.contains("MoreSearchResultsLoaded"),
            "Should contain action name"
        );
        assert!(output.contains("<error>"), "Should show error indicator");
    }

    #[test]
    fn test_redact_indexes_loaded() {
        let indexes = vec![
            Index {
                name: "internal".to_string(),
                max_total_data_size_mb: None,
                current_db_size_mb: 100,
                total_event_count: 1000,
                max_warm_db_count: None,
                max_hot_buckets: None,
                frozen_time_period_in_secs: None,
                cold_db_path: None,
                home_path: None,
                thawed_path: None,
                cold_to_frozen_dir: None,
                primary_index: None,
            },
            Index {
                name: "main".to_string(),
                max_total_data_size_mb: None,
                current_db_size_mb: 200,
                total_event_count: 2000,
                max_warm_db_count: None,
                max_hot_buckets: None,
                frozen_time_period_in_secs: None,
                cold_db_path: None,
                home_path: None,
                thawed_path: None,
                cold_to_frozen_dir: None,
                primary_index: None,
            },
        ];
        let action = Action::IndexesLoaded(Ok(indexes));
        let output = redacted_debug(&action);

        assert!(
            !output.contains("internal"),
            "Should not contain index name"
        );
        assert!(
            !output.contains("/opt/splunk"),
            "Should not contain path data"
        );
        assert!(
            output.contains("IndexesLoaded"),
            "Should contain action name"
        );
        assert!(output.contains("2 items"), "Should show item count");
    }

    #[test]
    fn test_redact_indexes_loaded_err() {
        let action = Action::IndexesLoaded(Err("Failed to load indexes".to_string()));
        let output = redacted_debug(&action);

        assert!(
            !output.contains("Failed to load"),
            "Should not contain error message"
        );
        assert!(
            output.contains("IndexesLoaded"),
            "Should contain action name"
        );
        assert!(output.contains("<error>"), "Should show error indicator");
    }

    #[test]
    fn test_redact_jobs_loaded() {
        let jobs = vec![
            SearchJobStatus {
                sid: "job1".to_string(),
                is_done: true,
                is_finalized: true,
                done_progress: 1.0,
                run_duration: 1.0,
                cursor_time: None,
                scan_count: 100,
                event_count: 50,
                result_count: 25,
                disk_usage: 1024,
                priority: None,
                label: None,
            },
            SearchJobStatus {
                sid: "job2".to_string(),
                is_done: false,
                is_finalized: false,
                done_progress: 0.5,
                run_duration: 0.5,
                cursor_time: None,
                scan_count: 50,
                event_count: 25,
                result_count: 10,
                disk_usage: 512,
                priority: None,
                label: None,
            },
        ];
        let action = Action::JobsLoaded(Ok(jobs));
        let output = redacted_debug(&action);

        assert!(!output.contains("job1"), "Should not contain job SID");
        assert!(!output.contains("job2"), "Should not contain job SID");
        assert!(output.contains("JobsLoaded"), "Should contain action name");
        assert!(output.contains("2 items"), "Should show item count");
    }

    #[test]
    fn test_redact_saved_searches_loaded() {
        let searches = vec![SavedSearch {
            name: "Admin Activity".to_string(),
            search: "search user=admin".to_string(),
            description: None,
            disabled: false,
        }];
        let action = Action::SavedSearchesLoaded(Ok(searches));
        let output = redacted_debug(&action);

        assert!(
            !output.contains("Admin Activity"),
            "Should not contain search name"
        );
        assert!(
            !output.contains("user=admin"),
            "Should not contain search query"
        );
        assert!(
            output.contains("SavedSearchesLoaded"),
            "Should contain action name"
        );
        assert!(output.contains("1 items"), "Should show item count");
    }

    #[test]
    fn test_redact_internal_logs_loaded() {
        let logs = vec![
            LogEntry {
                time: "2025-01-20T10:30:00.000Z".to_string(),
                index_time: "2025-01-20T10:30:01.000Z".to_string(),
                serial: None,
                level: "INFO".to_string(),
                component: "Auth".to_string(),
                message: "User admin logged in".to_string(),
            },
            LogEntry {
                time: "2025-01-20T10:31:00.000Z".to_string(),
                index_time: "2025-01-20T10:31:01.000Z".to_string(),
                serial: None,
                level: "INFO".to_string(),
                component: "Token".to_string(),
                message: "Token abc123 generated".to_string(),
            },
        ];
        let action = Action::InternalLogsLoaded(Ok(logs));
        let output = redacted_debug(&action);

        assert!(
            !output.contains("admin logged in"),
            "Should not contain log messages"
        );
        assert!(!output.contains("abc123"), "Should not contain token");
        assert!(
            output.contains("InternalLogsLoaded"),
            "Should contain action name"
        );
        assert!(output.contains("2 items"), "Should show item count");
    }

    #[test]
    fn test_redact_apps_loaded() {
        let apps = vec![SplunkApp {
            name: "search".to_string(),
            label: Some("Search & Reporting".to_string()),
            version: None,
            is_configured: None,
            is_visible: None,
            disabled: false,
            description: None,
            author: None,
        }];
        let action = Action::AppsLoaded(Ok(apps));
        let output = redacted_debug(&action);

        assert!(!output.contains("search"), "Should not contain app name");
        assert!(
            !output.contains("Search & Reporting"),
            "Should not contain app label"
        );
        assert!(output.contains("AppsLoaded"), "Should contain action name");
        assert!(output.contains("1 items"), "Should show item count");
    }

    #[test]
    fn test_redact_users_loaded() {
        let users = vec![User {
            name: "admin".to_string(),
            realname: Some("Administrator".to_string()),
            email: None,
            user_type: None,
            default_app: None,
            roles: vec![],
            last_successful_login: None,
        }];
        let action = Action::UsersLoaded(Ok(users));
        let output = redacted_debug(&action);

        assert!(!output.contains("admin"), "Should not contain username");
        assert!(
            !output.contains("Administrator"),
            "Should not contain real name"
        );
        assert!(output.contains("UsersLoaded"), "Should contain action name");
        assert!(output.contains("1 items"), "Should show item count");
    }

    #[test]
    fn test_redact_cluster_peers_loaded() {
        let peers = vec![
            ClusterPeer {
                id: "peer1-id".to_string(),
                label: Some("peer1".to_string()),
                status: "Up".to_string(),
                peer_state: "Active".to_string(),
                site: None,
                guid: "guid1".to_string(),
                host: "internal-host1".to_string(),
                port: 8080,
                replication_count: None,
                replication_status: None,
                bundle_replication_count: None,
                is_captain: None,
            },
            ClusterPeer {
                id: "peer2-id".to_string(),
                label: Some("peer2".to_string()),
                status: "Up".to_string(),
                peer_state: "Active".to_string(),
                site: None,
                guid: "guid2".to_string(),
                host: "internal-host2".to_string(),
                port: 8080,
                replication_count: None,
                replication_status: None,
                bundle_replication_count: None,
                is_captain: None,
            },
        ];
        let action = Action::ClusterPeersLoaded(Ok(peers));
        let output = redacted_debug(&action);

        assert!(!output.contains("peer1"), "Should not contain peer name");
        assert!(
            !output.contains("internal-host1"),
            "Should not contain host"
        );
        assert!(
            output.contains("ClusterPeersLoaded"),
            "Should contain action name"
        );
        assert!(output.contains("2 items"), "Should show item count");
    }

    #[test]
    fn test_redact_cluster_info_loaded() {
        let info = ClusterInfo {
            id: "cluster1-id".to_string(),
            label: Some("cluster1".to_string()),
            mode: "master".to_string(),
            manager_uri: None,
            replication_factor: None,
            search_factor: None,
            status: None,
        };
        let action = Action::ClusterInfoLoaded(Ok(info));
        let output = redacted_debug(&action);

        assert!(
            !output.contains("cluster1"),
            "Should not contain cluster name"
        );
        assert!(
            output.contains("ClusterInfoLoaded"),
            "Should contain action name"
        );
        assert!(output.contains("<data>"), "Should show data indicator");
    }

    #[test]
    fn test_redact_health_loaded() {
        let health = HealthCheckOutput {
            server_info: None,
            splunkd_health: None,
            license_usage: None,
            kvstore_status: None,
            log_parsing_health: None,
        };
        let action = Action::HealthLoaded(Box::new(Ok(health)));
        let output = redacted_debug(&action);

        assert!(
            output.contains("HealthLoaded"),
            "Should contain action name"
        );
        assert!(output.contains("<data>"), "Should show data indicator");
    }

    #[test]
    fn test_redact_health_status_loaded() {
        let health = SplunkHealth {
            health: "yellow".to_string(),
            features: std::collections::HashMap::new(),
        };
        let action = Action::HealthStatusLoaded(Ok(health));
        let output = redacted_debug(&action);

        assert!(!output.contains("yellow"), "Should not contain status");
        assert!(
            output.contains("HealthStatusLoaded"),
            "Should contain action name"
        );
        assert!(output.contains("<data>"), "Should show data indicator");
    }

    #[test]
    fn test_redact_open_profile_selector_with_list() {
        let profiles = vec![
            "production".to_string(),
            "staging".to_string(),
            "admin-profile".to_string(),
        ];
        let action = Action::OpenProfileSelectorWithList(profiles);
        let output = redacted_debug(&action);

        assert!(
            !output.contains("production"),
            "Should not contain profile name"
        );
        assert!(
            !output.contains("admin-profile"),
            "Should not contain admin profile name"
        );
        assert!(
            output.contains("OpenProfileSelectorWithList"),
            "Should contain action name"
        );
        assert!(output.contains("3 profiles"), "Should show profile count");
    }

    #[test]
    fn test_redact_profile_switch_result_ok() {
        let action = Action::ProfileSwitchResult(Ok(ConnectionContext::default()));
        let output = redacted_debug(&action);

        assert!(
            output.contains("ProfileSwitchResult"),
            "Should contain action name"
        );
        assert!(output.contains("Ok"), "Should show Ok");
        assert!(
            !output.contains("ConnectionContext"),
            "Should not contain ConnectionContext details"
        );
    }

    #[test]
    fn test_redact_profile_switch_result_err() {
        let action =
            Action::ProfileSwitchResult(Err("Failed to connect with token abc123".to_string()));
        let output = redacted_debug(&action);

        assert!(
            !output.contains("Failed to connect"),
            "Should not contain error message"
        );
        assert!(!output.contains("abc123"), "Should not contain token");
        assert!(
            output.contains("ProfileSwitchResult"),
            "Should contain action name"
        );
        assert!(output.contains("Err"), "Should show Err");
    }

    #[test]
    fn test_redact_profile_selected() {
        let action = Action::ProfileSelected("production-admin".to_string());
        let output = redacted_debug(&action);

        assert!(
            !output.contains("production-admin"),
            "Should not contain profile name"
        );
        assert!(
            output.contains("ProfileSelected"),
            "Should contain action name"
        );
        assert!(
            output.contains("<redacted>"),
            "Should show redacted indicator"
        );
    }

    #[test]
    fn test_redact_settings_loaded() {
        use splunk_config::SearchDefaults;

        let state = PersistedState {
            auto_refresh: true,
            sort_column: "sid".to_string(),
            sort_direction: "asc".to_string(),
            last_search_query: Some("password='secret123'".to_string()),
            search_history: vec![
                "search user=admin".to_string(),
                "password='abc456'".to_string(),
            ],
            selected_theme: splunk_config::ColorTheme::Dark,
            search_defaults: SearchDefaults::default(),
            keybind_overrides: splunk_config::KeybindOverrides::default(),
        };
        let action = Action::SettingsLoaded(state);
        let output = redacted_debug(&action);

        assert!(
            !output.contains("secret123"),
            "Should not contain sensitive query data"
        );
        assert!(
            !output.contains("password"),
            "Should not contain password keyword"
        );
        assert!(
            !output.contains("admin"),
            "Should not contain user name from search history"
        );
        assert!(
            output.contains("SettingsLoaded"),
            "Should contain action name"
        );
        assert!(
            output.contains("<redacted>"),
            "Should show redacted indicator"
        );
    }

    #[test]
    fn test_redact_show_error_details() {
        let details = crate::error_details::ErrorDetails::from_error_string(
            "Authentication failed for user admin with password secret123",
        );
        let action = Action::ShowErrorDetails(details);
        let output = redacted_debug(&action);

        assert!(
            !output.contains("Authentication failed"),
            "Should not contain error message"
        );
        assert!(!output.contains("admin"), "Should not contain user name");
        assert!(!output.contains("secret123"), "Should not contain password");
        assert!(
            output.contains("ShowErrorDetails"),
            "Should contain action name"
        );
        assert!(
            output.contains("<redacted>"),
            "Should show redacted indicator"
        );
    }

    #[test]
    fn test_show_error_details_from_current() {
        let action = Action::ShowErrorDetailsFromCurrent;
        let output = redacted_debug(&action);

        assert!(
            output.contains("ShowErrorDetailsFromCurrent"),
            "Should contain action name"
        );
    }
}
