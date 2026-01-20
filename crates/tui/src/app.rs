//! Application state and rendering.
//!
//! This module contains the main application state, input handling,
//! and rendering logic for the TUI.

use crate::action::Action;
use crate::ui::Toast;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::KeyEvent;
use ratatui::{
    Frame,
    layout::{Alignment, Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, List, ListItem, ListState, Paragraph, TableState},
};
use serde_json::Value;
use splunk_client::models::{ClusterInfo, Index, SearchJobStatus};
use splunk_config::PersistedState;

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

    // Real data (Option for loading state)
    pub indexes: Option<Vec<Index>>,
    pub indexes_state: ListState,
    pub jobs: Option<Vec<SearchJobStatus>>,
    pub jobs_state: TableState,
    pub cluster_info: Option<ClusterInfo>,

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

    // Jobs sort state
    pub sort_state: SortState,
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

        let (auto_refresh, sort_column, sort_direction, last_search_query) = match persisted {
            Some(state) => (
                state.auto_refresh,
                parse_sort_column(&state.sort_column),
                parse_sort_direction(&state.sort_direction),
                state.last_search_query,
            ),
            None => (false, SortColumn::Sid, SortDirection::Asc, None),
        };

        Self {
            current_screen: CurrentScreen::Search,
            search_input: last_search_query.unwrap_or_default(),
            search_status: String::from("Press Enter to execute search"),
            search_results: Vec::new(),
            search_scroll_offset: 0,
            search_sid: None,
            indexes: None,
            indexes_state,
            jobs: None,
            jobs_state,
            cluster_info: None,
            loading: false,
            progress: 0.0,
            toasts: Vec::new(),
            auto_refresh,
            popup: None,
            search_filter: None,
            is_filtering: false,
            filter_input: String::new(),
            sort_state: SortState {
                column: sort_column,
                direction: sort_direction,
            },
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
        }
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
        }
    }

    /// Handle periodic tick events - returns Action if one should be dispatched.
    pub fn handle_tick(&self) -> Option<Action> {
        if self.current_screen == CurrentScreen::Jobs
            && self.auto_refresh
            && self.popup.is_none()
            && !self.is_filtering
        {
            Some(Action::LoadJobs)
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
            (
                Some(PopupType::ConfirmCancel(_) | PopupType::ConfirmDelete(_)),
                KeyCode::Char('n') | KeyCode::Esc,
            ) => {
                self.popup = None;
                None
            }
            _ => None,
        }
    }

    fn handle_search_input(&mut self, key: KeyEvent) -> Option<Action> {
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
            KeyCode::Char('j') => Some(Action::NavigateDown),
            KeyCode::Char('k') => Some(Action::NavigateUp),
            KeyCode::Enter => {
                if !self.search_input.is_empty() {
                    let query = self.search_input.clone();
                    self.search_status = format!("Running: {}", query);
                    Some(Action::RunSearch(query))
                } else {
                    None
                }
            }
            KeyCode::Backspace => {
                self.search_input.pop();
                None
            }
            KeyCode::Down => Some(Action::NavigateDown),
            KeyCode::Up => Some(Action::NavigateUp),
            KeyCode::PageDown => Some(Action::PageDown),
            KeyCode::PageUp => Some(Action::PageUp),
            KeyCode::Home => Some(Action::GoToTop),
            KeyCode::End => Some(Action::GoToBottom),
            KeyCode::Char('?') => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
                None
            }
            KeyCode::Char(c) => {
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
                        Some(Action::ClearSearch) // Triggers re-render
                    } else {
                        Some(Action::ClearSearch)
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
                if let (Some(state), Some(jobs)) = (self.jobs_state.selected(), &self.jobs)
                    && let Some(job) = jobs.get(state)
                {
                    self.popup =
                        Some(Popup::builder(PopupType::ConfirmCancel(job.sid.clone())).build());
                }
                None
            }
            KeyCode::Char('d') => {
                if let (Some(state), Some(jobs)) = (self.jobs_state.selected(), &self.jobs)
                    && let Some(job) = jobs.get(state)
                {
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
            }
            Action::CycleSortColumn => {
                self.sort_state.cycle();
            }
            Action::ToggleSortDirection => {
                self.sort_state.toggle_direction();
            }
            Action::Loading(is_loading) => {
                self.loading = is_loading;
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
                let jobs_len = jobs.len();
                self.jobs = Some(jobs);
                self.loading = false;
                // Restore selection clamped to new bounds
                self.jobs_state
                    .select(sel.map(|i| i.min(jobs_len.saturating_sub(1))).or(Some(0)));
            }
            Action::ClusterInfoLoaded(Ok(info)) => {
                self.cluster_info = Some(info);
                self.loading = false;
            }
            Action::SearchComplete(Ok((results, sid))) => {
                self.search_results = results;
                self.search_sid = Some(sid);
                self.search_status = format!("Search complete: {}", self.search_input);
                self.loading = false;
            }
            Action::JobOperationComplete(msg) => {
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
            Action::ClusterInfoLoaded(Err(e)) => {
                self.toasts
                    .push(Toast::error(format!("Failed to load cluster info: {}", e)));
                self.loading = false;
            }
            Action::SearchComplete(Err(e)) => {
                self.toasts
                    .push(Toast::error(format!("Search failed: {}", e)));
                self.loading = false;
            }
            Action::InspectJob => {
                // Transition to job inspect screen if we have jobs and a selection
                if self.jobs.is_some() && self.jobs_state.selected().is_some() {
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
            CurrentScreen::Jobs => {
                if let Some(jobs) = &self.jobs {
                    let i = self.jobs_state.selected().unwrap_or(0);
                    if i < jobs.len().saturating_sub(1) {
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
            _ => {}
        }
    }

    fn previous_item(&mut self) {
        match self.current_screen {
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
            _ => {}
        }
    }

    fn next_page(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                self.search_scroll_offset = self.search_scroll_offset.saturating_add(10);
            }
            CurrentScreen::Jobs => {
                if let Some(jobs) = &self.jobs {
                    let i = self.jobs_state.selected().unwrap_or(0);
                    self.jobs_state
                        .select(Some((i.saturating_add(10)).min(jobs.len() - 1)));
                }
            }
            CurrentScreen::Indexes => {
                if let Some(indexes) = &self.indexes {
                    let i = self.indexes_state.selected().unwrap_or(0);
                    self.indexes_state
                        .select(Some((i.saturating_add(10)).min(indexes.len() - 1)));
                }
            }
            _ => {}
        }
    }

    fn previous_page(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
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
            _ => {}
        }
    }

    fn go_to_top(&mut self) {
        match self.current_screen {
            CurrentScreen::Search => {
                self.search_scroll_offset = 0;
            }
            CurrentScreen::Jobs => {
                self.jobs_state.select(Some(0));
            }
            CurrentScreen::Indexes => {
                self.indexes_state.select(Some(0));
            }
            _ => {}
        }
    }

    fn go_to_bottom(&mut self) {
        match self.current_screen {
            CurrentScreen::Jobs => {
                if let Some(jobs) = &self.jobs {
                    self.jobs_state.select(Some(jobs.len().saturating_sub(1)));
                }
            }
            CurrentScreen::Indexes => {
                if let Some(indexes) = &self.indexes {
                    self.indexes_state
                        .select(Some(indexes.len().saturating_sub(1)));
                }
            }
            _ => {}
        }
    }

    pub fn render(&mut self, f: &mut Frame) {
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
                },
                Style::default().fg(Color::Yellow),
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
                Span::raw(" 1:Search 2:Indexes 3:Cluster 4:Jobs "),
                Span::raw("|"),
                Span::styled(" q:Quit ", Style::default().fg(Color::Red)),
            ])]
        } else {
            vec![Line::from(vec![
                Span::raw(" 1:Search 2:Indexes 3:Cluster 4:Jobs "),
                Span::raw("|"),
                Span::styled(" q:Quit ", Style::default().fg(Color::Red)),
            ])]
        };
        let footer = Paragraph::new(footer_text).block(Block::default().borders(Borders::ALL));
        f.render_widget(footer, chunks[2]);

        // Render toasts
        crate::ui::toast::render_toasts(f, &self.toasts);

        // Render popup if active (on top of toasts)
        if let Some(ref popup) = self.popup {
            crate::ui::popup::render_popup(f, popup);
        }
    }

    fn render_content(&mut self, f: &mut Frame, area: Rect) {
        match self.current_screen {
            CurrentScreen::Search => self.render_search(f, area),
            CurrentScreen::Indexes => self.render_indexes(f, area),
            CurrentScreen::Cluster => self.render_cluster(f, area),
            CurrentScreen::Jobs => self.render_jobs(f, area),
            CurrentScreen::JobInspect => self.render_job_details(f, area),
        }
    }

    fn render_search(&mut self, f: &mut Frame, area: Rect) {
        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints(
                [
                    Constraint::Length(3), // Search input
                    Constraint::Length(3), // Status
                    Constraint::Min(0),    // Results
                ]
                .as_ref(),
            )
            .split(area);

        // Search input
        let input = Paragraph::new(self.search_input.as_str())
            .block(Block::default().borders(Borders::ALL).title("Search Query"));
        f.render_widget(input, chunks[0]);

        // Status
        let status = Paragraph::new(self.search_status.as_str())
            .block(Block::default().borders(Borders::ALL).title("Status"));
        f.render_widget(status, chunks[1]);

        // Results
        if self.search_results.is_empty() {
            let placeholder = Paragraph::new("No results. Enter a search query and press Enter.")
                .block(Block::default().borders(Borders::ALL).title("Results"))
                .alignment(Alignment::Center);
            f.render_widget(placeholder, chunks[2]);
        } else {
            let results_text: Vec<Line> = self
                .search_results
                .iter()
                .skip(self.search_scroll_offset)
                .map(|v| {
                    Line::from(
                        serde_json::to_string_pretty(v).unwrap_or_else(|_| "<invalid>".to_string()),
                    )
                })
                .collect();

            let results = Paragraph::new(results_text)
                .block(Block::default().borders(Borders::ALL).title("Results"));
            f.render_widget(results, chunks[2]);
        }
    }

    fn render_indexes(&mut self, f: &mut Frame, area: Rect) {
        if self.loading && self.indexes.is_none() {
            let loading = Paragraph::new("Loading indexes...")
                .block(Block::default().borders(Borders::ALL).title("Indexes"))
                .alignment(Alignment::Center);
            f.render_widget(loading, area);
            return;
        }

        let indexes = match &self.indexes {
            Some(i) => i,
            None => {
                let placeholder = Paragraph::new("No indexes loaded. Press 'r' to refresh.")
                    .block(Block::default().borders(Borders::ALL).title("Indexes"))
                    .alignment(Alignment::Center);
                f.render_widget(placeholder, area);
                return;
            }
        };

        let items: Vec<ListItem> = indexes
            .iter()
            .map(|i| {
                ListItem::new(format!(
                    "{} - {} events, {} MB",
                    i.name, i.total_event_count, i.current_db_size_mb
                ))
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title("Indexes"))
            .highlight_style(Style::default().fg(Color::Yellow));
        f.render_stateful_widget(list, area, &mut self.indexes_state);
    }

    fn render_cluster(&mut self, f: &mut Frame, area: Rect) {
        if self.loading && self.cluster_info.is_none() {
            let loading = Paragraph::new("Loading cluster info...")
                .block(
                    Block::default()
                        .borders(Borders::ALL)
                        .title("Cluster Information"),
                )
                .alignment(Alignment::Center);
            f.render_widget(loading, area);
            return;
        }

        let info = match &self.cluster_info {
            Some(i) => i,
            None => {
                let placeholder = Paragraph::new("No cluster info loaded. Press 'r' to refresh.")
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .title("Cluster Information"),
                    )
                    .alignment(Alignment::Center);
                f.render_widget(placeholder, area);
                return;
            }
        };

        let items: Vec<ListItem> = vec![
            ListItem::new(format!("ID: {}", info.id)),
            ListItem::new(format!("Mode: {}", info.mode)),
            ListItem::new(format!("Label: {:?}", info.label)),
            ListItem::new(format!("Manager URI: {:?}", info.manager_uri)),
            ListItem::new(format!("Replication Factor: {:?}", info.replication_factor)),
            ListItem::new(format!("Search Factor: {:?}", info.search_factor)),
            ListItem::new(format!("Status: {:?}", info.status)),
        ];

        let list = List::new(items).block(
            Block::default()
                .borders(Borders::ALL)
                .title("Cluster Information"),
        );
        f.render_widget(list, area);
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

        jobs::render_jobs(
            f,
            area,
            jobs::JobsRenderConfig {
                jobs,
                state: &mut self.jobs_state,
                auto_refresh: self.auto_refresh,
                filter: &self.search_filter,
                filter_input: &self.filter_input,
                is_filtering: self.is_filtering,
                sort_column: self.sort_state.column,
                sort_direction: self.sort_state.direction,
            },
        );
    }

    fn render_job_details(&mut self, f: &mut Frame, area: Rect) {
        use crate::ui::screens::job_details;

        // Get the selected job
        let job = match (&self.jobs, self.jobs_state.selected()) {
            (Some(jobs), Some(selected_idx)) => jobs.get(selected_idx),
            _ => None,
        };

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
}
