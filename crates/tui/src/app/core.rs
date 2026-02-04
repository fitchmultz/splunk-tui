//! Core App lifecycle methods.
//!
//! Responsibilities:
//! - App construction (new, default)
//! - State persistence (get_persisted_state)
//! - Initial state setup
//!
//! Non-responsibilities:
//! - Does NOT handle runtime updates (see actions/)
//! - Does NOT handle input (see input/)
//! - Does NOT render (see render.rs)

use crate::app::state::{
    ClusterViewMode, CurrentScreen, ListPaginationState, SearchInputMode, SortState,
};
use crate::app::structs::{App, ConnectionContext, SplValidationState};
use splunk_client::SearchMode;
use splunk_config::constants::DEFAULT_SEARCH_PAGE_SIZE;
use splunk_config::{
    ColorTheme, KeybindOverrides, ListDefaults, ListType, PersistedState, SearchDefaults, Theme,
};

impl Default for App {
    fn default() -> Self {
        Self::new(None, ConnectionContext::default())
    }
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

        let mut macros_state = ratatui::widgets::ListState::default();
        macros_state.select(Some(0));

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

        let mut audit_state = ratatui::widgets::TableState::default();
        audit_state.select(Some(0));

        let mut dashboards_state = ratatui::widgets::ListState::default();
        dashboards_state.select(Some(0));

        let mut data_models_state = ratatui::widgets::ListState::default();
        data_models_state.select(Some(0));

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
                crate::app::state::parse_sort_column(&state.sort_column),
                crate::app::state::parse_sort_direction(&state.sort_direction),
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
                crate::app::state::SortColumn::Sid,
                crate::app::state::SortDirection::Asc,
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
            search_cursor_position: last_search_query.unwrap_or_default().chars().count(),
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
            macros: None,
            macros_state,
            internal_logs: None,
            internal_logs_state,
            cluster_info: None,
            cluster_peers: None,
            cluster_peers_state,
            cluster_view_mode: ClusterViewMode::Summary,
            health_info: None,
            license_info: None,
            kvstore_status: None,
            apps: None,
            apps_state,
            users: None,
            users_state,
            roles: None,
            roles_state: ratatui::widgets::ListState::default(),
            capabilities: None,
            search_peers: None,
            search_peers_state,
            search_peers_pagination: ListPaginationState::new(30, 1000),
            inputs: None,
            inputs_state,
            inputs_pagination: ListPaginationState::new(30, 1000),
            overview_data: None,
            multi_instance_data: None,
            multi_instance_selected_index: 0,
            fired_alerts: None,
            fired_alerts_state,
            fired_alerts_pagination: ListPaginationState::new(30, 1000),
            forwarders: None,
            forwarders_state,
            forwarders_pagination: ListPaginationState::new(30, 1000),
            lookups: None,
            lookups_state,
            lookups_pagination: ListPaginationState::new(30, 1000),
            audit_events: None,
            audit_state,
            dashboards: None,
            dashboards_state,
            dashboards_pagination: ListPaginationState::new(30, 1000),
            data_models: None,
            data_models_state,
            data_models_pagination: ListPaginationState::new(30, 1000),
            workload_pools: None,
            workload_pools_state: ratatui::widgets::TableState::default(),
            workload_pools_pagination: ListPaginationState::new(30, 1000),
            workload_rules: None,
            workload_rules_state: ratatui::widgets::TableState::default(),
            workload_rules_pagination: ListPaginationState::new(30, 1000),
            workload_view_mode: crate::app::state::WorkloadViewMode::Pools,
            shc_status: None,
            shc_members: None,
            shc_captain: None,
            shc_config: None,
            shc_members_state: ratatui::widgets::TableState::default(),
            shc_view_mode: crate::app::state::ShcViewMode::Summary,
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
            loading_since: None,
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
            selected_jobs: std::collections::HashSet::new(),
            health_state: crate::app::state::HealthState::Unknown,
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
            export_format: crate::action::ExportFormat::Json,
            export_target: None,
            current_error: None,
            error_scroll_offset: 0,
            index_details_scroll_offset: 0,
            help_scroll_offset: 0,
            last_area: ratatui::layout::Rect::default(),
            profile_name: connection_ctx.profile_name,
            base_url: Some(connection_ctx.base_url),
            auth_mode: Some(connection_ctx.auth_mode),
            server_version: None,
            server_build: None,
            search_input_mode: SearchInputMode::QueryFocused,
            spl_validation_state: SplValidationState::default(),
            spl_validation_pending: false,
            last_input_change: None,
            search_mode: SearchMode::Normal,
            realtime_window: None,
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
}
