//! Action enum definitions for the TUI event system.
//!
//! This module defines the unified `Action` enum that replaces simple events.
//! Actions represent both user inputs and async API operation results.
//!
//! # Action Categories
//!
//! - **System**: Application lifecycle (Quit, screen switching)
//! - **Input**: User interactions (keyboard, mouse, navigation)
//! - **API Triggers**: Commands to load data or execute operations
//! - **API Results**: Async responses from Splunk API calls
//! - **Job Operations**: Search job management (cancel, delete)
//! - **App Operations**: App management (enable, disable)
//! - **Progress**: Loading indicators and progress updates
//! - **Notifications**: Toast messages and periodic ticks
//! - **Error Handling**: Error display and clearing
//! - **Profile Switching**: Connection profile management
//!
//! # Security Note
//!
//! When logging Actions, use `RedactedAction(&action)` wrapper instead of
//! `?action` Debug formatting to prevent sensitive payloads from being written
//! to log files. See `RedactedAction` documentation for details.
//!
//! # What This Module Does NOT Handle
//!
//! - Action handling logic (handled by the app state machine)
//! - Async task execution (handled by the runtime module)
//! - UI rendering (handled by the ui module)

use crossterm::event::KeyEvent;
use serde_json::Value;
use splunk_client::ClientError;
use splunk_client::models::{
    App as SplunkApp, ClusterInfo, ClusterPeer, HealthCheckOutput, Index, LicensePool,
    LicenseStack, LicenseUsage, LogEntry, SavedSearch, SearchJobStatus, SplunkHealth, User,
};
use splunk_config::{PersistedState, SearchDefaults};
use std::path::PathBuf;
use std::sync::Arc;

use crate::ConnectionContext;
use crate::action::format::ExportFormat;
use crate::ui::ToastLevel;

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
    /// Terminal resize event with new dimensions (width, height)
    Resize(u16, u16),
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
    /// Load the list of indexes with pagination
    LoadIndexes {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load the list of search jobs with pagination
    LoadJobs {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load cluster information
    LoadClusterInfo,
    /// Load health check information
    LoadHealth,
    /// Load license information (usage, pools, stacks)
    LoadLicense,
    /// Load the list of saved searches
    LoadSavedSearches,
    /// Load internal logs from index=_internal
    LoadInternalLogs {
        /// Number of log entries to fetch
        count: u64,
        /// Earliest time for the query (e.g., "-15m")
        earliest: String,
    },
    /// Load the list of apps with pagination
    LoadApps {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load the list of users with pagination
    LoadUsers {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load cluster peers (detailed view)
    LoadClusterPeers,
    /// Load more indexes (pagination)
    LoadMoreIndexes,
    /// Load more jobs (pagination)
    LoadMoreJobs,
    /// Load more apps (pagination)
    LoadMoreApps,
    /// Load more users (pagination)
    LoadMoreUsers,
    /// Load more internal logs (refresh)
    LoadMoreInternalLogs,
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
    IndexesLoaded(Result<Vec<Index>, Arc<ClientError>>),
    /// Result of loading jobs
    JobsLoaded(Result<Vec<SearchJobStatus>, Arc<ClientError>>),
    /// Result of loading cluster info
    ClusterInfoLoaded(Result<ClusterInfo, Arc<ClientError>>),
    /// Result of loading health check
    HealthLoaded(Box<Result<HealthCheckOutput, Arc<ClientError>>>),
    /// Result of loading license information
    LicenseLoaded(Box<Result<LicenseData, Arc<ClientError>>>),
    /// Result of loading saved searches
    SavedSearchesLoaded(Result<Vec<SavedSearch>, Arc<ClientError>>),
    /// Result of loading internal logs
    InternalLogsLoaded(Result<Vec<LogEntry>, Arc<ClientError>>),
    /// Result of loading apps
    AppsLoaded(Result<Vec<SplunkApp>, Arc<ClientError>>),
    /// Result of loading users
    UsersLoaded(Result<Vec<User>, Arc<ClientError>>),
    /// Result of loading cluster peers
    ClusterPeersLoaded(Result<Vec<ClusterPeer>, Arc<ClientError>>),
    /// Result of loading more indexes (pagination)
    MoreIndexesLoaded(Result<Vec<Index>, Arc<ClientError>>),
    /// Result of loading more jobs (pagination)
    MoreJobsLoaded(Result<Vec<SearchJobStatus>, Arc<ClientError>>),
    /// Result of loading more apps (pagination)
    MoreAppsLoaded(Result<Vec<SplunkApp>, Arc<ClientError>>),
    /// Result of loading more users (pagination)
    MoreUsersLoaded(Result<Vec<User>, Arc<ClientError>>),
    /// Result of loading persisted settings
    SettingsLoaded(PersistedState),
    /// Result of background health status check
    HealthStatusLoaded(Result<SplunkHealth, Arc<ClientError>>),
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
    MoreSearchResultsLoaded(Result<(Vec<Value>, u64, Option<u64>), Arc<ClientError>>),

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

/// Aggregated license data from multiple API endpoints.
///
/// This struct combines license usage, pools, and stacks into a single
/// data structure for the TUI license screen.
#[derive(Debug, Clone, serde::Serialize)]
pub struct LicenseData {
    /// License usage information (quota and used bytes)
    pub usage: Vec<LicenseUsage>,
    /// License pools
    pub pools: Vec<LicensePool>,
    /// License stacks
    pub stacks: Vec<LicenseStack>,
}
