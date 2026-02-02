//! App struct definitions.
//!
//! Responsibilities:
//! - Define the main App struct and its fields
//! - Define ConnectionContext for connection info
//! - Define SplValidationState for SPL validation
//!
//! Non-responsibilities:
//! - Does NOT implement behavior methods (see core.rs, actions/, input/)
//! - Does NOT handle state mutations directly

use crate::action::ExportFormat;
use crate::app::export::ExportTarget;
use crate::app::state::{
    ClusterViewMode, CurrentScreen, HealthState, ListPaginationState, SearchInputMode, SortState,
};
use crate::error_details::ErrorDetails;
use crate::ui::Toast;
use crate::ui::popup::Popup;
use ratatui::layout::Rect;
use serde_json::Value;
use splunk_client::models::{
    App as SplunkApp, Capability, ClusterInfo, ClusterPeer, HealthCheckOutput, Index,
    KvStoreStatus, LogEntry, Role, SavedSearch, SearchJobStatus, SearchPeer, User,
};
use splunk_config::{ColorTheme, KeybindOverrides, ListDefaults, SearchDefaults, Theme};
use std::collections::HashSet;

/// Main application state.
pub struct App {
    pub current_screen: CurrentScreen,
    pub search_input: String,
    /// Cursor position within search_input (byte index, not character index).
    /// Must be kept in sync with search_input modifications.
    pub search_cursor_position: usize,
    /// The query that was submitted for the currently running search.
    /// Used to display accurate status messages even if search_input is edited.
    pub running_query: Option<String>,
    pub search_status: String,
    pub search_results: Vec<Value>,
    pub search_scroll_offset: usize,
    pub search_sid: Option<String>,

    // Pagination state for search results
    pub search_results_total_count: Option<u64>,
    pub search_results_page_size: u64,
    pub search_has_more_results: bool,

    // Real data (Option for loading state)
    pub indexes: Option<Vec<Index>>,
    pub indexes_state: ratatui::widgets::ListState,
    pub jobs: Option<Vec<SearchJobStatus>>,
    pub jobs_state: ratatui::widgets::TableState,
    pub saved_searches: Option<Vec<SavedSearch>>,
    pub saved_searches_state: ratatui::widgets::ListState,
    pub internal_logs: Option<Vec<LogEntry>>,
    pub internal_logs_state: ratatui::widgets::TableState,
    pub cluster_info: Option<ClusterInfo>,
    pub cluster_peers: Option<Vec<ClusterPeer>>,
    pub cluster_peers_state: ratatui::widgets::TableState,
    pub cluster_view_mode: ClusterViewMode,
    pub health_info: Option<HealthCheckOutput>,
    pub license_info: Option<crate::action::LicenseData>,
    pub kvstore_status: Option<KvStoreStatus>,
    pub apps: Option<Vec<SplunkApp>>,
    pub apps_state: ratatui::widgets::ListState,
    pub users: Option<Vec<User>>,
    pub users_state: ratatui::widgets::ListState,
    pub roles: Option<Vec<Role>>,
    pub roles_state: ratatui::widgets::ListState,
    pub capabilities: Option<Vec<Capability>>,
    pub search_peers: Option<Vec<SearchPeer>>,
    pub search_peers_state: ratatui::widgets::TableState,
    pub search_peers_pagination: ListPaginationState,
    pub inputs: Option<Vec<splunk_client::models::Input>>,
    pub inputs_state: ratatui::widgets::TableState,
    pub inputs_pagination: ListPaginationState,
    pub overview_data: Option<crate::action::OverviewData>,

    // Multi-instance dashboard state
    pub multi_instance_data: Option<crate::action::MultiInstanceOverviewData>,
    pub multi_instance_selected_index: usize,

    // Fired alerts state
    pub fired_alerts: Option<Vec<splunk_client::models::FiredAlert>>,
    pub fired_alerts_state: ratatui::widgets::ListState,
    pub fired_alerts_pagination: ListPaginationState,

    // Forwarders state
    pub forwarders: Option<Vec<splunk_client::models::Forwarder>>,
    pub forwarders_state: ratatui::widgets::TableState,
    pub forwarders_pagination: ListPaginationState,

    // Lookups state
    pub lookups: Option<Vec<splunk_client::models::LookupTable>>,
    pub lookups_state: ratatui::widgets::TableState,
    pub lookups_pagination: ListPaginationState,

    // Configs state
    pub config_files: Option<Vec<splunk_client::models::ConfigFile>>,
    pub config_files_state: ratatui::widgets::TableState,
    pub selected_config_file: Option<String>,
    pub config_stanzas: Option<Vec<splunk_client::models::ConfigStanza>>,
    pub config_stanzas_state: ratatui::widgets::TableState,
    pub selected_stanza: Option<splunk_client::models::ConfigStanza>,
    pub config_view_mode: crate::ui::screens::configs::ConfigViewMode,

    // Configs search state
    pub config_search_mode: bool,
    pub config_search_query: String,
    pub config_search_before_edit: Option<String>,
    pub filtered_stanza_indices: Vec<usize>,

    // UI State
    pub loading: bool,
    pub progress: f32,
    pub toasts: Vec<Toast>,
    pub auto_refresh: bool,
    pub popup: Option<Popup>,

    /// Currently selected color theme (persisted across runs).
    pub color_theme: ColorTheme,
    /// Expanded runtime theme derived from `color_theme`.
    pub theme: Theme,

    // Jobs filter state
    pub search_filter: Option<String>,
    pub is_filtering: bool,
    pub filter_input: String,
    /// Stores the filter value before entering edit mode, used for cancel semantics.
    /// When Some, pressing Esc reverts to this value instead of clearing.
    pub filter_before_edit: Option<String>,
    /// Maps filtered view index -> original jobs list index
    pub filtered_job_indices: Vec<usize>,

    // Jobs sort state
    pub sort_state: SortState,

    // Multi-selection state for batch job operations
    pub selected_jobs: HashSet<String>,

    // Health monitoring state
    pub health_state: HealthState,

    // Search history
    pub search_history: Vec<String>,
    pub history_index: Option<usize>,
    pub saved_search_input: String,

    // Search defaults (persisted)
    pub search_defaults: SearchDefaults,

    // Keybinding overrides (persisted)
    pub keybind_overrides: KeybindOverrides,

    // List defaults (persisted)
    pub list_defaults: ListDefaults,

    // Internal logs defaults (persisted)
    pub internal_logs_defaults: splunk_config::InternalLogsDefaults,

    // Pagination state for list screens
    pub indexes_pagination: ListPaginationState,
    pub jobs_pagination: ListPaginationState,
    pub apps_pagination: ListPaginationState,
    pub users_pagination: ListPaginationState,

    // Export state
    pub export_input: String,
    pub export_format: ExportFormat,
    pub(crate) export_target: Option<ExportTarget>,

    // Error state
    pub current_error: Option<ErrorDetails>,
    pub error_scroll_offset: usize,

    // Index details popup scroll offset
    pub index_details_scroll_offset: usize,

    // Help popup scroll offset
    pub help_scroll_offset: usize,

    // Layout tracking
    pub last_area: Rect,

    // Connection context (RQ-0134)
    /// Profile name used for this connection (from CLI --profile or SPLUNK_PROFILE env var)
    pub profile_name: Option<String>,
    /// Base URL of the Splunk server
    pub base_url: Option<String>,
    /// Auth mode display string (e.g., "token" or "session")
    pub auth_mode: Option<String>,
    /// Server version (fetched from server info)
    pub server_version: Option<String>,
    /// Server build (fetched from server info)
    pub server_build: Option<String>,

    // Search input mode (RQ-0101)
    /// Current input mode for the search screen.
    /// When QueryFocused, printable characters insert into the query.
    /// When ResultsFocused, navigation keys work on results.
    pub search_input_mode: SearchInputMode,

    // SPL validation state (RQ-0240)
    /// Current SPL validation state for real-time feedback.
    pub spl_validation_state: SplValidationState,
    /// Whether validation is pending (debounced).
    pub spl_validation_pending: bool,
    /// Timestamp of last input change for debouncing.
    pub last_input_change: Option<std::time::Instant>,
}

/// SPL validation state for real-time feedback in the search screen.
#[derive(Debug, Clone, Default)]
pub struct SplValidationState {
    /// Whether the SPL is valid (None = not validated yet)
    pub valid: Option<bool>,
    /// List of validation error messages
    pub errors: Vec<String>,
    /// List of validation warning messages
    pub warnings: Vec<String>,
    /// Timestamp of last validation
    pub last_validated: Option<std::time::Instant>,
}

/// Connection context for the TUI header display.
///
/// Contains static connection information passed at startup.
/// Server version/build are fetched separately and populated later.
#[derive(Debug, Clone, Default)]
pub struct ConnectionContext {
    /// Profile name (from --profile or SPLUNK_PROFILE env var)
    pub profile_name: Option<String>,
    /// Base URL of the Splunk server
    pub base_url: String,
    /// Auth mode display string ("token" or "session")
    pub auth_mode: String,
}
