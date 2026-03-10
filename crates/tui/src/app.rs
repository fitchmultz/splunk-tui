//! Application state and rendering.
//!
//! This module contains the main application state, input handling,
//! and rendering logic for the TUI.
//!
//! The module is organized into submodules:
//! - `structs`: Core struct definitions (App, ConnectionContext, SplValidationState)
//! - `core`: App lifecycle methods (new, default, persistence)
//! - `state`: Core state types (HealthState, CurrentScreen, Sort types)
//! - `clipboard`: Clipboard integration
//! - `export`: Export functionality
//! - `navigation`: Navigation helpers (next/previous item, page up/down, etc.)
//! - `jobs`: Jobs-specific logic (filtering, sorting)
//! - `mouse`: Mouse event handling
//! - `popups`: Popup input handling
//! - `input`: Per-screen input handlers
//! - `actions`: Action handling
//! - `parsing`: Parsing helpers for API string values
//! - `render`: Rendering logic

pub mod clipboard;
pub mod command_palette;
pub mod core;
pub mod state;
pub mod structs;

mod actions;
mod export;
pub mod footer_layout;
pub mod input;
mod jobs;
mod load_actions;
mod mouse;
mod navigation;
mod parsing;
mod popups;
mod render;

pub use state::{
    ClusterViewMode, CurrentScreen, EscAction, FOOTER_HEIGHT, HEADER_HEIGHT, HealthState,
    ListPaginationState, NavigationContext, NavigationMode, SearchInputMode, SortColumn,
    SortDirection, SortState, TabAction,
};
pub use structs::{App, ConnectionContext, SplValidationState};

use crate::action::Action;
use crate::ui::{Toast, ToastLevel};
use crossterm::event::{KeyCode, KeyEvent};
use splunk_config::constants::{
    DEFAULT_CLIPBOARD_PREVIEW_CHARS, DEFAULT_HISTORY_MAX_ITEMS, DEFAULT_SCROLL_THRESHOLD,
    DEFAULT_TIMEOUT_SECS,
};

impl App {
    /// Push a toast only if an identical active toast is not already present.
    pub(crate) fn push_toast_once(&mut self, level: ToastLevel, message: impl Into<String>) {
        let message = message.into();
        let duplicate_active = self
            .toasts
            .iter()
            .any(|t| !t.is_expired() && t.level == level && t.message == message);
        if duplicate_active {
            return;
        }
        self.toasts.push(Toast::new(message, level));
    }

    /// Push an info toast with active-duplicate suppression.
    pub(crate) fn push_info_toast_once(&mut self, message: impl Into<String>) {
        self.push_toast_once(ToastLevel::Info, message);
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
    pub fn set_search_results(&mut self, results: Vec<serde_json::Value>) {
        self.search_results = results;
        self.search_results_total_count = Some(self.search_results.len());
        self.search_has_more_results = false;
        // Reset scroll offset when new results arrive
        self.search_scroll_offset = 0;
    }

    /// Append more search results (for pagination, virtualization: no eager formatting).
    pub fn append_search_results(
        &mut self,
        mut results: Vec<serde_json::Value>,
        total: Option<usize>,
    ) {
        let results_count = results.len();
        self.search_results.append(&mut results);
        self.search_results_total_count = total;

        // Determine if more results may exist
        self.search_has_more_results = if let Some(t) = total {
            // When total is known, use it directly
            self.search_results.len() < t
        } else {
            // When total is None, infer from page fullness:
            // If we got exactly page_size results, there might be more.
            // If we got fewer, we're likely at the end.
            results_count >= self.search_results_page_size
        };
        // Note: No pre-formatting - results are formatted on-demand during rendering
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
                offset: loaded_count,
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

        // Onboarding checklist dismiss shortcuts are global while the widget is visible.
        if self.onboarding_checklist.should_show_checklist() {
            let is_shift_d = key.code == KeyCode::Char('D')
                || (key.code == KeyCode::Char('d')
                    && key.modifiers == crossterm::event::KeyModifiers::SHIFT);
            if is_shift_d {
                return Some(Action::DismissOnboardingItem);
            }
            if key.code == KeyCode::Char('d')
                && key.modifiers == crossterm::event::KeyModifiers::CONTROL
            {
                return Some(Action::DismissOnboardingAll);
            }
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
        // Tab/BackTab are handled by global keymap for screen navigation (deterministic behavior).
        // Also skip cursor movement/editing keys for query editing (RQ-0110).
        let skip_global_bindings = self.current_screen == CurrentScreen::Search
            && matches!(self.search_input_mode, SearchInputMode::QueryFocused)
            && (input::helpers::is_printable_char(key)
                || input::helpers::is_cursor_editing_key(key));

        if !skip_global_bindings
            && let Some(action) = crate::input::keymap::resolve_action(self.current_screen, key)
        {
            return Some(action);
        }

        self.dispatch_screen_input(key)
    }

    /// Set loading state with automatic timestamp tracking.
    pub fn set_loading(&mut self, loading: bool) {
        self.loading = loading;
        if loading {
            self.loading_since = Some(std::time::Instant::now());
        } else {
            self.loading_since = None;
        }
    }

    /// Handle periodic tick events - returns Action if one should be dispatched.
    pub fn handle_tick(&mut self) -> Option<Action> {
        // Check for loading timeout to prevent stuck loading state
        const LOADING_TIMEOUT_SECS: u64 = DEFAULT_TIMEOUT_SECS;
        if self.loading {
            if let Some(since) = self.loading_since {
                if since.elapsed().as_secs() > LOADING_TIMEOUT_SECS {
                    self.loading = false;
                    self.loading_since = None;
                    self.toasts.push(crate::ui::Toast::warning(format!(
                        "Operation timed out after {} seconds",
                        LOADING_TIMEOUT_SECS
                    )));
                }
            } else {
                // If loading is true but no timestamp, set it now (backwards compatibility)
                self.loading_since = Some(std::time::Instant::now());
            }
        }

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
