//! App struct definitions.
//!
//! Responsibilities:
//! - Define the main App struct and its fields
//! - Define ConnectionContext for connection info
//! - Define SplValidationState for SPL validation
//!
//! Does NOT handle:
//! - Does NOT implement behavior methods (see core.rs, actions/, input/)
//! - Does NOT handle state mutations directly

use crate::action::ExportFormat;
use crate::app::export::ExportTarget;
use crate::app::input::components::SingleLineInput;
use crate::app::state::{
    ClusterViewMode, CurrentScreen, HealthState, ListPaginationState, SearchInputMode, ShcViewMode,
    SortState,
};
use crate::error_details::ErrorDetails;
use crate::focus::FocusManager;
use crate::onboarding::{OnboardingChecklistState, TutorialState};
use crate::ui::Toast;
use crate::ui::popup::Popup;
use ratatui::layout::Rect;
use serde_json::Value;
use splunk_client::SearchMode;
use splunk_client::models::{
    App as SplunkApp, Capability, ClusterInfo, ClusterPeer, DataModel, HealthCheckOutput, Index,
    KvStoreStatus, LogEntry, Macro, Role, SavedSearch, SearchJobStatus, SearchPeer, User,
};
use splunk_config::{ColorTheme, KeybindOverrides, ListDefaults, SearchDefaults, Theme};
use std::collections::HashSet;

/// Main application state.
pub struct App {
    pub current_screen: CurrentScreen,
    /// Single-line input component for search queries with enhanced editing.
    pub search_input: SingleLineInput,
    /// The query that was submitted for the currently running search.
    /// Used to display accurate status messages even if search_input is edited.
    pub running_query: Option<String>,
    pub search_status: String,
    pub search_results: Vec<Value>,
    pub search_scroll_offset: usize,
    pub search_sid: Option<String>,

    // Pagination state for search results
    pub search_results_total_count: Option<usize>,
    /// Page size for search result pagination.
    /// Initialized from `search_defaults.max_results` to stay in sync with the API request.
    /// This ensures the UI's pagination assumptions match the actual request page size.
    pub search_results_page_size: usize,
    pub search_has_more_results: bool,

    // Real data (Option for loading state)
    pub indexes: Option<Vec<Index>>,
    pub indexes_state: ratatui::widgets::ListState,
    pub jobs: Option<Vec<SearchJobStatus>>,
    pub jobs_state: ratatui::widgets::TableState,
    pub saved_searches: Option<Vec<SavedSearch>>,
    pub saved_searches_state: ratatui::widgets::ListState,
    pub macros: Option<Vec<Macro>>,
    pub macros_state: ratatui::widgets::ListState,
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

    // Audit events state
    pub audit_events: Option<Vec<splunk_client::models::AuditEvent>>,
    pub audit_state: ratatui::widgets::TableState,

    // Dashboards state
    pub dashboards: Option<Vec<splunk_client::models::Dashboard>>,
    pub dashboards_state: ratatui::widgets::ListState,
    pub dashboards_pagination: ListPaginationState,

    // Data models state
    pub data_models: Option<Vec<DataModel>>,
    pub data_models_state: ratatui::widgets::ListState,
    pub data_models_pagination: ListPaginationState,

    // Workload management state
    pub workload_pools: Option<Vec<splunk_client::models::WorkloadPool>>,
    pub workload_pools_state: ratatui::widgets::TableState,
    pub workload_pools_pagination: ListPaginationState,
    pub workload_rules: Option<Vec<splunk_client::models::WorkloadRule>>,
    pub workload_rules_state: ratatui::widgets::TableState,
    pub workload_rules_pagination: ListPaginationState,
    pub workload_view_mode: crate::app::state::WorkloadViewMode,

    // SHC state
    pub shc_status: Option<splunk_client::models::ShcStatus>,
    pub shc_members: Option<Vec<splunk_client::models::ShcMember>>,
    pub shc_captain: Option<splunk_client::models::ShcCaptain>,
    pub shc_config: Option<splunk_client::models::ShcConfig>,
    pub shc_members_state: ratatui::widgets::TableState,
    pub shc_view_mode: ShcViewMode,

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
    /// Single-line input for config search.
    pub config_search_query: SingleLineInput,
    pub config_search_before_edit: Option<String>,
    pub filtered_stanza_indices: Vec<usize>,

    // UI State
    pub loading: bool,
    pub loading_since: Option<std::time::Instant>,
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
    /// Single-line input for job filtering.
    pub filter_input: SingleLineInput,
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
    /// Saved search input for history navigation restoration.
    pub saved_search_input: SingleLineInput,

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
    pub roles_pagination: ListPaginationState,

    // Export state
    /// Single-line input for export filename.
    pub export_input: SingleLineInput,
    pub export_format: ExportFormat,
    pub(crate) export_target: Option<ExportTarget>,
    /// Recently used export paths (persisted)
    pub recent_export_paths: Vec<String>,

    // Error state
    pub current_error: Option<ErrorDetails>,
    pub error_scroll_offset: usize,

    // Loading spinner animation state (cycles 0-7 for spinner frames)
    pub spinner_frame: u8,

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
    /// Monotonically increasing request ID for SPL validation (for stale result detection).
    /// Incremented each time a validation is triggered; used to correlate results.
    pub validation_request_id: u64,

    // Search mode (RQ-0254)
    /// Current search mode (normal or realtime).
    pub search_mode: SearchMode,
    /// Real-time window in seconds (only used when search_mode is Realtime).
    pub realtime_window: Option<u64>,

    // Focus management (RQ-0323)
    /// Focus manager for keyboard navigation between components.
    pub focus_manager: FocusManager,
    /// Whether focus navigation mode is active (Ctrl+Tab to toggle).
    pub focus_navigation_mode: bool,
    /// Tutorial state for onboarding (only Some during tutorial)
    pub tutorial_state: Option<TutorialState>,
    /// Whether the tutorial has been completed (persisted)
    pub tutorial_completed: bool,
    /// Progressive onboarding checklist state (persisted)
    pub onboarding_checklist: OnboardingChecklistState,
    /// Command palette state for fuzzy search and recent commands
    pub command_palette_state: crate::app::command_palette::CommandPaletteState,

    // Undo/Redo system
    /// Buffer for managing undoable operations
    pub undo_buffer: crate::undo::UndoBuffer,
    /// Active undo toast ID for countdown updates
    pub undo_toast_id: Option<uuid::Uuid>,
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
    /// Request ID of the validation that produced this state (for stale result detection)
    pub request_id: u64,
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
