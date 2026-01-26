//! Application state and rendering.
//!
//! This module contains the main application state, input handling,
//! and rendering logic for the TUI.

use crate::action::{Action, ExportFormat};
use crate::ui::Toast;
use crate::ui::popup::{Popup, PopupType};
use crate::ui::screens::{apps, cluster, health, indexes, saved_searches, search, settings, users};
use crossterm::event::{KeyEvent, MouseEvent, MouseEventKind};
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, ListState, Paragraph, TableState},
};
use serde_json::Value;
use splunk_client::models::{
    App as SplunkApp, ClusterInfo, HealthCheckOutput, Index, LogEntry, SavedSearch,
    SearchJobStatus, User,
};
use splunk_config::PersistedState;
use std::collections::HashSet;

/// Health state of the Splunk instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    /// Health status is unknown (initial state or check pending)
    Unknown,
    /// Splunk is healthy
    Healthy,
    /// Splunk is unhealthy
    Unhealthy,
}

impl HealthState {
    /// Map Splunk health string to HealthState.
    ///
    /// Splunk returns "green", "yellow", or "red" for health status.
    /// - "green" → Healthy
    /// - "yellow" → Unknown (degraded but not failed)
    /// - "red" → Unhealthy
    /// - any other value → Unknown
    pub fn from_health_str(health: &str) -> Self {
        match health.to_lowercase().as_str() {
            "green" => HealthState::Healthy,
            "red" => HealthState::Unhealthy,
            _ => HealthState::Unknown,
        }
    }
}

/// Layout constants for UI components.
pub const HEADER_HEIGHT: u16 = 3;
pub const FOOTER_HEIGHT: u16 = 3;

/// Current active screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentScreen {
    Search,
    Indexes,
    Cluster,
    Jobs,
    JobInspect,
    Health,
    SavedSearches,
    InternalLogs,
    Apps,
    Users,
    Settings,
}

/// Sort column for jobs table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Sid,
    Status,
    Duration,
    Results,
    Events,
}

impl SortColumn {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sid => "sid",
            Self::Status => "status",
            Self::Duration => "duration",
            Self::Results => "results",
            Self::Events => "events",
        }
    }
}

/// Parse sort column from string (for deserialization).
fn parse_sort_column(s: &str) -> SortColumn {
    match s.to_lowercase().as_str() {
        "sid" => SortColumn::Sid,
        "status" => SortColumn::Status,
        "duration" => SortColumn::Duration,
        "results" => SortColumn::Results,
        "events" => SortColumn::Events,
        _ => SortColumn::Sid, // Default fallback
    }
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }
}

/// Parse sort direction from string (for deserialization).
fn parse_sort_direction(s: &str) -> SortDirection {
    match s.to_lowercase().as_str() {
        "asc" => SortDirection::Asc,
        "desc" => SortDirection::Desc,
        _ => SortDirection::Asc, // Default fallback
    }
}

/// Sort state for jobs table.
#[derive(Debug, Clone, Copy)]
pub struct SortState {
    pub column: SortColumn,
    pub direction: SortDirection,
}

impl Default for SortState {
    fn default() -> Self {
        Self::new()
    }
}

impl SortState {
    pub fn new() -> Self {
        Self {
            column: SortColumn::Sid,
            direction: SortDirection::Asc,
        }
    }

    pub fn cycle(&mut self) {
        self.column = match self.column {
            SortColumn::Sid => SortColumn::Status,
            SortColumn::Status => SortColumn::Duration,
            SortColumn::Duration => SortColumn::Results,
            SortColumn::Results => SortColumn::Events,
            SortColumn::Events => SortColumn::Sid,
        };
    }

    pub fn toggle_direction(&mut self) {
        self.direction = match self.direction {
            SortDirection::Asc => SortDirection::Desc,
            SortDirection::Desc => SortDirection::Asc,
        };
    }
}

/// Main application state.
pub struct App {
    pub current_screen: CurrentScreen,
    pub search_input: String,
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
    pub indexes_state: ListState,
    pub jobs: Option<Vec<SearchJobStatus>>,
    pub jobs_state: TableState,
    pub saved_searches: Option<Vec<SavedSearch>>,
    pub saved_searches_state: ListState,
    pub internal_logs: Option<Vec<LogEntry>>,
    pub internal_logs_state: TableState,
    pub cluster_info: Option<ClusterInfo>,
    pub health_info: Option<HealthCheckOutput>,
    pub apps: Option<Vec<SplunkApp>>,
    pub apps_state: ListState,
    pub users: Option<Vec<User>>,
    pub users_state: ListState,

    // UI State
    pub loading: bool,
    pub progress: f32,
    pub toasts: Vec<Toast>,
    pub auto_refresh: bool,
    pub popup: Option<Popup>,

    // Jobs filter state
    pub search_filter: Option<String>,
    pub is_filtering: bool,
    pub filter_input: String,
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

    // Export state
    pub export_input: String,
    pub export_format: ExportFormat,

    // Error state
    pub current_error: Option<crate::error_details::ErrorDetails>,
    pub error_scroll_offset: usize,

    // Layout tracking
    pub last_area: Rect,
}

impl Default for App {
    fn default() -> Self {
        Self::new(None)
    }
}

impl App {
    pub fn new(persisted: Option<PersistedState>) -> Self {
        let mut indexes_state = ListState::default();
        indexes_state.select(Some(0));

        let mut jobs_state = TableState::default();
        jobs_state.select(Some(0));

        let mut saved_searches_state = ListState::default();
        saved_searches_state.select(Some(0));

        let mut internal_logs_state = TableState::default();
        internal_logs_state.select(Some(0));

        let mut apps_state = ListState::default();
        apps_state.select(Some(0));

        let mut users_state = ListState::default();
        users_state.select(Some(0));

        let (auto_refresh, sort_column, sort_direction, last_search_query, search_history) =
            match persisted {
                Some(state) => (
                    state.auto_refresh,
                    parse_sort_column(&state.sort_column),
                    parse_sort_direction(&state.sort_direction),
                    state.last_search_query,
                    state.search_history,
                ),
                None => (false, SortColumn::Sid, SortDirection::Asc, None, Vec::new()),
            };

        Self {
            current_screen: CurrentScreen::Search,
            search_input: last_search_query.unwrap_or_default(),
            search_status: String::from("Press Enter to execute search"),
            search_results: Vec::new(),
            search_scroll_offset: 0,
            search_sid: None,
            search_results_total_count: None,
            search_results_page_size: 100,
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
            health_info: None,
            apps: None,
            apps_state,
            users: None,
            users_state,
            loading: false,
            progress: 0.0,
            toasts: Vec::new(),
            auto_refresh,
            popup: None,
            search_filter: None,
            is_filtering: false,
            filter_input: String::new(),
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
            export_input: String::new(),
            export_format: ExportFormat::Json,
            current_error: None,
            error_scroll_offset: 0,
            last_area: Rect::default(),
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

    /// Check if we should load more results based on scroll position.
    /// Returns the LoadMoreSearchResults action if needed.
    pub fn maybe_fetch_more_results(&self) -> Option<Action> {
        // Only fetch if we have a SID, more results exist, and we're not already loading
        if self.search_sid.is_none() || !self.search_has_more_results || self.loading {
            return None;
        }

        // Trigger fetch when user is within 10 items of the end
        let threshold = 10;
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
    fn add_to_history(&mut self, query: String) {
        if query.trim().is_empty() {
            return;
        }

        // Remove if already exists to move to front
        if let Some(pos) = self.search_history.iter().position(|h| h == &query) {
            self.search_history.remove(pos);
        }

        self.search_history.insert(0, query);

        // Truncate to 50 items
        if self.search_history.len() > 50 {
            self.search_history.truncate(50);
        }

        // Reset history navigation
        self.history_index = None;
    }

    /// Handle keyboard input - returns Action if one should be dispatched.
    pub fn handle_input(&mut self, key: KeyEvent) -> Option<Action> {
        if self.popup.is_some() {
            return self.handle_popup_input(key);
        }
        match self.current_screen {
            CurrentScreen::Search => self.handle_search_input(key),
            CurrentScreen::Jobs => self.handle_jobs_input(key),
            CurrentScreen::Indexes => self.handle_indexes_input(key),
            CurrentScreen::Cluster => self.handle_cluster_input(key),
            CurrentScreen::JobInspect => self.handle_job_inspect_input(key),
            CurrentScreen::Health => self.handle_health_input(key),
            CurrentScreen::SavedSearches => self.handle_saved_searches_input(key),
            CurrentScreen::InternalLogs => self.handle_internal_logs_input(key),
            CurrentScreen::Apps => self.handle_apps_input(key),
            CurrentScreen::Users => self.handle_users_input(key),
            CurrentScreen::Settings => self.handle_settings_input(key),
        }
    }

    /// Handle mouse input - returns Action if one should be dispatched.
    pub fn handle_mouse(&mut self, mouse: MouseEvent) -> Option<Action> {
        if self.popup.is_some() {
            return None;
        }
        match mouse.kind {
            MouseEventKind::ScrollUp => Some(Action::NavigateUp),
            MouseEventKind::ScrollDown => Some(Action::NavigateDown),
            MouseEventKind::Down(crossterm::event::MouseButton::Left) => {
                // Check for footer navigation
                if mouse.row >= self.last_area.height.saturating_sub(2)
                    && mouse.row < self.last_area.height.saturating_sub(1)
                {
                    return self.handle_footer_click(mouse.column);
                }

                // Check for content area clicks
                if mouse.row >= HEADER_HEIGHT && mouse.row < self.last_area.height - FOOTER_HEIGHT {
                    return self.handle_content_click(mouse.row, mouse.column);
                }
                None
            }
            _ => None,
        }
    }

    /// Handle clicks in the footer area for screen navigation.
    fn handle_footer_click(&mut self, col: u16) -> Option<Action> {
        // Content in the footer block starts at column 1 (due to border)
        // Offset if loading message is present
        let offset = if self.loading { 18 } else { 0 };

        // Adjusted column ranges to match the spans rendered in render()
        // Tab 1: " 1:Search " (10 chars) -> Indices 0..10
        if col > offset && col <= offset + 10 {
            self.current_screen = CurrentScreen::Search;
            None
        // Tab 2: " 2:Indexes " (10 chars) -> Indices 10..20
        } else if col > offset + 10 && col <= offset + 20 {
            self.current_screen = CurrentScreen::Indexes;
            Some(Action::LoadIndexes)
        // Tab 3: " 3:Cluster " (10 chars) -> Indices 20..30
        } else if col > offset + 20 && col <= offset + 30 {
            self.current_screen = CurrentScreen::Cluster;
            Some(Action::LoadClusterInfo)
        // Tab 4: " 4:Jobs " (8 chars) -> Indices 30..38
        } else if col > offset + 30 && col <= offset + 38 {
            self.current_screen = CurrentScreen::Jobs;
            Some(Action::LoadJobs)
        // Tab 5: " 5:Health " (10 chars) -> Indices 38..48
        } else if col > offset + 38 && col <= offset + 48 {
            self.current_screen = CurrentScreen::Health;
            Some(Action::LoadHealth)
        // Tab 6: " 6:Saved " (9 chars) -> Indices 48..57
        } else if col > offset + 48 && col <= offset + 57 {
            self.current_screen = CurrentScreen::SavedSearches;
            Some(Action::LoadSavedSearches)
        // Tab 7: " 7:Logs " (8 chars) -> Indices 57..65
        } else if col > offset + 57 && col <= offset + 65 {
            self.current_screen = CurrentScreen::InternalLogs;
            Some(Action::LoadInternalLogs)
        // Tab 8: " 8:Apps " (8 chars) -> Indices 65..73
        } else if col > offset + 65 && col <= offset + 73 {
            self.current_screen = CurrentScreen::Apps;
            Some(Action::LoadApps)
        // Tab 9: " 9:Users " (9 chars) -> Indices 73..82
        } else if col > offset + 73 && col <= offset + 82 {
            self.current_screen = CurrentScreen::Users;
            Some(Action::LoadUsers)
        // Tab q: " q:Quit " (8 chars) -> After "|" at index 83
        } else if col > offset + 83 && col <= offset + 91 {
            Some(Action::Quit)
        } else {
            None
        }
    }

    /// Handle clicks in the main content area.
    fn handle_content_click(&mut self, row: u16, _col: u16) -> Option<Action> {
        match self.current_screen {
            CurrentScreen::Jobs => {
                // If filtering is active, the table area is pushed down by 3 rows
                let filter_offset = if self.is_filtering || self.search_filter.is_some() {
                    3
                } else {
                    0
                };

                // Jobs table has a header row at content start + 1
                // Data starts at content start + 2
                let data_start = HEADER_HEIGHT + filter_offset + 2;
                if row >= data_start {
                    let relative_row = (row - data_start) as usize;
                    let offset = self.jobs_state.offset();
                    let index = offset + relative_row;

                    if index < self.filtered_jobs_len() {
                        let already_selected = self.jobs_state.selected() == Some(index);
                        self.jobs_state.select(Some(index));
                        if already_selected {
                            return Some(Action::InspectJob);
                        }
                    }
                }
            }
            CurrentScreen::Indexes => {
                // Indexes list starts at HEADER_HEIGHT + 1 (no table header)
                let data_start = HEADER_HEIGHT + 1;
                if row >= data_start {
                    let relative_row = (row - data_start) as usize;
                    let offset = self.indexes_state.offset();
                    let index = offset + relative_row;

                    if let Some(indexes) = &self.indexes
                        && index < indexes.len()
                    {
                        self.indexes_state.select(Some(index));
                    }
                }
            }
            _ => {}
        }
        None
    }

    /// Handle periodic tick events - returns Action if one should be dispatched.
    pub fn handle_tick(&self) -> Option<Action> {
        if self.current_screen == CurrentScreen::Jobs
            && self.auto_refresh
            && self.popup.is_none()
            && !self.is_filtering
        {
            Some(Action::LoadJobs)
        } else if self.current_screen == CurrentScreen::InternalLogs
            && self.auto_refresh
            && self.popup.is_none()
        {
            Some(Action::LoadInternalLogs)
        } else {
            None
        }
    }

    /// Handle keyboard input when a popup is active.
    fn handle_popup_input(&mut self, key: KeyEvent) -> Option<Action> {
        use crossterm::event::KeyCode;
        match (self.popup.as_ref().map(|p| &p.kind), key.code) {
            (Some(PopupType::Help), KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')) => {
                self.popup = None;
                None
            }
            (Some(PopupType::ConfirmCancel(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sid = if let Some(Popup {
                    kind: PopupType::ConfirmCancel(s),
                    ..
                }) = self.popup.take()
                {
                    s
                } else {
                    unreachable!()
                };
                Some(Action::CancelJob(sid))
            }
            (Some(PopupType::ConfirmDelete(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sid = if let Some(Popup {
                    kind: PopupType::ConfirmDelete(s),
                    ..
                }) = self.popup.take()
                {
                    s
                } else {
                    unreachable!()
                };
                Some(Action::DeleteJob(sid))
            }
            (Some(PopupType::ConfirmCancelBatch(sids)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sids = sids.clone();
                self.popup = None;
                Some(Action::CancelJobsBatch(sids))
            }
            (Some(PopupType::ConfirmDeleteBatch(sids)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sids = sids.clone();
                self.popup = None;
                Some(Action::DeleteJobsBatch(sids))
            }
            (Some(PopupType::ExportSearch), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (Some(PopupType::ExportSearch), KeyCode::Enter) => {
                if !self.export_input.is_empty() && !self.search_results.is_empty() {
                    let path = std::path::PathBuf::from(&self.export_input);
                    let format = self.export_format;
                    let results = self.search_results.clone();
                    self.popup = None;
                    Some(Action::ExportSearchResults(results, path, format))
                } else {
                    None
                }
            }
            (Some(PopupType::ExportSearch), KeyCode::Tab) => {
                self.export_format = match self.export_format {
                    ExportFormat::Json => ExportFormat::Csv,
                    ExportFormat::Csv => ExportFormat::Json,
                };
                // Automatically update extension if it matches the previous format
                match self.export_format {
                    ExportFormat::Json => {
                        if self.export_input.ends_with(".csv") {
                            self.export_input.truncate(self.export_input.len() - 4);
                            self.export_input.push_str(".json");
                        }
                    }
                    ExportFormat::Csv => {
                        if self.export_input.ends_with(".json") {
                            self.export_input.truncate(self.export_input.len() - 5);
                            self.export_input.push_str(".csv");
                        }
                    }
                }
                self.update_export_popup();
                None
            }
            (Some(PopupType::ExportSearch), KeyCode::Backspace) => {
                self.export_input.pop();
                self.update_export_popup();
                None
            }
            (Some(PopupType::ExportSearch), KeyCode::Char(c)) => {
                self.export_input.push(c);
                self.update_export_popup();
                None
            }
            (
                Some(PopupType::ErrorDetails),
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('e'),
            ) => {
                self.popup = None;
                None
            }
            (Some(PopupType::ErrorDetails), KeyCode::Char('j') | KeyCode::Down) => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_add(1);
                None
            }
            (Some(PopupType::ErrorDetails), KeyCode::Char('k') | KeyCode::Up) => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_sub(1);
                None
            }
            (Some(PopupType::ErrorDetails), KeyCode::PageDown) => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_add(10);
                None
            }
            (Some(PopupType::ErrorDetails), KeyCode::PageUp) => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_sub(10);
                None
            }
            (
                Some(
                    PopupType::ConfirmCancel(_)
                    | PopupType::ConfirmDelete(_)
                    | PopupType::ConfirmCancelBatch(_)
                    | PopupType::ConfirmDeleteBatch(_),
                ),
                KeyCode::Char('n') | KeyCode::Esc,
            ) => {
                self.popup = None;
                None
            }
            _ => None,
        }
    }

    /// Refresh the export popup content based on current input and format.
    fn update_export_popup(&mut self) {
        if let Some(Popup {
            kind: PopupType::ExportSearch,
            ..
        }) = &mut self.popup
        {
            let format_str = match self.export_format {
                ExportFormat::Json => "JSON",
                ExportFormat::Csv => "CSV",
            };
            let popup = Popup::builder(PopupType::ExportSearch)
                .title("Export Search Results")
                .content(format!(
                    "File: {}\nFormat: {} (Tab to toggle)\n\nPress Enter to export, Esc to cancel",
                    self.export_input, format_str
                ))
                .build();
            self.popup = Some(popup);
        }
    }

    fn handle_search_input(&mut self, key: KeyEvent) -> Option<Action> {
        use crossterm::event::{KeyCode, KeyModifiers};

        // Handle Ctrl+j/k for result navigation while in input
        if key.modifiers.contains(KeyModifiers::CONTROL) {
            match key.code {
                KeyCode::Char('j') => return Some(Action::NavigateDown),
                KeyCode::Char('k') => return Some(Action::NavigateUp),
                _ => {}
            }
        }

        match key.code {
            KeyCode::Char('q') if key.modifiers.is_empty() => Some(Action::Quit),
            KeyCode::Char('1') if key.modifiers.is_empty() => {
                self.current_screen = CurrentScreen::Search;
                None
            }
            KeyCode::Char('2') if key.modifiers.is_empty() => {
                self.current_screen = CurrentScreen::Indexes;
                Some(Action::LoadIndexes)
            }
            KeyCode::Char('3') if key.modifiers.is_empty() => {
                self.current_screen = CurrentScreen::Cluster;
                Some(Action::LoadClusterInfo)
            }
            KeyCode::Char('4') if key.modifiers.is_empty() => {
                self.current_screen = CurrentScreen::Jobs;
                Some(Action::LoadJobs)
            }
            KeyCode::Char('5') if key.modifiers.is_empty() => {
                self.current_screen = CurrentScreen::Health;
                Some(Action::LoadHealth)
            }
            KeyCode::Char('6') if key.modifiers.is_empty() => {
                self.current_screen = CurrentScreen::SavedSearches;
                Some(Action::LoadSavedSearches)
            }
            KeyCode::Char('7') if key.modifiers.is_empty() => {
                self.current_screen = CurrentScreen::InternalLogs;
                Some(Action::LoadInternalLogs)
            }
            KeyCode::Char('8') if key.modifiers.is_empty() => {
                self.current_screen = CurrentScreen::Apps;
                Some(Action::LoadApps)
            }
            KeyCode::Char('9') if key.modifiers.is_empty() => {
                self.current_screen = CurrentScreen::Users;
                Some(Action::LoadUsers)
            }
            KeyCode::Char('0') if key.modifiers.is_empty() => {
                self.current_screen = CurrentScreen::Settings;
                None
            }
            KeyCode::Enter => {
                if !self.search_input.is_empty() {
                    let query = self.search_input.clone();
                    self.add_to_history(query.clone());
                    self.search_status = format!("Running: {}", query);
                    Some(Action::RunSearch(query))
                } else {
                    None
                }
            }
            KeyCode::Backspace => {
                self.history_index = None;
                self.search_input.pop();
                None
            }
            KeyCode::Down => {
                if let Some(curr) = self.history_index {
                    if curr > 0 {
                        self.history_index = Some(curr - 1);
                        self.search_input = self.search_history[curr - 1].clone();
                    } else {
                        self.history_index = None;
                        self.search_input = self.saved_search_input.clone();
                    }
                }
                None
            }
            KeyCode::Up => {
                if self.search_history.is_empty() {
                    return None;
                }

                if self.history_index.is_none() {
                    self.saved_search_input = self.search_input.clone();
                    self.history_index = Some(0);
                } else {
                    let curr = self.history_index.unwrap();
                    if curr < self.search_history.len().saturating_sub(1) {
                        self.history_index = Some(curr + 1);
                    }
                }

                if let Some(idx) = self.history_index {
                    self.search_input = self.search_history[idx].clone();
                }
                None
            }
            KeyCode::PageDown => Some(Action::PageDown),
            KeyCode::PageUp => Some(Action::PageUp),
            KeyCode::Home => Some(Action::GoToTop),
            KeyCode::End => Some(Action::GoToBottom),
            KeyCode::Char('?') if key.modifiers.is_empty() => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
                None
            }
            KeyCode::Char('e') if key.modifiers.is_empty() => {
                if !self.search_results.is_empty() {
                    self.export_input = "results.json".to_string();
                    self.export_format = ExportFormat::Json;
                    self.popup = Some(Popup::builder(PopupType::ExportSearch).build());
                    self.update_export_popup();
                }
                None
            }
            KeyCode::Char(c) => {
                self.history_index = None;
                self.search_input.push(c);
                None
            }
            _ => None,
        }
    }

    fn handle_jobs_input(&mut self, key: KeyEvent) -> Option<Action> {
        use crossterm::event::KeyCode;

        // Handle filter mode input
        if self.is_filtering {
            return match key.code {
                KeyCode::Esc => {
                    self.is_filtering = false;
                    self.filter_input.clear();
                    Some(Action::ClearSearch)
                }
                KeyCode::Enter => {
                    self.is_filtering = false;
                    if !self.filter_input.is_empty() {
                        self.search_filter = Some(self.filter_input.clone());
                        self.filter_input.clear();
                        self.rebuild_filtered_indices(); // Rebuild indices after filter is applied
                        None
                    } else {
                        Some(Action::ClearSearch) // Empty input clears the filter
                    }
                }
                KeyCode::Backspace => {
                    self.filter_input.pop();
                    None
                }
                KeyCode::Char(c) => {
                    self.filter_input.push(c);
                    None
                }
                _ => None,
            };
        }

        // Normal jobs screen input
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('1') => {
                self.current_screen = CurrentScreen::Search;
                None
            }
            KeyCode::Char('2') => {
                self.current_screen = CurrentScreen::Indexes;
                Some(Action::LoadIndexes)
            }
            KeyCode::Char('3') => {
                self.current_screen = CurrentScreen::Cluster;
                Some(Action::LoadClusterInfo)
            }
            KeyCode::Char('4') => {
                self.current_screen = CurrentScreen::Jobs;
                Some(Action::LoadJobs)
            }
            KeyCode::Char('5') => {
                self.current_screen = CurrentScreen::Health;
                Some(Action::LoadHealth)
            }
            KeyCode::Char('6') => {
                self.current_screen = CurrentScreen::SavedSearches;
                Some(Action::LoadSavedSearches)
            }
            KeyCode::Char('7') => {
                self.current_screen = CurrentScreen::InternalLogs;
                Some(Action::LoadInternalLogs)
            }
            KeyCode::Char('8') => {
                self.current_screen = CurrentScreen::Apps;
                Some(Action::LoadApps)
            }
            KeyCode::Char('9') => {
                self.current_screen = CurrentScreen::Users;
                Some(Action::LoadUsers)
            }
            KeyCode::Char('0') => {
                self.current_screen = CurrentScreen::Settings;
                None
            }
            KeyCode::Char('r') => Some(Action::LoadJobs),
            KeyCode::Char('a') => {
                self.auto_refresh = !self.auto_refresh;
                None
            }
            KeyCode::Char('j') => Some(Action::NavigateDown),
            KeyCode::Char('k') => Some(Action::NavigateUp),
            KeyCode::Down => Some(Action::NavigateDown),
            KeyCode::Up => Some(Action::NavigateUp),
            KeyCode::Char('c') => {
                if !self.selected_jobs.is_empty() {
                    self.popup = Some(
                        Popup::builder(PopupType::ConfirmCancelBatch(
                            self.selected_jobs.iter().cloned().collect(),
                        ))
                        .build(),
                    );
                } else if let Some(job) = self.get_selected_job() {
                    self.popup =
                        Some(Popup::builder(PopupType::ConfirmCancel(job.sid.clone())).build());
                }
                None
            }
            KeyCode::Char('d') => {
                if !self.selected_jobs.is_empty() {
                    self.popup = Some(
                        Popup::builder(PopupType::ConfirmDeleteBatch(
                            self.selected_jobs.iter().cloned().collect(),
                        ))
                        .build(),
                    );
                } else if let Some(job) = self.get_selected_job() {
                    self.popup =
                        Some(Popup::builder(PopupType::ConfirmDelete(job.sid.clone())).build());
                }
                None
            }
            KeyCode::Char('s') => Some(Action::CycleSortColumn),
            KeyCode::Char('/') => Some(Action::EnterSearchMode),
            KeyCode::Enter => Some(Action::InspectJob),
            KeyCode::Char('?') => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
                None
            }
            KeyCode::Char(' ') => {
                if let Some(job) = self.get_selected_job() {
                    let sid = job.sid.clone();
                    if self.selected_jobs.contains(&sid) {
                        self.selected_jobs.remove(&sid);
                    } else {
                        self.selected_jobs.insert(sid);
                    }
                }
                None
            }
            _ => None,
        }
    }

    fn handle_indexes_input(&mut self, key: KeyEvent) -> Option<Action> {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('1') => {
                self.current_screen = CurrentScreen::Search;
                None
            }
            KeyCode::Char('2') => {
                self.current_screen = CurrentScreen::Indexes;
                Some(Action::LoadIndexes)
            }
            KeyCode::Char('3') => {
                self.current_screen = CurrentScreen::Cluster;
                Some(Action::LoadClusterInfo)
            }
            KeyCode::Char('4') => {
                self.current_screen = CurrentScreen::Jobs;
                Some(Action::LoadJobs)
            }
            KeyCode::Char('5') => {
                self.current_screen = CurrentScreen::Health;
                Some(Action::LoadHealth)
            }
            KeyCode::Char('6') => {
                self.current_screen = CurrentScreen::SavedSearches;
                Some(Action::LoadSavedSearches)
            }
            KeyCode::Char('7') => {
                self.current_screen = CurrentScreen::InternalLogs;
                Some(Action::LoadInternalLogs)
            }
            KeyCode::Char('8') => {
                self.current_screen = CurrentScreen::Apps;
                Some(Action::LoadApps)
            }
            KeyCode::Char('9') => {
                self.current_screen = CurrentScreen::Users;
                Some(Action::LoadUsers)
            }
            KeyCode::Char('0') => {
                self.current_screen = CurrentScreen::Settings;
                None
            }
            KeyCode::Char('r') => Some(Action::LoadIndexes),
            KeyCode::Char('j') => Some(Action::NavigateDown),
            KeyCode::Char('k') => Some(Action::NavigateUp),
            KeyCode::Down => Some(Action::NavigateDown),
            KeyCode::Up => Some(Action::NavigateUp),
            KeyCode::Char('?') => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
                None
            }
            _ => None,
        }
    }

    fn handle_cluster_input(&mut self, key: KeyEvent) -> Option<Action> {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('1') => {
                self.current_screen = CurrentScreen::Search;
                None
            }
            KeyCode::Char('2') => {
                self.current_screen = CurrentScreen::Indexes;
                Some(Action::LoadIndexes)
            }
            KeyCode::Char('3') => {
                self.current_screen = CurrentScreen::Cluster;
                Some(Action::LoadClusterInfo)
            }
            KeyCode::Char('4') => {
                self.current_screen = CurrentScreen::Jobs;
                Some(Action::LoadJobs)
            }
            KeyCode::Char('5') => {
                self.current_screen = CurrentScreen::Health;
                Some(Action::LoadHealth)
            }
            KeyCode::Char('6') => {
                self.current_screen = CurrentScreen::SavedSearches;
                Some(Action::LoadSavedSearches)
            }
            KeyCode::Char('7') => {
                self.current_screen = CurrentScreen::InternalLogs;
                Some(Action::LoadInternalLogs)
            }
            KeyCode::Char('8') => {
                self.current_screen = CurrentScreen::Apps;
                Some(Action::LoadApps)
            }
            KeyCode::Char('9') => {
                self.current_screen = CurrentScreen::Users;
                Some(Action::LoadUsers)
            }
            KeyCode::Char('0') => {
                self.current_screen = CurrentScreen::Settings;
                None
            }
            KeyCode::Char('r') => Some(Action::LoadClusterInfo),
            KeyCode::Char('?') => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
                None
            }
            _ => None,
        }
    }

    fn handle_job_inspect_input(&mut self, key: KeyEvent) -> Option<Action> {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Esc => Some(Action::ExitInspectMode),
            KeyCode::Char('0') => {
                self.current_screen = CurrentScreen::Settings;
                None
            }
            KeyCode::Char('?') => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
                None
            }
            _ => None,
        }
    }

    fn handle_health_input(&mut self, key: KeyEvent) -> Option<Action> {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('1') => {
                self.current_screen = CurrentScreen::Search;
                None
            }
            KeyCode::Char('2') => {
                self.current_screen = CurrentScreen::Indexes;
                Some(Action::LoadIndexes)
            }
            KeyCode::Char('3') => {
                self.current_screen = CurrentScreen::Cluster;
                Some(Action::LoadClusterInfo)
            }
            KeyCode::Char('4') => {
                self.current_screen = CurrentScreen::Jobs;
                Some(Action::LoadJobs)
            }
            KeyCode::Char('5') => {
                self.current_screen = CurrentScreen::Health;
                Some(Action::LoadHealth)
            }
            KeyCode::Char('6') => {
                self.current_screen = CurrentScreen::SavedSearches;
                Some(Action::LoadSavedSearches)
            }
            KeyCode::Char('7') => {
                self.current_screen = CurrentScreen::InternalLogs;
                Some(Action::LoadInternalLogs)
            }
            KeyCode::Char('8') => {
                self.current_screen = CurrentScreen::Apps;
                Some(Action::LoadApps)
            }
            KeyCode::Char('9') => {
                self.current_screen = CurrentScreen::Users;
                Some(Action::LoadUsers)
            }
            KeyCode::Char('r') => Some(Action::LoadHealth),
            KeyCode::Char('?') => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
                None
            }
            _ => None,
        }
    }

    fn handle_saved_searches_input(&mut self, key: KeyEvent) -> Option<Action> {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('1') => {
                self.current_screen = CurrentScreen::Search;
                None
            }
            KeyCode::Char('2') => {
                self.current_screen = CurrentScreen::Indexes;
                Some(Action::LoadIndexes)
            }
            KeyCode::Char('3') => {
                self.current_screen = CurrentScreen::Cluster;
                Some(Action::LoadClusterInfo)
            }
            KeyCode::Char('4') => {
                self.current_screen = CurrentScreen::Jobs;
                Some(Action::LoadJobs)
            }
            KeyCode::Char('5') => {
                self.current_screen = CurrentScreen::Health;
                Some(Action::LoadHealth)
            }
            KeyCode::Char('6') => {
                self.current_screen = CurrentScreen::SavedSearches;
                Some(Action::LoadSavedSearches)
            }
            KeyCode::Char('7') => {
                self.current_screen = CurrentScreen::InternalLogs;
                Some(Action::LoadInternalLogs)
            }
            KeyCode::Char('8') => {
                self.current_screen = CurrentScreen::Apps;
                Some(Action::LoadApps)
            }
            KeyCode::Char('9') => {
                self.current_screen = CurrentScreen::Users;
                Some(Action::LoadUsers)
            }
            KeyCode::Char('0') => {
                self.current_screen = CurrentScreen::Settings;
                None
            }
            KeyCode::Char('r') => Some(Action::LoadSavedSearches),
            KeyCode::Char('j') | KeyCode::Down => Some(Action::NavigateDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Action::NavigateUp),
            KeyCode::Enter => {
                let query = self.saved_searches.as_ref().and_then(|searches| {
                    self.saved_searches_state.selected().and_then(|selected| {
                        searches.get(selected).map(|search| search.search.clone())
                    })
                });

                if let Some(query) = query {
                    self.search_input = query.clone();
                    self.current_screen = CurrentScreen::Search;
                    self.add_to_history(query.clone());
                    self.search_status = format!("Running: {}", query);
                    return Some(Action::RunSearch(query));
                }
                None
            }
            KeyCode::Char('?') => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
                None
            }
            _ => None,
        }
    }

    fn handle_internal_logs_input(&mut self, key: KeyEvent) -> Option<Action> {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('1') => {
                self.current_screen = CurrentScreen::Search;
                None
            }
            KeyCode::Char('2') => {
                self.current_screen = CurrentScreen::Indexes;
                Some(Action::LoadIndexes)
            }
            KeyCode::Char('3') => {
                self.current_screen = CurrentScreen::Cluster;
                Some(Action::LoadClusterInfo)
            }
            KeyCode::Char('4') => {
                self.current_screen = CurrentScreen::Jobs;
                Some(Action::LoadJobs)
            }
            KeyCode::Char('5') => {
                self.current_screen = CurrentScreen::Health;
                Some(Action::LoadHealth)
            }
            KeyCode::Char('6') => {
                self.current_screen = CurrentScreen::SavedSearches;
                Some(Action::LoadSavedSearches)
            }
            KeyCode::Char('7') => {
                self.current_screen = CurrentScreen::InternalLogs;
                Some(Action::LoadInternalLogs)
            }
            KeyCode::Char('8') => {
                self.current_screen = CurrentScreen::Apps;
                Some(Action::LoadApps)
            }
            KeyCode::Char('9') => {
                self.current_screen = CurrentScreen::Users;
                Some(Action::LoadUsers)
            }
            KeyCode::Char('0') => {
                self.current_screen = CurrentScreen::Settings;
                None
            }
            KeyCode::Char('r') => Some(Action::LoadInternalLogs),
            KeyCode::Char('a') => {
                self.auto_refresh = !self.auto_refresh;
                None
            }
            KeyCode::Char('j') | KeyCode::Down => Some(Action::NavigateDown),
            KeyCode::Char('k') | KeyCode::Up => Some(Action::NavigateUp),
            KeyCode::Char('?') => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
                None
            }
            _ => None,
        }
    }

    fn handle_apps_input(&mut self, key: KeyEvent) -> Option<Action> {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('1') => {
                self.current_screen = CurrentScreen::Search;
                None
            }
            KeyCode::Char('2') => {
                self.current_screen = CurrentScreen::Indexes;
                Some(Action::LoadIndexes)
            }
            KeyCode::Char('3') => {
                self.current_screen = CurrentScreen::Cluster;
                Some(Action::LoadClusterInfo)
            }
            KeyCode::Char('4') => {
                self.current_screen = CurrentScreen::Jobs;
                Some(Action::LoadJobs)
            }
            KeyCode::Char('5') => {
                self.current_screen = CurrentScreen::Health;
                Some(Action::LoadHealth)
            }
            KeyCode::Char('6') => {
                self.current_screen = CurrentScreen::SavedSearches;
                Some(Action::LoadSavedSearches)
            }
            KeyCode::Char('7') => {
                self.current_screen = CurrentScreen::InternalLogs;
                Some(Action::LoadInternalLogs)
            }
            KeyCode::Char('8') => {
                self.current_screen = CurrentScreen::Apps;
                Some(Action::LoadApps)
            }
            KeyCode::Char('9') => {
                self.current_screen = CurrentScreen::Users;
                Some(Action::LoadUsers)
            }
            KeyCode::Char('0') => {
                self.current_screen = CurrentScreen::Settings;
                None
            }
            KeyCode::Char('r') => Some(Action::LoadApps),
            KeyCode::Char('j') => Some(Action::NavigateDown),
            KeyCode::Char('k') => Some(Action::NavigateUp),
            KeyCode::Down => Some(Action::NavigateDown),
            KeyCode::Up => Some(Action::NavigateUp),
            KeyCode::Char('?') => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
                None
            }
            _ => None,
        }
    }

    fn handle_users_input(&mut self, key: KeyEvent) -> Option<Action> {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('1') => {
                self.current_screen = CurrentScreen::Search;
                None
            }
            KeyCode::Char('2') => {
                self.current_screen = CurrentScreen::Indexes;
                Some(Action::LoadIndexes)
            }
            KeyCode::Char('3') => {
                self.current_screen = CurrentScreen::Cluster;
                Some(Action::LoadClusterInfo)
            }
            KeyCode::Char('4') => {
                self.current_screen = CurrentScreen::Jobs;
                Some(Action::LoadJobs)
            }
            KeyCode::Char('5') => {
                self.current_screen = CurrentScreen::Health;
                Some(Action::LoadHealth)
            }
            KeyCode::Char('6') => {
                self.current_screen = CurrentScreen::SavedSearches;
                Some(Action::LoadSavedSearches)
            }
            KeyCode::Char('7') => {
                self.current_screen = CurrentScreen::InternalLogs;
                Some(Action::LoadInternalLogs)
            }
            KeyCode::Char('8') => {
                self.current_screen = CurrentScreen::Apps;
                Some(Action::LoadApps)
            }
            KeyCode::Char('9') => {
                self.current_screen = CurrentScreen::Users;
                Some(Action::LoadUsers)
            }
            KeyCode::Char('0') => {
                self.current_screen = CurrentScreen::Settings;
                None
            }
            KeyCode::Char('r') => Some(Action::LoadUsers),
            KeyCode::Char('j') => Some(Action::NavigateDown),
            KeyCode::Char('k') => Some(Action::NavigateUp),
            KeyCode::Down => Some(Action::NavigateDown),
            KeyCode::Up => Some(Action::NavigateUp),
            KeyCode::Char('?') => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
                None
            }
            _ => None,
        }
    }

    fn handle_settings_input(&mut self, key: KeyEvent) -> Option<Action> {
        use crossterm::event::KeyCode;
        match key.code {
            KeyCode::Char('q') => Some(Action::Quit),
            KeyCode::Char('1') => {
                self.current_screen = CurrentScreen::Search;
                None
            }
            KeyCode::Char('2') => {
                self.current_screen = CurrentScreen::Indexes;
                Some(Action::LoadIndexes)
            }
            KeyCode::Char('3') => {
                self.current_screen = CurrentScreen::Cluster;
                Some(Action::LoadClusterInfo)
            }
            KeyCode::Char('4') => {
                self.current_screen = CurrentScreen::Jobs;
                Some(Action::LoadJobs)
            }
            KeyCode::Char('5') => {
                self.current_screen = CurrentScreen::Health;
                Some(Action::LoadHealth)
            }
            KeyCode::Char('6') => {
                self.current_screen = CurrentScreen::SavedSearches;
                Some(Action::LoadSavedSearches)
            }
            KeyCode::Char('7') => {
                self.current_screen = CurrentScreen::InternalLogs;
                Some(Action::LoadInternalLogs)
            }
            KeyCode::Char('8') => {
                self.current_screen = CurrentScreen::Apps;
                Some(Action::LoadApps)
            }
            KeyCode::Char('9') => {
                self.current_screen = CurrentScreen::Users;
                Some(Action::LoadUsers)
            }
            KeyCode::Char('0') => {
                self.current_screen = CurrentScreen::Settings;
                None
            }
            KeyCode::Char('a') => {
                self.auto_refresh = !self.auto_refresh;
                self.toasts.push(Toast::info(format!(
                    "Auto-refresh: {}",
                    if self.auto_refresh { "On" } else { "Off" }
                )));
                None
            }
            KeyCode::Char('s') => {
                self.sort_state.column = match self.sort_state.column {
                    SortColumn::Sid => SortColumn::Status,
                    SortColumn::Status => SortColumn::Duration,
                    SortColumn::Duration => SortColumn::Results,
                    SortColumn::Results => SortColumn::Events,
                    SortColumn::Events => SortColumn::Sid,
                };
                self.toasts.push(Toast::info(format!(
                    "Sort column: {}",
                    self.sort_state.column.as_str()
                )));
                None
            }
            KeyCode::Char('d') => {
                self.sort_state.direction = match self.sort_state.direction {
                    SortDirection::Asc => SortDirection::Desc,
                    SortDirection::Desc => SortDirection::Asc,
                };
                self.toasts.push(Toast::info(format!(
                    "Sort direction: {}",
                    self.sort_state.direction.as_str()
                )));
                None
            }
            KeyCode::Char('c') => {
                self.search_history.clear();
                self.toasts.push(Toast::info("Search history cleared"));
                None
            }
            KeyCode::Char('r') => Some(Action::SwitchToSettings),
            KeyCode::Char('?') => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
                None
            }
            _ => None,
        }
    }

    /// Pure state mutation based on Action.
    pub fn update(&mut self, action: Action) {
        match action {
            Action::NavigateDown => self.next_item(),
            Action::NavigateUp => self.previous_item(),
            Action::PageDown => self.next_page(),
            Action::PageUp => self.previous_page(),
            Action::GoToTop => self.go_to_top(),
            Action::GoToBottom => self.go_to_bottom(),
            Action::EnterSearchMode => {
                self.is_filtering = true;
                self.filter_input.clear();
            }
            Action::SearchInput(c) => {
                self.filter_input.push(c);
            }
            Action::ClearSearch => {
                self.search_filter = None;
                self.rebuild_filtered_indices();
            }
            Action::CycleSortColumn => {
                self.sort_state.cycle();
                self.rebuild_filtered_indices();
            }
            Action::ToggleSortDirection => {
                self.sort_state.toggle_direction();
                self.rebuild_filtered_indices();
            }
            Action::Loading(is_loading) => {
                self.loading = is_loading;
                if is_loading {
                    self.progress = 0.0;
                }
            }
            Action::Progress(p) => {
                self.progress = p;
            }
            Action::Notify(level, message) => {
                self.toasts.push(Toast::new(message, level));
            }
            Action::Tick => {
                // Prune expired toasts
                self.toasts.retain(|t| !t.is_expired());
            }
            Action::IndexesLoaded(Ok(indexes)) => {
                self.indexes = Some(indexes);
                self.loading = false;
            }
            Action::JobsLoaded(Ok(jobs)) => {
                let sel = self.jobs_state.selected();
                self.jobs = Some(jobs);
                self.loading = false;
                // Rebuild filtered indices and restore selection clamped to new bounds
                self.rebuild_filtered_indices();
                let filtered_len = self.filtered_jobs_len();
                self.jobs_state.select(
                    sel.map(|i| i.min(filtered_len.saturating_sub(1)))
                        .or(Some(0)),
                );
            }
            Action::SavedSearchesLoaded(Ok(searches)) => {
                self.saved_searches = Some(searches);
                self.loading = false;
            }
            Action::InternalLogsLoaded(Ok(logs)) => {
                let sel = self.internal_logs_state.selected();
                self.internal_logs = Some(logs);
                self.loading = false;
                if let Some(logs) = &self.internal_logs {
                    self.internal_logs_state
                        .select(sel.map(|i| i.min(logs.len().saturating_sub(1))).or(Some(0)));
                }
            }
            Action::ClusterInfoLoaded(Ok(info)) => {
                self.cluster_info = Some(info);
                self.loading = false;
            }
            Action::HealthLoaded(boxed_result) => match *boxed_result {
                Ok(ref info) => {
                    self.health_info = Some(info.clone());
                    // Update health state from splunkd_health if available
                    if let Some(ref health) = info.splunkd_health {
                        let new_state = HealthState::from_health_str(&health.health);
                        self.set_health_state(new_state);
                    }
                    self.loading = false;
                }
                Err(e) => {
                    self.toasts
                        .push(Toast::error(format!("Failed to load health info: {}", e)));
                    self.loading = false;
                }
            },
            Action::HealthStatusLoaded(result) => match result {
                Ok(health) => {
                    let new_state = HealthState::from_health_str(&health.health);
                    self.set_health_state(new_state);
                }
                Err(_) => {
                    // Error getting health - mark as unhealthy
                    self.set_health_state(HealthState::Unhealthy);
                }
            },
            Action::SearchComplete(Ok((results, sid, total))) => {
                let results_count = results.len() as u64;
                self.set_search_results(results);
                self.search_sid = Some(sid);
                // Set pagination state from initial search results
                self.search_results_total_count = total;
                self.search_has_more_results = if let Some(t) = total {
                    results_count < t
                } else {
                    // When total is None, infer from page fullness
                    // Note: initial fetch in main.rs uses 1000, but we use app's page_size for consistency
                    results_count >= self.search_results_page_size
                };
                self.search_status = format!("Search complete: {}", self.search_input);
                self.loading = false;
            }
            Action::MoreSearchResultsLoaded(Ok((results, _offset, total))) => {
                self.append_search_results(results, total);
                self.loading = false;
            }
            Action::MoreSearchResultsLoaded(Err(e)) => {
                self.toasts
                    .push(Toast::error(format!("Failed to load more results: {}", e)));
                self.loading = false;
            }
            Action::JobOperationComplete(msg) => {
                self.selected_jobs.clear();
                self.search_status = msg;
                self.loading = false;
            }
            Action::IndexesLoaded(Err(e)) => {
                self.toasts
                    .push(Toast::error(format!("Failed to load indexes: {}", e)));
                self.loading = false;
            }
            Action::JobsLoaded(Err(e)) => {
                self.toasts
                    .push(Toast::error(format!("Failed to load jobs: {}", e)));
                self.loading = false;
            }
            Action::SavedSearchesLoaded(Err(e)) => {
                self.toasts.push(Toast::error(format!(
                    "Failed to load saved searches: {}",
                    e
                )));
                self.loading = false;
            }
            Action::InternalLogsLoaded(Err(e)) => {
                self.toasts
                    .push(Toast::error(format!("Failed to load internal logs: {}", e)));
                self.loading = false;
            }
            Action::AppsLoaded(Ok(apps)) => {
                self.apps = Some(apps);
                self.loading = false;
            }
            Action::AppsLoaded(Err(e)) => {
                self.toasts
                    .push(Toast::error(format!("Failed to load apps: {}", e)));
                self.loading = false;
            }
            Action::UsersLoaded(Ok(users)) => {
                self.users = Some(users);
                self.loading = false;
            }
            Action::UsersLoaded(Err(e)) => {
                self.toasts
                    .push(Toast::error(format!("Failed to load users: {}", e)));
                self.loading = false;
            }
            Action::SettingsLoaded(state) => {
                self.auto_refresh = state.auto_refresh;
                self.sort_state.column = parse_sort_column(&state.sort_column);
                self.sort_state.direction = parse_sort_direction(&state.sort_direction);
                self.search_history = state.search_history;
                if let Some(query) = state.last_search_query {
                    self.search_input = query;
                }
                self.toasts.push(Toast::info("Settings loaded from file"));
                self.loading = false;
            }
            Action::ClusterInfoLoaded(Err(e)) => {
                self.toasts
                    .push(Toast::error(format!("Failed to load cluster info: {}", e)));
                self.loading = false;
            }
            Action::SearchComplete(Err(e)) => {
                let details = crate::error_details::ErrorDetails::from_error_string(&e);
                self.current_error = Some(details.clone());
                self.toasts.push(Toast::error(details.to_summary()));
                self.loading = false;
            }
            Action::ShowErrorDetails(details) => {
                self.current_error = Some(details);
                self.popup = Some(Popup::builder(PopupType::ErrorDetails).build());
            }
            Action::ClearErrorDetails => {
                self.current_error = None;
                self.popup = None;
            }
            Action::InspectJob => {
                // Transition to job inspect screen if we have jobs and a selection
                if self.jobs.as_ref().map(|j| !j.is_empty()).unwrap_or(false)
                    && self.jobs_state.selected().is_some()
                {
                    self.current_screen = CurrentScreen::JobInspect;
                }
            }
            Action::ExitInspectMode => {
                // Return to jobs screen
                self.current_screen = CurrentScreen::Jobs;
            }
            _ => {}
        }
    }

    // Navigation helpers
    fn next_item(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                if !self.search_results.is_empty() {
                    let max_offset = self.search_results.len().saturating_sub(1);
                    if self.search_scroll_offset < max_offset {
                        self.search_scroll_offset += 1;
                    }
                }
            }
            CurrentScreen::Jobs => {
                let len = self.filtered_jobs_len();
                if len > 0 {
                    let i = self.jobs_state.selected().unwrap_or(0);
                    if i < len.saturating_sub(1) {
                        self.jobs_state.select(Some(i + 1));
                    }
                }
            }
            CurrentScreen::Indexes => {
                if let Some(indexes) = &self.indexes {
                    let i = self.indexes_state.selected().unwrap_or(0);
                    if i < indexes.len().saturating_sub(1) {
                        self.indexes_state.select(Some(i + 1));
                    }
                }
            }
            CurrentScreen::SavedSearches => {
                if let Some(searches) = &self.saved_searches {
                    let i = self.saved_searches_state.selected().unwrap_or(0);
                    if i < searches.len().saturating_sub(1) {
                        self.saved_searches_state.select(Some(i + 1));
                    }
                }
            }
            CurrentScreen::InternalLogs => {
                if let Some(logs) = &self.internal_logs {
                    let i = self.internal_logs_state.selected().unwrap_or(0);
                    if i < logs.len().saturating_sub(1) {
                        self.internal_logs_state.select(Some(i + 1));
                    }
                }
            }
            CurrentScreen::Apps => {
                if let Some(apps) = &self.apps {
                    let i = self.apps_state.selected().unwrap_or(0);
                    if i < apps.len().saturating_sub(1) {
                        self.apps_state.select(Some(i + 1));
                    }
                }
            }
            CurrentScreen::Users => {
                if let Some(users) = &self.users {
                    let i = self.users_state.selected().unwrap_or(0);
                    if i < users.len().saturating_sub(1) {
                        self.users_state.select(Some(i + 1));
                    }
                }
            }
            _ => {}
        }
    }

    fn previous_item(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                self.search_scroll_offset = self.search_scroll_offset.saturating_sub(1);
            }
            CurrentScreen::Jobs => {
                let i = self.jobs_state.selected().unwrap_or(0);
                if i > 0 {
                    self.jobs_state.select(Some(i - 1));
                }
            }
            CurrentScreen::Indexes => {
                let i = self.indexes_state.selected().unwrap_or(0);
                if i > 0 {
                    self.indexes_state.select(Some(i - 1));
                }
            }
            CurrentScreen::SavedSearches => {
                let i = self.saved_searches_state.selected().unwrap_or(0);
                if i > 0 {
                    self.saved_searches_state.select(Some(i - 1));
                }
            }
            CurrentScreen::InternalLogs => {
                let i = self.internal_logs_state.selected().unwrap_or(0);
                if i > 0 {
                    self.internal_logs_state.select(Some(i - 1));
                }
            }
            CurrentScreen::Apps => {
                let i = self.apps_state.selected().unwrap_or(0);
                if i > 0 {
                    self.apps_state.select(Some(i - 1));
                }
            }
            CurrentScreen::Users => {
                let i = self.users_state.selected().unwrap_or(0);
                if i > 0 {
                    self.users_state.select(Some(i - 1));
                }
            }
            _ => {}
        }
    }

    fn next_page(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                // Clamp offset to prevent scrolling past the end
                let max_offset = self.search_results.len().saturating_sub(1);
                self.search_scroll_offset =
                    self.search_scroll_offset.saturating_add(10).min(max_offset);
            }
            CurrentScreen::Jobs => {
                let len = self.filtered_jobs_len();
                if len > 0 {
                    let i = self.jobs_state.selected().unwrap_or(0);
                    self.jobs_state
                        .select(Some((i.saturating_add(10)).min(len - 1)));
                }
            }
            CurrentScreen::Indexes => {
                if let Some(indexes) = &self.indexes {
                    let i = self.indexes_state.selected().unwrap_or(0);
                    self.indexes_state
                        .select(Some((i.saturating_add(10)).min(indexes.len() - 1)));
                }
            }
            CurrentScreen::SavedSearches => {
                if let Some(searches) = &self.saved_searches {
                    let i = self.saved_searches_state.selected().unwrap_or(0);
                    self.saved_searches_state
                        .select(Some((i.saturating_add(10)).min(searches.len() - 1)));
                }
            }
            CurrentScreen::InternalLogs => {
                if let Some(logs) = &self.internal_logs {
                    let i = self.internal_logs_state.selected().unwrap_or(0);
                    self.internal_logs_state
                        .select(Some((i.saturating_add(10)).min(logs.len() - 1)));
                }
            }
            CurrentScreen::Apps => {
                if let Some(apps) = &self.apps {
                    let i = self.apps_state.selected().unwrap_or(0);
                    self.apps_state
                        .select(Some((i.saturating_add(10)).min(apps.len() - 1)));
                }
            }
            _ => {}
        }
    }

    fn previous_page(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                // saturating_sub already prevents going below 0
                self.search_scroll_offset = self.search_scroll_offset.saturating_sub(10);
            }
            CurrentScreen::Jobs => {
                let i = self.jobs_state.selected().unwrap_or(0);
                self.jobs_state.select(Some(i.saturating_sub(10)));
            }
            CurrentScreen::Indexes => {
                let i = self.indexes_state.selected().unwrap_or(0);
                self.indexes_state.select(Some(i.saturating_sub(10)));
            }
            CurrentScreen::SavedSearches => {
                let i = self.saved_searches_state.selected().unwrap_or(0);
                self.saved_searches_state.select(Some(i.saturating_sub(10)));
            }
            CurrentScreen::InternalLogs => {
                let i = self.internal_logs_state.selected().unwrap_or(0);
                self.internal_logs_state.select(Some(i.saturating_sub(10)));
            }
            CurrentScreen::Apps => {
                let i = self.apps_state.selected().unwrap_or(0);
                self.apps_state.select(Some(i.saturating_sub(10)));
            }
            _ => {}
        }
    }

    fn go_to_top(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                self.search_scroll_offset = 0;
            }
            CurrentScreen::Jobs => {
                if self.filtered_jobs_len() > 0 {
                    self.jobs_state.select(Some(0));
                }
            }
            CurrentScreen::Indexes => {
                self.indexes_state.select(Some(0));
            }
            CurrentScreen::SavedSearches => {
                self.saved_searches_state.select(Some(0));
            }
            CurrentScreen::InternalLogs => {
                self.internal_logs_state.select(Some(0));
            }
            CurrentScreen::Apps => {
                self.apps_state.select(Some(0));
            }
            CurrentScreen::Users => {
                self.users_state.select(Some(0));
            }
            _ => {}
        }
    }

    fn go_to_bottom(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                // Scroll to the last valid page (offset such that at least one result is visible)
                if !self.search_results.is_empty() {
                    self.search_scroll_offset = self.search_results.len().saturating_sub(1);
                } else {
                    self.search_scroll_offset = 0;
                }
            }
            CurrentScreen::Jobs => {
                let len = self.filtered_jobs_len();
                if len > 0 {
                    self.jobs_state.select(Some(len.saturating_sub(1)));
                }
            }
            CurrentScreen::Indexes => {
                if let Some(indexes) = &self.indexes {
                    self.indexes_state
                        .select(Some(indexes.len().saturating_sub(1)));
                }
            }
            CurrentScreen::SavedSearches => {
                if let Some(searches) = &self.saved_searches {
                    self.saved_searches_state
                        .select(Some(searches.len().saturating_sub(1)));
                }
            }
            CurrentScreen::InternalLogs => {
                if let Some(logs) = &self.internal_logs {
                    self.internal_logs_state
                        .select(Some(logs.len().saturating_sub(1)));
                }
            }
            CurrentScreen::Apps => {
                if let Some(apps) = &self.apps {
                    self.apps_state.select(Some(apps.len().saturating_sub(1)));
                }
            }
            CurrentScreen::Users => {
                if let Some(users) = &self.users {
                    self.users_state.select(Some(users.len().saturating_sub(1)));
                }
            }
            _ => {}
        }
    }

    /// Rebuild the filtered job indices based on the current filter and jobs.
    /// The indices are sorted according to the current sort settings.
    fn rebuild_filtered_indices(&mut self) {
        let Some(jobs) = &self.jobs else {
            self.filtered_job_indices.clear();
            return;
        };

        // First filter the jobs
        let mut filtered_and_sorted: Vec<usize> = if let Some(filter) = &self.search_filter {
            let lower_filter = filter.to_lowercase();
            jobs.iter()
                .enumerate()
                .filter(|(_, job)| {
                    job.sid.to_lowercase().contains(&lower_filter)
                        || (job.is_done && "done".contains(&lower_filter))
                        || (!job.is_done && "running".contains(&lower_filter))
                })
                .map(|(i, _)| i)
                .collect()
        } else {
            // No filter: all jobs are visible
            (0..jobs.len()).collect()
        };

        // Then sort the filtered indices using the same comparison logic as jobs.rs
        filtered_and_sorted.sort_by(|&a, &b| {
            let job_a = &jobs[a];
            let job_b = &jobs[b];
            self.compare_jobs_for_sort(job_a, job_b)
        });

        self.filtered_job_indices = filtered_and_sorted;

        // Clamp selection to filtered list length
        let filtered_len = self.filtered_job_indices.len();
        if let Some(selected) = self.jobs_state.selected() {
            if filtered_len == 0 {
                self.jobs_state.select(None);
            } else if selected >= filtered_len {
                self.jobs_state.select(Some(filtered_len - 1));
            }
        }
    }

    /// Compare two jobs for sorting based on current sort settings.
    /// Matches the logic in jobs.rs::compare_jobs.
    fn compare_jobs_for_sort(
        &self,
        a: &SearchJobStatus,
        b: &SearchJobStatus,
    ) -> std::cmp::Ordering {
        let ordering = match self.sort_state.column {
            SortColumn::Sid => a.sid.cmp(&b.sid),
            SortColumn::Status => {
                // Sort by is_done first, then by progress
                match (a.is_done, b.is_done) {
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    _ => a
                        .done_progress
                        .partial_cmp(&b.done_progress)
                        .unwrap_or(std::cmp::Ordering::Equal),
                }
            }
            SortColumn::Duration => a
                .run_duration
                .partial_cmp(&b.run_duration)
                .unwrap_or(std::cmp::Ordering::Equal),
            SortColumn::Results => a.result_count.cmp(&b.result_count),
            SortColumn::Events => a.event_count.cmp(&b.event_count),
        };

        match self.sort_state.direction {
            SortDirection::Asc => ordering,
            SortDirection::Desc => ordering.reverse(),
        }
    }

    /// Get the currently selected job, accounting for any active filter.
    pub fn get_selected_job(&self) -> Option<&SearchJobStatus> {
        let selected = self.jobs_state.selected()?;
        let original_idx = self.filtered_job_indices.get(selected)?;
        self.jobs.as_ref()?.get(*original_idx)
    }

    /// Get the filtered and sorted list of jobs (references into the original list).
    /// NOTE: Currently unused directly as render_jobs accesses filtered_job_indices.
    /// Kept for potential future use or testing.
    #[allow(dead_code)]
    pub fn get_filtered_jobs(&self) -> Vec<&SearchJobStatus> {
        let Some(jobs) = &self.jobs else {
            return Vec::new();
        };
        self.filtered_job_indices
            .iter()
            .filter_map(|&i| jobs.get(i))
            .collect()
    }

    /// Get the length of the filtered jobs list.
    fn filtered_jobs_len(&self) -> usize {
        self.filtered_job_indices.len()
    }

    pub fn render(&mut self, f: &mut Frame) {
        self.last_area = f.area();

        // Create layout
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(HEADER_HEIGHT),
                    Constraint::Min(0),
                    Constraint::Length(FOOTER_HEIGHT),
                ]
                .as_ref(),
            )
            .split(f.area());

        // Header
        // Build health indicator span
        let health_indicator = match self.health_state {
            HealthState::Healthy => Span::styled("[+]", Style::default().fg(Color::Green)),
            HealthState::Unhealthy => Span::styled("[!]", Style::default().fg(Color::Red)),
            HealthState::Unknown => Span::styled("[?]", Style::default().fg(Color::Yellow)),
        };

        let health_label = match self.health_state {
            HealthState::Healthy => "Healthy",
            HealthState::Unhealthy => "Unhealthy",
            HealthState::Unknown => "Unknown",
        };

        let header = Paragraph::new(vec![Line::from(vec![
            Span::styled(
                "Splunk TUI",
                Style::default()
                    .fg(Color::Cyan)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" - "),
            Span::styled(
                match self.current_screen {
                    CurrentScreen::Search => "Search",
                    CurrentScreen::Indexes => "Indexes",
                    CurrentScreen::Cluster => "Cluster",
                    CurrentScreen::Jobs => "Jobs",
                    CurrentScreen::JobInspect => "Job Details",
                    CurrentScreen::Health => "Health",
                    CurrentScreen::SavedSearches => "Saved Searches",
                    CurrentScreen::InternalLogs => "Internal Logs",
                    CurrentScreen::Apps => "Apps",
                    CurrentScreen::Users => "Users",
                    CurrentScreen::Settings => "Settings",
                },
                Style::default().fg(Color::Yellow),
            ),
            Span::raw(" | "),
            health_indicator,
            Span::raw(" "),
            Span::styled(
                health_label,
                match self.health_state {
                    HealthState::Healthy => Style::default().fg(Color::Green),
                    HealthState::Unhealthy => Style::default().fg(Color::Red),
                    HealthState::Unknown => Style::default().fg(Color::Yellow),
                },
            ),
        ])])
        .block(Block::default().borders(Borders::ALL));
        f.render_widget(header, chunks[0]);

        // Main content
        self.render_content(f, chunks[1]);

        // Footer with status
        let footer_text = if self.loading {
            vec![Line::from(vec![
                Span::styled(
                    format!(" Loading... {:.0}% ", self.progress * 100.0),
                    Style::default().fg(Color::Yellow),
                ),
                Span::raw("|"),
                Span::raw(
                    " 1:Search 2:Indexes 3:Cluster 4:Jobs 5:Health 6:Saved 7:Logs 8:Apps 9:Users 0:Settings ",
                ),
                Span::raw("|"),
                Span::styled(" q:Quit ", Style::default().fg(Color::Red)),
            ])]
        } else {
            vec![Line::from(vec![
                Span::raw(
                    " 1:Search 2:Indexes 3:Cluster 4:Jobs 5:Health 6:Saved 7:Logs 8:Apps 9:Users 0:Settings ",
                ),
                Span::raw("|"),
                Span::styled(" q:Quit ", Style::default().fg(Color::Red)),
            ])]
        };
        let footer = Paragraph::new(footer_text).block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);

        // Render toasts
        crate::ui::toast::render_toasts(f, &self.toasts, self.current_error.is_some());

        // Render popup if active (on top of toasts)
        if let Some(ref popup) = self.popup {
            crate::ui::popup::render_popup(f, popup);
        }

        // Render error details popup if active
        if let Some(Popup {
            kind: PopupType::ErrorDetails,
            ..
        }) = &self.popup
            && let Some(error) = &self.current_error
        {
            crate::ui::error_details::render_error_details(f, error, self);
        }
    }

    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        match self.current_screen {
            CurrentScreen::Search => {
                search::render_search(
                    f,
                    area,
                    search::SearchRenderConfig {
                        search_input: &self.search_input,
                        search_status: &self.search_status,
                        loading: self.loading,
                        progress: self.progress,
                        search_results: &self.search_results,
                        search_scroll_offset: self.search_scroll_offset,
                        search_results_total_count: self.search_results_total_count,
                        search_has_more_results: self.search_has_more_results,
                    },
                );
            }
            CurrentScreen::Indexes => {
                indexes::render_indexes(
                    f,
                    area,
                    indexes::IndexesRenderConfig {
                        loading: self.loading,
                        indexes: self.indexes.as_deref(),
                        state: &mut self.indexes_state,
                    },
                );
            }
            CurrentScreen::Cluster => {
                cluster::render_cluster(
                    f,
                    area,
                    cluster::ClusterRenderConfig {
                        loading: self.loading,
                        cluster_info: self.cluster_info.as_ref(),
                    },
                );
            }
            CurrentScreen::Jobs => self.render_jobs(f, area),
            CurrentScreen::JobInspect => self.render_job_details(f, area),
            CurrentScreen::Health => {
                health::render_health(
                    f,
                    area,
                    health::HealthRenderConfig {
                        loading: self.loading,
                        health_info: self.health_info.as_ref(),
                    },
                );
            }
            CurrentScreen::SavedSearches => {
                saved_searches::render_saved_searches(
                    f,
                    area,
                    saved_searches::SavedSearchesRenderConfig {
                        loading: self.loading,
                        saved_searches: self.saved_searches.as_deref(),
                        state: &mut self.saved_searches_state,
                    },
                );
            }
            CurrentScreen::InternalLogs => self.render_internal_logs(f, area),
            CurrentScreen::Apps => {
                apps::render_apps(
                    f,
                    area,
                    apps::AppsRenderConfig {
                        loading: self.loading,
                        apps: self.apps.as_deref(),
                        state: &mut self.apps_state,
                    },
                );
            }
            CurrentScreen::Users => {
                users::render_users(
                    f,
                    area,
                    users::UsersRenderConfig {
                        loading: self.loading,
                        users: self.users.as_deref(),
                        state: &mut self.users_state,
                    },
                );
            }
            CurrentScreen::Settings => {
                settings::render_settings(
                    f,
                    area,
                    settings::SettingsRenderConfig {
                        auto_refresh: self.auto_refresh,
                        sort_column: self.sort_state.column.as_str(),
                        sort_direction: self.sort_state.direction.as_str(),
                        search_history_count: self.search_history.len(),
                        profile_info: std::env::var("SPLUNK_PROFILE").ok().as_deref(),
                    },
                );
            }
        }
    }

    fn render_jobs(&mut self, f: &mut Frame, area: Rect) {
        use crate::ui::screens::jobs;

        if self.loading && self.jobs.is_none() {
            let loading = Paragraph::new("Loading jobs...")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(if self.auto_refresh {
                            "Search Jobs [AUTO]"
                        } else {
                            "Search Jobs"
                        }),
                )
                .alignment(Alignment::Center);
            f.render_widget(loading, area);
            return;
        }

        let jobs = match &self.jobs {
            Some(j) => j,
            None => {
                let placeholder = Paragraph::new(if self.auto_refresh {
                    "No jobs loaded. Press 'r' to refresh, 'a' to toggle auto-refresh."
                } else {
                    "No jobs loaded. Press 'r' to refresh."
                })
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title(if self.auto_refresh {
                            "Search Jobs [AUTO]"
                        } else {
                            "Search Jobs"
                        }),
                )
                .alignment(Alignment::Center);
                f.render_widget(placeholder, area);
                return;
            }
        };

        // Get the filtered and sorted jobs (computed by App for selection consistency)
        let filtered_jobs: Vec<&SearchJobStatus> = self
            .filtered_job_indices
            .iter()
            .filter_map(|&i| jobs.get(i))
            .collect();

        jobs::render_jobs(
            f,
            area,
            jobs::JobsRenderConfig {
                jobs: &filtered_jobs,
                state: &mut self.jobs_state,
                auto_refresh: self.auto_refresh,
                filter: &self.search_filter,
                filter_input: &self.filter_input,
                is_filtering: self.is_filtering,
                sort_column: self.sort_state.column,
                sort_direction: self.sort_state.direction,
                selected_jobs: &self.selected_jobs,
            },
        );
    }

    fn render_job_details(&mut self, f: &mut Frame, area: Rect) {
        use crate::ui::screens::job_details;

        // Get the selected job (accounting for filter/sort)
        let job = self.get_selected_job();

        match job {
            Some(job) => {
                job_details::render_details(f, area, job);
            }
            None => {
                let placeholder = Paragraph::new("No job selected or jobs not loaded.")
                    .block(Block::default().borders(Borders::ALL).title("Job Details"))
                    .alignment(Alignment::Center);
                f.render_widget(placeholder, area);
            }
        }
    }

    fn render_internal_logs(&mut self, f: &mut Frame, area: Rect) {
        use crate::ui::screens::internal_logs;

        internal_logs::render_internal_logs(
            f,
            area,
            internal_logs::InternalLogsRenderConfig {
                loading: self.loading,
                logs: self.internal_logs.as_deref(),
                state: &mut self.internal_logs_state,
                auto_refresh: self.auto_refresh,
            },
        );
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crossterm::event::KeyModifiers;

    #[test]
    fn test_handle_mouse_scroll() {
        let mut app = App::new(None);

        // Scroll Down
        let event_down = MouseEvent {
            kind: MouseEventKind::ScrollDown,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        let action_down = app.handle_mouse(event_down);
        assert!(matches!(action_down, Some(Action::NavigateDown)));

        // Scroll Up
        let event_up = MouseEvent {
            kind: MouseEventKind::ScrollUp,
            column: 0,
            row: 0,
            modifiers: KeyModifiers::empty(),
        };
        let action_up = app.handle_mouse(event_up);
        assert!(matches!(action_up, Some(Action::NavigateUp)));
    }

    #[test]
    fn test_handle_mouse_footer_click() {
        let mut app = App::new(None);
        app.last_area = Rect::new(0, 0, 80, 24);

        // Click "Jobs" (4) in footer
        // " 1:Search 2:Indexes 3:Cluster 4:Jobs 5:Health | q:Quit "
        // Jobs starts at offset 30 in index, so col 31
        let event = MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 34, // middle of "4:Jobs"
            row: 22,    // Middle line of footer (24-2)
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        assert!(matches!(action, Some(Action::LoadJobs)));
        assert_eq!(app.current_screen, CurrentScreen::Jobs);
    }

    #[test]
    fn test_handle_mouse_content_click_jobs_filtered() {
        let mut app = App::new(None);
        app.last_area = Rect::new(0, 0, 80, 24);
        app.current_screen = CurrentScreen::Jobs;
        app.search_filter = Some("job".to_string()); // Filtering active
        app.jobs = Some(vec![SearchJobStatus {
            sid: "job1".to_string(),
            is_done: true,
            is_finalized: true,
            done_progress: 1.0,
            run_duration: 1.0,
            cursor_time: None,
            scan_count: 0,
            event_count: 0,
            result_count: 0,
            disk_usage: 0,
            priority: None,
            label: None,
        }]);
        app.rebuild_filtered_indices();

        // Click first job while filtering
        // Header (3) + Filter Area (3) + Table Header (1) + first row (1) = Row 8
        let event = MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 10,
            row: 8,
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        // Index 0 is selected by default in App::new, so first click on it should Inspect
        assert!(matches!(action, Some(Action::InspectJob)));
        assert_eq!(app.jobs_state.selected(), Some(0));
    }

    #[test]
    fn test_handle_mouse_content_click_jobs() {
        let mut app = App::new(None);
        app.last_area = Rect::new(0, 0, 80, 24);
        app.current_screen = CurrentScreen::Jobs;
        app.jobs = Some(vec![
            SearchJobStatus {
                sid: "job1".to_string(),
                is_done: true,
                is_finalized: true,
                done_progress: 1.0,
                run_duration: 1.0,
                cursor_time: None,
                scan_count: 0,
                event_count: 0,
                result_count: 0,
                disk_usage: 0,
                priority: None,
                label: None,
            },
            SearchJobStatus {
                sid: "job2".to_string(),
                is_done: true,
                is_finalized: true,
                done_progress: 1.0,
                run_duration: 1.0,
                cursor_time: None,
                scan_count: 0,
                event_count: 0,
                result_count: 0,
                disk_usage: 0,
                priority: None,
                label: None,
            },
        ]);
        app.rebuild_filtered_indices();

        // Click second job
        // Header (3) + Table Header (1) + first row (1) = Row 5
        let event = MouseEvent {
            kind: MouseEventKind::Down(crossterm::event::MouseButton::Left),
            column: 10,
            row: 6, // Second row of data
            modifiers: KeyModifiers::empty(),
        };

        let action = app.handle_mouse(event);
        // First click should just select
        assert!(action.is_none());
        assert_eq!(app.jobs_state.selected(), Some(1));

        // Second click on same row should Inspect
        let action2 = app.handle_mouse(event);
        assert!(matches!(action2, Some(Action::InspectJob)));
    }

    #[test]
    fn test_health_state_from_health_str() {
        // Test "green" maps to Healthy
        assert_eq!(HealthState::from_health_str("green"), HealthState::Healthy);
        assert_eq!(HealthState::from_health_str("GREEN"), HealthState::Healthy);
        assert_eq!(HealthState::from_health_str("Green"), HealthState::Healthy);

        // Test "red" maps to Unhealthy
        assert_eq!(HealthState::from_health_str("red"), HealthState::Unhealthy);
        assert_eq!(HealthState::from_health_str("RED"), HealthState::Unhealthy);
        assert_eq!(HealthState::from_health_str("Red"), HealthState::Unhealthy);

        // Test "yellow" and other values map to Unknown
        assert_eq!(HealthState::from_health_str("yellow"), HealthState::Unknown);
        assert_eq!(HealthState::from_health_str("YELLOW"), HealthState::Unknown);
        assert_eq!(
            HealthState::from_health_str("invalid"),
            HealthState::Unknown
        );
        assert_eq!(HealthState::from_health_str(""), HealthState::Unknown);
    }

    #[test]
    fn test_set_health_state_healthy_to_unhealthy_emits_toast() {
        let mut app = App::new(None);
        app.health_state = HealthState::Healthy;

        // Set to unhealthy should emit a toast
        app.set_health_state(HealthState::Unhealthy);

        assert_eq!(app.health_state, HealthState::Unhealthy);
        assert_eq!(app.toasts.len(), 1);
        assert_eq!(
            app.toasts[0].message,
            "Splunk health status changed to unhealthy"
        );
    }

    #[test]
    fn test_set_health_state_unknown_to_unhealthy_emits_no_toast() {
        let mut app = App::new(None);
        // Default state is Unknown
        assert_eq!(app.health_state, HealthState::Unknown);

        // Set to unhealthy from Unknown should not emit a toast
        app.set_health_state(HealthState::Unhealthy);

        assert_eq!(app.health_state, HealthState::Unhealthy);
        assert_eq!(app.toasts.len(), 0);
    }

    #[test]
    fn test_set_health_state_healthy_to_unknown_emits_no_toast() {
        let mut app = App::new(None);
        app.health_state = HealthState::Healthy;

        // Set to unknown should not emit a toast
        app.set_health_state(HealthState::Unknown);

        assert_eq!(app.health_state, HealthState::Unknown);
        assert_eq!(app.toasts.len(), 0);
    }

    #[test]
    fn test_set_health_state_unhealthy_to_healthy_emits_no_toast() {
        let mut app = App::new(None);
        app.health_state = HealthState::Unhealthy;

        // Set to healthy should not emit a toast (only Healthy -> Unhealthy does)
        app.set_health_state(HealthState::Healthy);

        assert_eq!(app.health_state, HealthState::Healthy);
        assert_eq!(app.toasts.len(), 0);
    }

    #[test]
    fn test_health_status_loaded_action_ok() {
        let mut app = App::new(None);

        // Simulate receiving a healthy status
        let health = splunk_client::models::SplunkHealth {
            health: "green".to_string(),
            features: std::collections::HashMap::new(),
        };

        app.update(Action::HealthStatusLoaded(Ok(health)));

        assert_eq!(app.health_state, HealthState::Healthy);
    }

    #[test]
    fn test_health_status_loaded_action_err() {
        let mut app = App::new(None);
        app.health_state = HealthState::Healthy;

        // Simulate error - should set to unhealthy
        app.update(Action::HealthStatusLoaded(Err(
            "Connection failed".to_string()
        )));

        assert_eq!(app.health_state, HealthState::Unhealthy);
        // Should emit toast since we went from Healthy to Unhealthy
        assert_eq!(app.toasts.len(), 1);
    }

    #[test]
    fn test_health_loaded_action_with_splunkd_health() {
        let mut app = App::new(None);

        // Simulate receiving HealthCheckOutput with splunkd_health
        let health_output = HealthCheckOutput {
            server_info: None,
            splunkd_health: Some(splunk_client::models::SplunkHealth {
                health: "red".to_string(),
                features: std::collections::HashMap::new(),
            }),
            license_usage: None,
            kvstore_status: None,
            log_parsing_health: None,
        };

        app.update(Action::HealthLoaded(Box::new(Ok(health_output))));

        assert_eq!(app.health_state, HealthState::Unhealthy);
    }
}
