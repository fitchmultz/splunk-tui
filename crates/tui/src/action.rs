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
    /// Result of a search completion (results, sid, total_count)
    SearchComplete(Result<(Vec<Value>, String, Option<u64>), String>),
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
    /// Clear current error details (when popup is dismissed)
    ClearErrorDetails,
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
}
