//! Application state and rendering.
//!
//! This module contains the main application state, input handling,
//! and rendering logic for the TUI.
//!
//! The module is organized into submodules:
//! - `state`: Core state types (HealthState, CurrentScreen, Sort types)
//! - `clipboard`: Clipboard integration
//! - `export`: Export functionality
//! - `navigation`: Navigation helpers (next/previous item, page up/down, etc.)
//! - `jobs`: Jobs-specific logic (filtering, sorting)
//! - `mouse`: Mouse event handling
//! - `popups`: Popup input handling
//! - `input`: Per-screen input handlers
//! - `actions`: Action handling
//! - `render`: Rendering logic

pub mod clipboard;
pub mod state;

mod actions;
mod export;
pub mod footer_layout;
pub mod input;
mod jobs;
mod mouse;
mod navigation;
mod popups;
mod render;

pub use state::{
    ClusterViewMode, CurrentScreen, FOOTER_HEIGHT, HEADER_HEIGHT, HealthState, ListPaginationState,
    SearchInputMode, SortColumn, SortDirection, SortState,
};

use crate::action::{Action, ExportFormat, LicenseData};
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crate::ui::popup::Popup;
use crossterm::event::{KeyCode, KeyEvent};
use ratatui::layout::Rect;
use serde_json::Value;
use splunk_client::models::{
    App as SplunkApp, ClusterInfo, ClusterPeer, HealthCheckOutput, Index, KvStoreStatus, LogEntry,
    SavedSearch, SearchJobStatus, SearchPeer, User,
};
use splunk_config::constants::{
    DEFAULT_CLIPBOARD_PREVIEW_CHARS, DEFAULT_HISTORY_MAX_ITEMS, DEFAULT_SCROLL_THRESHOLD,
    DEFAULT_SEARCH_PAGE_SIZE,
};
use splunk_config::{
    ColorTheme, KeybindOverrides, ListDefaults, ListType, PersistedState, SearchDefaults, Theme,
};
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
    pub cluster_view_mode: crate::app::state::ClusterViewMode,
    pub health_info: Option<HealthCheckOutput>,
    pub license_info: Option<LicenseData>,
    pub kvstore_status: Option<KvStoreStatus>,
    pub apps: Option<Vec<SplunkApp>>,
    pub apps_state: ratatui::widgets::ListState,
    pub users: Option<Vec<User>>,
    pub users_state: ratatui::widgets::ListState,
    pub search_peers: Option<Vec<SearchPeer>>,
    pub search_peers_state: ratatui::widgets::TableState,
    pub search_peers_pagination: crate::app::state::ListPaginationState,
    pub inputs: Option<Vec<splunk_client::models::Input>>,
    pub inputs_state: ratatui::widgets::TableState,
    pub inputs_pagination: crate::app::state::ListPaginationState,
    pub overview_data: Option<crate::action::OverviewData>,

    // Multi-instance dashboard state
    pub multi_instance_data: Option<crate::action::MultiInstanceOverviewData>,
    pub multi_instance_selected_index: usize,

    // Fired alerts state
    pub fired_alerts: Option<Vec<splunk_client::models::FiredAlert>>,
    pub fired_alerts_state: ratatui::widgets::ListState,
    pub fired_alerts_pagination: crate::app::state::ListPaginationState,

    // Forwarders state
    pub forwarders: Option<Vec<splunk_client::models::Forwarder>>,
    pub forwarders_state: ratatui::widgets::TableState,
    pub forwarders_pagination: crate::app::state::ListPaginationState,

    // Lookups state
    pub lookups: Option<Vec<splunk_client::models::LookupTable>>,
    pub lookups_state: ratatui::widgets::TableState,
    pub lookups_pagination: crate::app::state::ListPaginationState,

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
    /// Maps filtered view index → original jobs list index
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
    pub current_error: Option<crate::error_details::ErrorDetails>,
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
    pub search_input_mode: crate::app::state::SearchInputMode,

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

impl Default for App {
    fn default() -> Self {
        Self::new(None, ConnectionContext::default())
    }
}

/// Check if a key event represents a printable character that should be inserted
/// into text input during QueryFocused mode.
///
/// A key is considered printable only if:
/// - It's a character key (KeyCode::Char)
/// - The character is not a control character
/// - No modifier keys (Ctrl, Alt, etc.) are pressed
fn is_printable_char(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char(c) if !c.is_control() && key.modifiers.is_empty())
}

/// Check if a key event is used for mode switching in the search screen.
/// These keys should bypass global bindings when in QueryFocused mode.
fn is_mode_switch_key(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Tab | KeyCode::BackTab)
}

/// Check if a key event is used for cursor movement/editing in the search query.
/// These keys should bypass global bindings when in QueryFocused mode (RQ-0110).
fn is_cursor_editing_key(key: KeyEvent) -> bool {
    matches!(
        key.code,
        KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Delete
            | KeyCode::Backspace
    )
}

impl App {
    /// Create a new App instance.
    ///
    /// # Arguments
    ///
    /// * `persisted` - Optional persisted state from previous runs
    /// * `connection_ctx` - Connection context (profile, base_url, auth_mode)
    pub fn new(persisted: Option<PersistedState>, connection_ctx: ConnectionContext) -> Self {
        let mut indexes_state = ratatui::widgets::ListState::default();
        indexes_state.select(Some(0));

        let mut jobs_state = ratatui::widgets::TableState::default();
        jobs_state.select(Some(0));

        let mut saved_searches_state = ratatui::widgets::ListState::default();
        saved_searches_state.select(Some(0));

        let mut internal_logs_state = ratatui::widgets::TableState::default();
        internal_logs_state.select(Some(0));

        let mut apps_state = ratatui::widgets::ListState::default();
        apps_state.select(Some(0));

        let mut users_state = ratatui::widgets::ListState::default();
        users_state.select(Some(0));

        let mut cluster_peers_state = ratatui::widgets::TableState::default();
        cluster_peers_state.select(Some(0));

        let mut search_peers_state = ratatui::widgets::TableState::default();
        search_peers_state.select(Some(0));

        let mut inputs_state = ratatui::widgets::TableState::default();
        inputs_state.select(Some(0));

        let mut config_files_state = ratatui::widgets::TableState::default();
        config_files_state.select(Some(0));

        let mut config_stanzas_state = ratatui::widgets::TableState::default();
        config_stanzas_state.select(Some(0));

        let mut fired_alerts_state = ratatui::widgets::ListState::default();
        fired_alerts_state.select(Some(0));

        let mut forwarders_state = ratatui::widgets::TableState::default();
        forwarders_state.select(Some(0));

        let mut lookups_state = ratatui::widgets::TableState::default();
        lookups_state.select(Some(0));

        let (
            auto_refresh,
            sort_column,
            sort_direction,
            last_search_query,
            search_history,
            color_theme,
            search_defaults,
            keybind_overrides,
            list_defaults,
            internal_logs_defaults,
        ) = match persisted {
            Some(state) => (
                state.auto_refresh,
                state::parse_sort_column(&state.sort_column),
                state::parse_sort_direction(&state.sort_direction),
                state.last_search_query,
                state.search_history,
                state.selected_theme,
                state.search_defaults,
                state.keybind_overrides,
                state.list_defaults,
                state.internal_logs_defaults,
            ),
            None => (
                false,
                SortColumn::Sid,
                SortDirection::Asc,
                None,
                Vec::new(),
                ColorTheme::Default,
                SearchDefaults::default(),
                KeybindOverrides::default(),
                ListDefaults::default(),
                splunk_config::InternalLogsDefaults::default(),
            ),
        };

        Self {
            current_screen: CurrentScreen::Search,
            search_input: last_search_query.clone().unwrap_or_default(),
            search_cursor_position: last_search_query.unwrap_or_default().len(),
            running_query: None,
            search_status: String::from("Press Enter to execute search"),
            search_results: Vec::new(),
            search_scroll_offset: 0,
            search_sid: None,
            search_results_total_count: None,
            search_results_page_size: DEFAULT_SEARCH_PAGE_SIZE,
            search_has_more_results: false,
            indexes: None,
            indexes_state,
            jobs: None,
            jobs_state,
            saved_searches: None,
            saved_searches_state,
            internal_logs: None,
            internal_logs_state,
            cluster_info: None,
            cluster_peers: None,
            cluster_peers_state,
            cluster_view_mode: crate::app::state::ClusterViewMode::Summary,
            health_info: None,
            license_info: None,
            kvstore_status: None,
            apps: None,
            apps_state,
            users: None,
            users_state,
            search_peers: None,
            search_peers_state,
            search_peers_pagination: crate::app::state::ListPaginationState::new(30, 1000),
            inputs: None,
            inputs_state,
            inputs_pagination: crate::app::state::ListPaginationState::new(30, 1000),
            overview_data: None,
            multi_instance_data: None,
            multi_instance_selected_index: 0,
            fired_alerts: None,
            fired_alerts_state,
            fired_alerts_pagination: crate::app::state::ListPaginationState::new(30, 1000),
            forwarders: None,
            forwarders_state,
            forwarders_pagination: crate::app::state::ListPaginationState::new(30, 1000),
            lookups: None,
            lookups_state,
            lookups_pagination: crate::app::state::ListPaginationState::new(30, 1000),
            config_files: None,
            config_files_state,
            selected_config_file: None,
            config_stanzas: None,
            config_stanzas_state,
            selected_stanza: None,
            config_view_mode: crate::ui::screens::configs::ConfigViewMode::FileList,
            config_search_mode: false,
            config_search_query: String::new(),
            config_search_before_edit: None,
            filtered_stanza_indices: Vec::new(),
            loading: false,
            progress: 0.0,
            toasts: Vec::new(),
            auto_refresh,
            popup: None,

            color_theme,
            theme: Theme::from(color_theme),
            search_filter: None,
            is_filtering: false,
            filter_input: String::new(),
            filter_before_edit: None,
            filtered_job_indices: Vec::new(),
            sort_state: SortState {
                column: sort_column,
                direction: sort_direction,
            },
            selected_jobs: HashSet::new(),
            health_state: HealthState::Unknown,
            search_history,
            history_index: None,
            saved_search_input: String::new(),
            search_defaults,
            keybind_overrides,
            list_defaults: list_defaults.clone(),
            internal_logs_defaults,
            indexes_pagination: ListPaginationState::new(
                list_defaults.page_size_for(ListType::Indexes),
                list_defaults.max_items,
            ),
            jobs_pagination: ListPaginationState::new(
                list_defaults.page_size_for(ListType::Jobs),
                list_defaults.max_items,
            ),
            apps_pagination: ListPaginationState::new(
                list_defaults.page_size_for(ListType::Apps),
                list_defaults.max_items,
            ),
            users_pagination: ListPaginationState::new(
                list_defaults.page_size_for(ListType::Users),
                list_defaults.max_items,
            ),
            export_input: String::new(),
            export_format: ExportFormat::Json,
            export_target: None,
            current_error: None,
            error_scroll_offset: 0,
            index_details_scroll_offset: 0,
            help_scroll_offset: 0,
            last_area: Rect::default(),

            // Connection context (RQ-0134)
            profile_name: connection_ctx.profile_name,
            base_url: Some(connection_ctx.base_url),
            auth_mode: Some(connection_ctx.auth_mode),
            server_version: None,
            server_build: None,

            // Search input mode (RQ-0101)
            search_input_mode: crate::app::state::SearchInputMode::QueryFocused,

            // SPL validation state (RQ-0240)
            spl_validation_state: SplValidationState::default(),
            spl_validation_pending: false,
            last_input_change: None,
        }
    }

    /// Exports the current state for persistence.
    pub fn get_persisted_state(&self) -> PersistedState {
        PersistedState {
            auto_refresh: self.auto_refresh,
            sort_column: self.sort_state.column.as_str().to_string(),
            sort_direction: self.sort_state.direction.as_str().to_string(),
            last_search_query: if self.search_filter.is_some() {
                self.search_filter.clone()
            } else if !self.search_input.is_empty() {
                Some(self.search_input.clone())
            } else {
                None
            },
            search_history: self.search_history.clone(),
            selected_theme: self.color_theme,
            search_defaults: self.search_defaults.clone(),
            keybind_overrides: self.keybind_overrides.clone(),
            list_defaults: self.list_defaults.clone(),
            internal_logs_defaults: self.internal_logs_defaults.clone(),
        }
    }

    /// Update the health state and emit a toast notification on transition to unhealthy.
    pub fn set_health_state(&mut self, new_state: HealthState) {
        // Only emit toast on Healthy -> Unhealthy transition
        if self.health_state == HealthState::Healthy && new_state == HealthState::Unhealthy {
            self.toasts
                .push(Toast::warning("Splunk health status changed to unhealthy"));
        }
        self.health_state = new_state;
    }

    /// Set server info from health check (RQ-0134).
    ///
    /// Populates server version and build info for display in the header.
    pub fn set_server_info(&mut self, server_info: &splunk_client::models::ServerInfo) {
        self.server_version = Some(server_info.version.clone());
        self.server_build = Some(server_info.build.clone());
    }

    /// Set search results (virtualization: formatting is deferred to render time).
    pub fn set_search_results(&mut self, results: Vec<Value>) {
        self.search_results = results;
        self.search_results_total_count = Some(self.search_results.len() as u64);
        self.search_has_more_results = false;
        // Reset scroll offset when new results arrive
        self.search_scroll_offset = 0;
    }

    /// Append more search results (for pagination, virtualization: no eager formatting).
    pub fn append_search_results(&mut self, mut results: Vec<Value>, total: Option<u64>) {
        let results_count = results.len() as u64;
        self.search_results.append(&mut results);
        self.search_results_total_count = total;

        // Determine if more results may exist
        self.search_has_more_results = if let Some(t) = total {
            // When total is known, use it directly
            (self.search_results.len() as u64) < t
        } else {
            // When total is None, infer from page fullness:
            // If we got exactly page_size results, there might be more.
            // If we got fewer, we're likely at the end.
            results_count >= self.search_results_page_size
        };
        // Note: No pre-formatting - results are formatted on-demand during rendering
    }

    /// Returns the load action for the current screen, if one is needed.
    /// Used after screen navigation to trigger data loading.
    pub fn load_action_for_screen(&self) -> Option<Action> {
        match self.current_screen {
            CurrentScreen::Search => None, // Search doesn't need pre-loading
            CurrentScreen::Indexes => Some(Action::LoadIndexes {
                count: self.indexes_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::Cluster => Some(Action::LoadClusterInfo),
            CurrentScreen::Jobs => Some(Action::LoadJobs {
                count: self.jobs_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::JobInspect => None, // Already loaded when entering inspect mode
            CurrentScreen::Health => Some(Action::LoadHealth),
            CurrentScreen::License => Some(Action::LoadLicense),
            CurrentScreen::Kvstore => Some(Action::LoadKvstore),
            CurrentScreen::SavedSearches => Some(Action::LoadSavedSearches),
            CurrentScreen::InternalLogs => Some(Action::LoadInternalLogs {
                count: self.internal_logs_defaults.count,
                earliest: self.internal_logs_defaults.earliest_time.clone(),
            }),
            CurrentScreen::Apps => Some(Action::LoadApps {
                count: self.apps_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::Users => Some(Action::LoadUsers {
                count: self.users_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::SearchPeers => Some(Action::LoadSearchPeers {
                count: self.search_peers_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::Inputs => Some(Action::LoadInputs {
                count: self.inputs_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::Configs => Some(Action::LoadConfigFiles),
            CurrentScreen::FiredAlerts => Some(Action::LoadFiredAlerts),
            CurrentScreen::Forwarders => Some(Action::LoadForwarders {
                count: self.forwarders_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::Lookups => Some(Action::LoadLookups {
                count: self.lookups_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::Settings => Some(Action::SwitchToSettings),
            CurrentScreen::Overview => Some(Action::LoadOverview),
            CurrentScreen::MultiInstance => Some(Action::LoadMultiInstanceOverview),
        }
    }

    /// Returns a load-more action for the current screen if pagination is available.
    pub fn load_more_action_for_current_screen(&self) -> Option<Action> {
        match self.current_screen {
            CurrentScreen::Indexes => {
                if self.indexes_pagination.can_load_more() {
                    Some(Action::LoadIndexes {
                        count: self.indexes_pagination.page_size,
                        offset: self.indexes_pagination.current_offset,
                    })
                } else {
                    None
                }
            }
            CurrentScreen::Jobs => {
                if self.jobs_pagination.can_load_more() {
                    Some(Action::LoadJobs {
                        count: self.jobs_pagination.page_size,
                        offset: self.jobs_pagination.current_offset,
                    })
                } else {
                    None
                }
            }
            CurrentScreen::Apps => {
                if self.apps_pagination.can_load_more() {
                    Some(Action::LoadApps {
                        count: self.apps_pagination.page_size,
                        offset: self.apps_pagination.current_offset,
                    })
                } else {
                    None
                }
            }
            CurrentScreen::Users => {
                if self.users_pagination.can_load_more() {
                    Some(Action::LoadUsers {
                        count: self.users_pagination.page_size,
                        offset: self.users_pagination.current_offset,
                    })
                } else {
                    None
                }
            }
            CurrentScreen::SearchPeers => {
                if self.search_peers_pagination.can_load_more() {
                    Some(Action::LoadSearchPeers {
                        count: self.search_peers_pagination.page_size,
                        offset: self.search_peers_pagination.current_offset,
                    })
                } else {
                    None
                }
            }
            CurrentScreen::Forwarders => {
                if self.forwarders_pagination.can_load_more() {
                    Some(Action::LoadForwarders {
                        count: self.forwarders_pagination.page_size,
                        offset: self.forwarders_pagination.current_offset,
                    })
                } else {
                    None
                }
            }
            CurrentScreen::Lookups => {
                if self.lookups_pagination.can_load_more() {
                    Some(Action::LoadLookups {
                        count: self.lookups_pagination.page_size,
                        offset: self.lookups_pagination.current_offset,
                    })
                } else {
                    None
                }
            }
            _ => None,
        }
    }

    pub fn maybe_fetch_more_results(&self) -> Option<Action> {
        // Only fetch if we have a SID, more results exist, and we're not already loading
        if self.search_sid.is_none() || !self.search_has_more_results || self.loading {
            return None;
        }

        // Trigger fetch when user is within threshold items of the end
        let threshold = DEFAULT_SCROLL_THRESHOLD;
        let loaded_count = self.search_results.len();
        let visible_end = self.search_scroll_offset.saturating_add(threshold);

        if visible_end >= loaded_count {
            Some(Action::LoadMoreSearchResults {
                sid: self.search_sid.clone()?,
                offset: loaded_count as u64,
                count: self.search_results_page_size,
            })
        } else {
            None
        }
    }

    /// Add a query to history, moving it to front if it exists, and truncating to max 50 items.
    pub(crate) fn add_to_history(&mut self, query: String) {
        if query.trim().is_empty() {
            return;
        }

        // Remove if already exists to move to front
        if let Some(pos) = self.search_history.iter().position(|h| h == &query) {
            self.search_history.remove(pos);
        }

        self.search_history.insert(0, query);

        // Truncate to max items
        if self.search_history.len() > DEFAULT_HISTORY_MAX_ITEMS {
            self.search_history.truncate(DEFAULT_HISTORY_MAX_ITEMS);
        }

        // Reset history navigation
        self.history_index = None;
    }

    /// Create a single-line, truncated preview for clipboard toast notifications.
    pub(crate) fn clipboard_preview(content: &str) -> String {
        // Normalize whitespace for toasts (avoid multi-line notifications).
        let normalized = content.replace(['\n', '\r', '\t'], " ");

        let max_chars = DEFAULT_CLIPBOARD_PREVIEW_CHARS;
        let ellipsis = "...";

        let char_count = normalized.chars().count();
        if char_count <= max_chars {
            return normalized;
        }

        let take = max_chars.saturating_sub(ellipsis.len());
        let mut out = String::with_capacity(max_chars);
        for (i, ch) in normalized.chars().enumerate() {
            if i >= take {
                break;
            }
            out.push(ch);
        }
        out.push_str(ellipsis);
        out
    }

    /// Handle keyboard input - returns Action if one should be dispatched.
    pub fn handle_input(&mut self, key: KeyEvent) -> Option<Action> {
        if self.popup.is_some() {
            return self.handle_popup_input(key);
        }

        if self.current_screen == CurrentScreen::Jobs && self.is_filtering {
            return self.handle_jobs_filter_input(key);
        }

        // Configs search mode takes precedence over other bindings
        if self.current_screen == CurrentScreen::Configs && self.config_search_mode {
            return self.handle_config_search_input(key);
        }

        // Global 'e' keybinding to show error details when an error is present.
        // This takes precedence over screen-specific bindings (like "enable app" on Apps screen)
        // because viewing error details is more urgent.
        if key.code == KeyCode::Char('e')
            && key.modifiers.is_empty()
            && self.current_error.is_some()
        {
            return Some(Action::ShowErrorDetailsFromCurrent);
        }

        // When in Search screen with QueryFocused mode, skip global binding resolution
        // for printable characters to allow typing (RQ-0101 fix).
        // Also skip Tab/BackTab to allow mode switching within the search screen.
        // Also skip cursor movement/editing keys for query editing (RQ-0110).
        let skip_global_bindings = self.current_screen == CurrentScreen::Search
            && matches!(self.search_input_mode, SearchInputMode::QueryFocused)
            && (is_printable_char(key) || is_mode_switch_key(key) || is_cursor_editing_key(key));

        if !skip_global_bindings
            && let Some(action) = crate::input::keymap::resolve_action(self.current_screen, key)
        {
            return Some(action);
        }

        self.dispatch_screen_input(key)
    }

    /// Handle periodic tick events - returns Action if one should be dispatched.
    pub fn handle_tick(&mut self) -> Option<Action> {
        // Handle debounced SPL validation first
        if let Some(action) = self.handle_validation_tick() {
            return Some(action);
        }

        if self.current_screen == CurrentScreen::Jobs
            && self.auto_refresh
            && self.popup.is_none()
            && !self.is_filtering
        {
            // Auto-refresh resets pagination to get fresh data
            Some(Action::LoadJobs {
                count: self.jobs_pagination.page_size,
                offset: 0,
            })
        } else if self.current_screen == CurrentScreen::InternalLogs
            && self.auto_refresh
            && self.popup.is_none()
        {
            Some(Action::LoadInternalLogs {
                count: self.internal_logs_defaults.count,
                earliest: self.internal_logs_defaults.earliest_time.clone(),
            })
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests;
