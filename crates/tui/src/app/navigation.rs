//! Navigation helpers for the TUI app.
//!
//! Responsibilities:
//! - Handle item navigation (next/previous)
//! - Handle page navigation (page up/down)
//! - Handle jump navigation (top/bottom)
//!
//! Non-responsibilities:
//! - Does NOT handle screen switching (handled by actions)
//! - Does NOT handle input events

use crate::app::App;
use crate::app::state::CurrentScreen;

impl App {
    // Navigation helpers
    pub(crate) fn next_item(&mut self) {
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

    pub(crate) fn previous_item(&mut self) {
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

    pub(crate) fn next_page(&mut self) {
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

    pub(crate) fn previous_page(&mut self) {
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

    pub(crate) fn go_to_top(&mut self) {
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

    pub(crate) fn go_to_bottom(&mut self) {
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
}
