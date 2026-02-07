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
use splunk_client::SearchMode;
use splunk_client::models::{
    App as SplunkApp, AuditEvent, Capability, ClusterInfo, ClusterPeer, ConfigFile, ConfigStanza,
    Dashboard, DataModel, FiredAlert, Forwarder, HealthCheckOutput, Index, Input,
    KvStoreCollection, KvStoreRecord, KvStoreStatus, LicensePool, LicenseStack, LicenseUsage,
    LogEntry, LookupTable, Macro, Role, SavedSearch, SearchJobStatus, SearchPeer, ShcCaptain,
    ShcConfig, ShcMember, ShcStatus, SplunkHealth, User, WorkloadPool, WorkloadRule,
};
use splunk_config::{PersistedState, SearchDefaults};
use std::path::PathBuf;
use std::sync::Arc;

use crate::ConnectionContext;
use crate::action::format::ExportFormat;
use crate::ui::ToastLevel;

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

/// Per-resource summary for the overview screen.
///
/// Mirrors the CLI's ResourceSummary type for CLI/TUI parity.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OverviewResource {
    /// The resource type name (e.g., "indexes", "jobs", "apps")
    pub resource_type: String,
    /// Count of items for this resource type
    pub count: u64,
    /// Status string (e.g., "ok", "error", "timeout")
    pub status: String,
    /// Optional error message if the fetch failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Aggregated overview data for all Splunk resources.
///
/// This is the TUI equivalent of the CLI's list-all output,
/// providing a unified dashboard view of resource counts and status.
#[derive(Debug, Clone, serde::Serialize)]
pub struct OverviewData {
    /// List of resource summaries
    pub resources: Vec<OverviewResource>,
}

/// Per-instance overview data for multi-instance dashboard.
///
/// Represents the health and resource status of a single Splunk instance
/// within the multi-instance dashboard view.
#[derive(Debug, Clone, serde::Serialize)]
pub struct InstanceOverview {
    /// Profile name for this instance
    pub profile_name: String,
    /// Base URL of the Splunk instance
    pub base_url: String,
    /// Resource summaries for this instance
    pub resources: Vec<OverviewResource>,
    /// Error message if connection failed
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Health status (green/yellow/red)
    pub health_status: String,
    /// Job count (for quick reference)
    pub job_count: u64,
}

/// Aggregated multi-instance overview data.
///
/// Contains overview data for all configured Splunk instances,
/// enabling administrators to monitor multiple instances from a single view.
#[derive(Debug, Clone, serde::Serialize)]
pub struct MultiInstanceOverviewData {
    /// Timestamp of the data fetch
    pub timestamp: String,
    /// Overview data per instance
    pub instances: Vec<InstanceOverview>,
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

    // Focus Management
    /// Move focus to the next component within the current screen.
    NextFocus,
    /// Move focus to the previous component within the current screen.
    PreviousFocus,
    /// Set focus to a specific component by ID.
    SetFocus(String),
    /// Toggle focus navigation mode (Ctrl+Tab to navigate between components).
    ToggleFocusMode,

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
    /// Load KVStore status information
    LoadKvstore,
    /// Load the list of saved searches
    LoadSavedSearches,
    /// Load the list of macros
    LoadMacros,
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
    /// Load overview information (all resources)
    LoadOverview,
    /// Load multi-instance overview from all profiles
    LoadMultiInstanceOverview,
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
    /// Load the list of search peers with pagination
    LoadSearchPeers {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load more search peers (pagination)
    LoadMoreSearchPeers,
    /// Load the list of forwarders with pagination
    LoadForwarders {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load more forwarders (pagination)
    LoadMoreForwarders,
    /// Load the list of lookup tables with pagination
    LoadLookups {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load more lookup tables (pagination)
    LoadMoreLookups,
    /// Load the list of data inputs with pagination
    LoadInputs {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load more inputs (pagination)
    LoadMoreInputs,
    /// Load more roles (pagination)
    LoadMoreRoles,
    /// Load the list of config files
    LoadConfigFiles,
    /// Load the list of config stanzas for a specific config file
    LoadConfigStanzas {
        /// The config file name (e.g., "props", "transforms")
        config_file: String,
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load the list of fired alerts with pagination
    LoadFiredAlerts {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load more fired alerts (pagination)
    LoadMoreFiredAlerts,
    /// Load audit events with time range
    LoadAuditEvents {
        /// Number of events to load
        count: u64,
        /// Offset for pagination
        offset: u64,
        /// Earliest time for events
        earliest: String,
        /// Latest time for events
        latest: String,
    },
    /// Load recent audit events (last 24 hours)
    LoadRecentAuditEvents {
        /// Number of events to load
        count: u64,
    },
    /// Load the list of dashboards with pagination
    LoadDashboards {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load more dashboards (pagination)
    LoadMoreDashboards,
    /// Load the list of data models with pagination
    LoadDataModels {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load more data models (pagination)
    LoadMoreDataModels,

    // Refresh actions (reset pagination, load from offset=0)
    /// Refresh indexes - reset pagination and reload from offset 0
    RefreshIndexes,
    /// Refresh jobs - reset pagination and reload from offset 0
    RefreshJobs,
    /// Refresh apps - reset pagination and reload from offset 0
    RefreshApps,
    /// Refresh users - reset pagination and reload from offset 0
    RefreshUsers,
    /// Refresh roles - reset pagination and reload from offset 0
    RefreshRoles,
    /// Refresh internal logs - reload with default parameters
    RefreshInternalLogs,
    /// Refresh dashboards - reset pagination and reload from offset 0
    RefreshDashboards,
    /// Refresh data models - reset pagination and reload from offset 0
    RefreshDataModels,
    /// Refresh inputs - reset pagination and reload from offset 0
    RefreshInputs,
    /// Load the list of workload pools with pagination
    LoadWorkloadPools {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load more workload pools (pagination)
    LoadMoreWorkloadPools,
    /// Load the list of workload rules with pagination
    LoadWorkloadRules {
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Load more workload rules (pagination)
    LoadMoreWorkloadRules,
    /// Toggle workload view mode (Pools <-> Rules)
    ToggleWorkloadViewMode,
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
        search_mode: SearchMode,
        realtime_window: Option<u64>,
    },
    /// Toggle search mode between Normal and Realtime.
    ToggleSearchMode,
    /// Validate SPL syntax (debounced).
    ///
    /// Triggered when the user pauses typing in the search query input.
    /// The validation is performed asynchronously via the search parser endpoint.
    ValidateSpl { search: String, request_id: u64 },
    /// SPL validation completed.
    ///
    /// Contains the validation result with any errors or warnings found.
    SplValidationResult {
        valid: bool,
        errors: Vec<String>,
        warnings: Vec<String>,
        request_id: u64,
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
    /// Result of loading KVStore status
    KvstoreLoaded(Result<KvStoreStatus, Arc<ClientError>>),
    /// Result of loading saved searches
    SavedSearchesLoaded(Result<Vec<SavedSearch>, Arc<ClientError>>),
    /// Open edit dialog for selected saved search
    EditSavedSearch,
    /// Update saved search
    UpdateSavedSearch {
        name: String,
        search: Option<String>,
        description: Option<String>,
        disabled: Option<bool>,
    },
    /// Result of updating saved search
    SavedSearchUpdated(Result<(), Arc<ClientError>>),
    /// Open create saved search dialog
    OpenCreateSavedSearchDialog,
    /// Create a new saved search
    CreateSavedSearch {
        name: String,
        search: String,
        description: Option<String>,
        disabled: bool,
    },
    /// Open delete saved search confirmation
    OpenDeleteSavedSearchConfirm { name: String },
    /// Delete a saved search
    DeleteSavedSearch { name: String },
    /// Toggle saved search enabled/disabled state
    ToggleSavedSearch { name: String, disabled: bool },
    /// Result of creating a saved search
    SavedSearchCreated(Result<(), Arc<ClientError>>),
    /// Result of deleting a saved search
    SavedSearchDeleted(Result<String, Arc<ClientError>>),
    /// Result of toggling saved search state
    SavedSearchToggled(Result<(), Arc<ClientError>>),

    // Macro Operations
    /// Result of loading macros
    MacrosLoaded(Result<Vec<Macro>, Arc<ClientError>>),
    /// Open edit dialog for selected macro
    EditMacro,
    /// Open create macro dialog
    OpenCreateMacroDialog,
    /// Create a new macro
    CreateMacro {
        name: String,
        definition: String,
        args: Option<String>,
        description: Option<String>,
        disabled: bool,
        iseval: bool,
    },
    /// Update an existing macro
    UpdateMacro {
        name: String,
        definition: Option<String>,
        args: Option<String>,
        description: Option<String>,
        disabled: Option<bool>,
        iseval: Option<bool>,
    },
    /// Delete a macro
    DeleteMacro { name: String },
    /// Result of creating a macro
    MacroCreated(Result<(), Arc<ClientError>>),
    /// Result of updating a macro
    MacroUpdated(Result<(), Arc<ClientError>>),
    /// Result of deleting a macro
    MacroDeleted(Result<String, Arc<ClientError>>),

    /// Result of loading internal logs
    InternalLogsLoaded(Result<Vec<LogEntry>, Arc<ClientError>>),
    /// Result of loading apps
    AppsLoaded(Result<Vec<SplunkApp>, Arc<ClientError>>),
    /// Result of loading users
    UsersLoaded(Result<Vec<User>, Arc<ClientError>>),
    /// Result of loading cluster peers
    ClusterPeersLoaded(Result<Vec<ClusterPeer>, Arc<ClientError>>),

    // Cluster management actions
    /// Set maintenance mode on the cluster
    SetMaintenanceMode { enable: bool },
    /// Result of setting maintenance mode
    MaintenanceModeSet { result: Result<(), String> },
    /// Rebalance cluster primaries
    RebalanceCluster,
    /// Result of rebalancing cluster
    ClusterRebalanced { result: Result<(), String> },
    /// Decommission a peer by GUID
    DecommissionPeer { peer_guid: String },
    /// Result of decommissioning a peer
    PeerDecommissioned { result: Result<(), String> },
    /// Remove a peer from the cluster
    RemovePeer { peer_guid: String },
    /// Result of removing a peer
    PeerRemoved { result: Result<(), String> },

    /// Result of loading overview
    OverviewLoaded(OverviewData),
    /// Multi-instance overview data loaded
    MultiInstanceOverviewLoaded(MultiInstanceOverviewData),
    /// Result of loading more indexes (pagination)
    MoreIndexesLoaded(Result<Vec<Index>, Arc<ClientError>>),
    /// Result of loading more jobs (pagination)
    MoreJobsLoaded(Result<Vec<SearchJobStatus>, Arc<ClientError>>),
    /// Result of loading more apps (pagination)
    MoreAppsLoaded(Result<Vec<SplunkApp>, Arc<ClientError>>),
    /// Result of loading more users (pagination)
    MoreUsersLoaded(Result<Vec<User>, Arc<ClientError>>),
    /// Result of loading search peers
    SearchPeersLoaded(Result<Vec<SearchPeer>, Arc<ClientError>>),
    /// Result of loading more search peers (pagination)
    MoreSearchPeersLoaded(Result<Vec<SearchPeer>, Arc<ClientError>>),
    /// Result of loading forwarders
    ForwardersLoaded(Result<Vec<Forwarder>, Arc<ClientError>>),
    /// Result of loading more forwarders (pagination)
    MoreForwardersLoaded(Result<Vec<Forwarder>, Arc<ClientError>>),
    /// Result of loading lookup tables
    LookupsLoaded(Result<Vec<LookupTable>, Arc<ClientError>>),
    /// Result of loading more lookup tables (pagination)
    MoreLookupsLoaded(Result<Vec<LookupTable>, Arc<ClientError>>),

    // Lookup Operations
    /// Download a lookup table file
    DownloadLookup {
        name: String,
        app: Option<String>,
        owner: Option<String>,
        output_path: PathBuf,
    },
    /// Delete a lookup table file
    DeleteLookup {
        name: String,
        app: Option<String>,
        owner: Option<String>,
    },
    /// Open delete lookup confirmation dialog
    OpenDeleteLookupConfirm { name: String },
    /// Result of downloading a lookup
    LookupDownloaded(Result<String, Arc<ClientError>>),
    /// Result of deleting a lookup
    LookupDeleted(Result<String, Arc<ClientError>>),
    /// Result of loading inputs
    InputsLoaded(Result<Vec<Input>, Arc<ClientError>>),
    /// Result of loading more inputs (pagination)
    MoreInputsLoaded(Result<Vec<Input>, Arc<ClientError>>),
    /// Result of loading more roles (pagination)
    MoreRolesLoaded(Result<Vec<Role>, Arc<ClientError>>),
    /// Result of loading config files
    ConfigFilesLoaded(Result<Vec<ConfigFile>, Arc<ClientError>>),
    /// Result of loading config stanzas
    ConfigStanzasLoaded(Result<Vec<ConfigStanza>, Arc<ClientError>>),
    /// Result of loading fired alerts
    FiredAlertsLoaded(Result<Vec<FiredAlert>, Arc<ClientError>>),
    /// Result of loading more fired alerts (pagination)
    MoreFiredAlertsLoaded(Result<Vec<FiredAlert>, Arc<ClientError>>),
    /// Result of loading audit events
    AuditEventsLoaded(Result<Vec<AuditEvent>, Arc<ClientError>>),
    /// Result of loading dashboards
    DashboardsLoaded(Result<Vec<Dashboard>, Arc<ClientError>>),
    /// Result of loading more dashboards (pagination)
    MoreDashboardsLoaded(Result<Vec<Dashboard>, Arc<ClientError>>),
    /// Result of loading data models
    DataModelsLoaded(Result<Vec<DataModel>, Arc<ClientError>>),
    /// Result of loading more data models (pagination)
    MoreDataModelsLoaded(Result<Vec<DataModel>, Arc<ClientError>>),
    /// Result of loading workload pools
    WorkloadPoolsLoaded(Result<Vec<WorkloadPool>, Arc<ClientError>>),
    /// Result of loading more workload pools (pagination)
    MoreWorkloadPoolsLoaded(Result<Vec<WorkloadPool>, Arc<ClientError>>),
    /// Result of loading workload rules
    WorkloadRulesLoaded(Result<Vec<WorkloadRule>, Arc<ClientError>>),
    /// Result of loading more workload rules (pagination)
    MoreWorkloadRulesLoaded(Result<Vec<WorkloadRule>, Arc<ClientError>>),

    // SHC Actions
    /// Load SHC status
    LoadShcStatus,
    /// Load SHC members
    LoadShcMembers,
    /// Load SHC captain
    LoadShcCaptain,
    /// Load SHC config
    LoadShcConfig,
    /// Toggle SHC view mode (Summary <-> Members)
    ToggleShcViewMode,
    /// Result of loading SHC status
    ShcStatusLoaded(Result<ShcStatus, Arc<ClientError>>),
    /// Result of loading SHC members
    ShcMembersLoaded(Result<Vec<ShcMember>, Arc<ClientError>>),
    /// Result of loading SHC captain
    ShcCaptainLoaded(Result<ShcCaptain, Arc<ClientError>>),
    /// Result of loading SHC config
    ShcConfigLoaded(Result<ShcConfig, Arc<ClientError>>),

    // SHC Management Actions
    /// Add a member to the SHC
    AddShcMember { target_uri: String },
    /// Result of adding SHC member
    ShcMemberAdded { result: Result<(), String> },
    /// Remove a member from the SHC
    RemoveShcMember { member_guid: String },
    /// Result of removing SHC member
    ShcMemberRemoved { result: Result<(), String> },
    /// Trigger SHC rolling restart
    RollingRestartShc { force: bool },
    /// Result of rolling restart
    ShcRollingRestarted { result: Result<(), String> },
    /// Set a specific member as captain
    SetShcCaptain { member_guid: String },
    /// Result of setting captain
    ShcCaptainSet { result: Result<(), String> },

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
    /// Install an app from a .spl file
    InstallApp { file_path: PathBuf },
    /// Remove (uninstall) an app by name
    RemoveApp { app_name: String },

    // Input Operations
    /// Enable an input by type and name
    EnableInput { input_type: String, name: String },
    /// Disable an input by type and name
    DisableInput { input_type: String, name: String },

    // Index Operations
    /// Create a new index
    CreateIndex {
        params: splunk_client::CreateIndexParams,
    },
    /// Modify an existing index
    ModifyIndex {
        name: String,
        params: splunk_client::ModifyIndexParams,
    },
    /// Delete an index
    DeleteIndex { name: String },
    /// Open index creation dialog
    OpenCreateIndexDialog,
    /// Open index modification dialog
    OpenModifyIndexDialog { name: String },
    /// Open index deletion confirmation
    OpenDeleteIndexConfirm { name: String },
    /// Result of creating an index
    IndexCreated(Result<Index, Arc<ClientError>>),
    /// Result of modifying an index
    IndexModified(Result<Index, Arc<ClientError>>),
    /// Result of deleting an index
    IndexDeleted(Result<String, Arc<ClientError>>),

    // User Operations
    /// Create a new user
    CreateUser {
        params: splunk_client::CreateUserParams,
    },
    /// Modify an existing user
    ModifyUser {
        name: String,
        params: splunk_client::ModifyUserParams,
    },
    /// Delete a user
    DeleteUser { name: String },
    /// Open user creation dialog
    OpenCreateUserDialog,
    /// Open user modification dialog
    OpenModifyUserDialog { name: String },
    /// Open user deletion confirmation
    OpenDeleteUserConfirm { name: String },
    /// Result of creating a user
    UserCreated(Result<User, Arc<ClientError>>),
    /// Result of modifying a user
    UserModified(Result<User, Arc<ClientError>>),
    /// Result of deleting a user
    UserDeleted(Result<String, Arc<ClientError>>),

    // Role Operations
    /// Load roles list
    LoadRoles { count: u64, offset: u64 },
    /// Create a new role
    CreateRole {
        params: splunk_client::CreateRoleParams,
    },
    /// Modify an existing role
    ModifyRole {
        name: String,
        params: splunk_client::ModifyRoleParams,
    },
    /// Delete a role
    DeleteRole { name: String },
    /// Load capabilities list
    LoadCapabilities,
    /// Open role creation dialog
    OpenCreateRoleDialog,
    /// Open role modification dialog
    OpenModifyRoleDialog { name: String },
    /// Open role deletion confirmation
    OpenDeleteRoleConfirm { name: String },
    /// Result of loading roles
    RolesLoaded(Result<Vec<Role>, Arc<ClientError>>),
    /// Result of creating a role
    RoleCreated(Result<Role, Arc<ClientError>>),
    /// Result of modifying a role
    RoleModified(Result<Role, Arc<ClientError>>),
    /// Result of deleting a role
    RoleDeleted(Result<String, Arc<ClientError>>),
    /// Result of loading capabilities
    CapabilitiesLoaded(Result<Vec<Capability>, Arc<ClientError>>),

    // License Operations
    /// Install a license file
    InstallLicense { file_path: PathBuf },
    /// Create a new license pool
    CreateLicensePool {
        params: splunk_client::CreatePoolParams,
    },
    /// Modify an existing license pool
    ModifyLicensePool {
        name: String,
        params: splunk_client::ModifyPoolParams,
    },
    /// Delete a license pool
    DeleteLicensePool { name: String },
    /// Activate a license
    ActivateLicense { name: String },
    /// Deactivate a license
    DeactivateLicense { name: String },
    /// Open license installation dialog
    OpenInstallLicenseDialog,
    /// Open license pool creation dialog
    OpenCreateLicensePoolDialog,
    /// Open license pool modification dialog
    OpenModifyLicensePoolDialog { name: String },
    /// Open license pool deletion confirmation
    OpenDeleteLicensePoolConfirm { name: String },
    /// Result of installing a license
    LicenseInstalled(Result<splunk_client::LicenseInstallResult, Arc<ClientError>>),
    /// Result of creating a license pool
    LicensePoolCreated(Result<splunk_client::LicensePool, Arc<ClientError>>),
    /// Result of modifying a license pool
    LicensePoolModified(Result<splunk_client::LicensePool, Arc<ClientError>>),
    /// Result of deleting a license pool
    LicensePoolDeleted(Result<String, Arc<ClientError>>),
    /// Result of activating a license
    LicenseActivated(Result<splunk_client::LicenseActivationResult, Arc<ClientError>>),
    /// Result of deactivating a license
    LicenseDeactivated(Result<splunk_client::LicenseActivationResult, Arc<ClientError>>),

    // KVStore Collection Operations
    /// Load KVStore collections list
    LoadCollections {
        /// App context (None for all apps)
        app: Option<String>,
        /// Owner context (None for nobody)
        owner: Option<String>,
        /// Number of items to load
        count: u64,
        /// Offset for pagination
        offset: u64,
    },
    /// Create a new KVStore collection
    CreateCollection {
        params: splunk_client::models::CreateCollectionParams,
    },
    /// Delete a KVStore collection
    DeleteCollection {
        name: String,
        app: String,
        owner: String,
    },
    /// Load collection records
    LoadCollectionRecords {
        collection_name: String,
        app: String,
        owner: String,
        query: Option<String>,
        count: u64,
        offset: u64,
    },
    /// Open collection creation dialog
    OpenCreateCollectionDialog,
    /// Open collection deletion confirmation
    OpenDeleteCollectionConfirm {
        name: String,
        app: String,
        owner: String,
    },
    /// Result of loading collections
    CollectionsLoaded(Result<Vec<KvStoreCollection>, Arc<ClientError>>),
    /// Result of creating a collection
    CollectionCreated(Result<KvStoreCollection, Arc<ClientError>>),
    /// Result of deleting a collection
    CollectionDeleted(Result<(String, String, String), Arc<ClientError>>),
    /// Result of loading collection records
    CollectionRecordsLoaded(Result<Vec<KvStoreRecord>, Arc<ClientError>>),

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

    // Profile Management
    /// Open profile creation dialog
    OpenCreateProfileDialog,
    /// Open profile editing dialog (triggers async load)
    OpenEditProfileDialog { name: String },
    /// Open profile editing dialog with pre-populated data
    #[allow(clippy::type_complexity)]
    OpenEditProfileDialogWithData {
        original_name: String,
        name_input: String,
        base_url_input: String,
        username_input: String,
        skip_verify: bool,
        timeout_seconds: u64,
        max_retries: usize,
    },
    /// Open profile deletion confirmation
    OpenDeleteProfileConfirm { name: String },
    /// Save/create a profile
    SaveProfile {
        name: String,
        profile: splunk_config::types::ProfileConfig,
        use_keyring: bool,
        /// Original name when renaming a profile (None for new profiles)
        original_name: Option<String>,
    },
    /// Delete a profile
    DeleteProfile { name: String },
    /// Result of profile save operation
    ProfileSaved(Result<String, String>),
    /// Result of profile delete operation
    ProfileDeleted(Result<String, String>),
}
