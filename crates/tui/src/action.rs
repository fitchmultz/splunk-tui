//! Action protocol for async TUI event handling.
//!
//! This module defines the unified Action enum that replaces simple events.
//! Actions represent both user inputs and async API operation results.

use crossterm::event::KeyEvent;
use serde_json::Value;
use splunk_client::models::{ClusterInfo, Index, SearchJobStatus};

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

    // API Triggers
    /// Load the list of indexes
    LoadIndexes,
    /// Load the list of search jobs
    LoadJobs,
    /// Load cluster information
    LoadClusterInfo,
    /// Run a search with the given query
    RunSearch(String),

    // API Results
    /// Result of loading indexes
    IndexesLoaded(Result<Vec<Index>, String>),
    /// Result of loading jobs
    JobsLoaded(Result<Vec<SearchJobStatus>, String>),
    /// Result of loading cluster info
    ClusterInfoLoaded(Result<ClusterInfo, String>),
    /// Result of a search completion (results, sid)
    SearchComplete(Result<(Vec<Value>, String), String>),

    // Job Operations
    /// Cancel a job by SID
    CancelJob(String),
    /// Delete a job by SID
    DeleteJob(String),
    /// Job operation completed successfully
    JobOperationComplete(String),

    // Progress
    /// Update progress indicator (0.0 - 1.0)
    Progress(f32),
    /// Set loading state
    Loading(bool),
    /// Display an error message
    Error(String),
}
