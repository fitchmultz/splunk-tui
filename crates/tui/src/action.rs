//! Action protocol for async TUI event handling.
//!
//! This module defines the unified Action enum that replaces simple events.
//! Actions represent both user inputs and async API operation results.

use crossterm::event::KeyEvent;
use serde_json::Value;
use splunk_client::models::{
    ClusterInfo, HealthCheckOutput, Index, LogEntry, SavedSearch, SearchJobStatus, SplunkHealth,
};
use std::path::PathBuf;

use crate::ui::ToastLevel;

/// Supported export formats for search results.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExportFormat {
    /// JSON format
    Json,
    /// CSV format
    Csv,
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
    #[allow(dead_code)]
    SearchInput(char),
    /// Clear the search filter
    ClearSearch,
    /// Cycle sort column for jobs
    CycleSortColumn,
    /// Toggle sort direction for jobs
    #[allow(dead_code)]
    ToggleSortDirection,

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
    /// Run a search with the given query
    RunSearch(String),
    /// Export search results to a file
    ExportSearchResults(Vec<Value>, PathBuf, ExportFormat),

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
    /// Result of background health status check
    HealthStatusLoaded(Result<SplunkHealth, String>),
    /// Result of a search completion (results, sid)
    SearchComplete(Result<(Vec<Value>, String), String>),

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
}
