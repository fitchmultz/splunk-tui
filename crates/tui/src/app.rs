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
//! - `render`: Rendering logic

pub mod clipboard;
pub mod core;
pub mod state;
pub mod structs;

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
pub use structs::{App, ConnectionContext, SplValidationState};

use crate::action::Action;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent};
use splunk_config::constants::{
    DEFAULT_CLIPBOARD_PREVIEW_CHARS, DEFAULT_HISTORY_MAX_ITEMS, DEFAULT_SCROLL_THRESHOLD,
    DEFAULT_TIMEOUT_SECS,
};

impl App {
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
        self.search_results_total_count = Some(self.search_results.len() as u64);
        self.search_has_more_results = false;
        // Reset scroll offset when new results arrive
        self.search_scroll_offset = 0;
    }

    /// Append more search results (for pagination, virtualization: no eager formatting).
    pub fn append_search_results(
        &mut self,
        mut results: Vec<serde_json::Value>,
        total: Option<u64>,
    ) {
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
            CurrentScreen::Macros => Some(Action::LoadMacros),
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
            CurrentScreen::Roles => Some(Action::LoadRoles {
                count: 100,
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
            CurrentScreen::FiredAlerts => Some(Action::LoadFiredAlerts {
                count: self.fired_alerts_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::Forwarders => Some(Action::LoadForwarders {
                count: self.forwarders_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::Lookups => Some(Action::LoadLookups {
                count: self.lookups_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::Audit => Some(Action::LoadAuditEvents {
                count: 50,
                offset: 0,
                earliest: "-24h".to_string(),
                latest: "now".to_string(),
            }),
            CurrentScreen::Dashboards => Some(Action::LoadDashboards {
                count: self.dashboards_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::DataModels => Some(Action::LoadDataModels {
                count: self.data_models_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::WorkloadManagement => Some(Action::LoadWorkloadPools {
                count: self.workload_pools_pagination.page_size,
                offset: 0,
            }),
            CurrentScreen::Shc => Some(Action::LoadShcStatus),
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
            CurrentScreen::Inputs => {
                if self.inputs_pagination.can_load_more() {
                    Some(Action::LoadInputs {
                        count: self.inputs_pagination.page_size,
                        offset: self.inputs_pagination.current_offset,
                    })
                } else {
                    None
                }
            }
            CurrentScreen::FiredAlerts => {
                if self.fired_alerts_pagination.can_load_more() {
                    Some(Action::LoadFiredAlerts {
                        count: self.fired_alerts_pagination.page_size,
                        offset: self.fired_alerts_pagination.current_offset,
                    })
                } else {
                    None
                }
            }
            CurrentScreen::Dashboards => {
                if self.dashboards_pagination.can_load_more() {
                    Some(Action::LoadDashboards {
                        count: self.dashboards_pagination.page_size,
                        offset: self.dashboards_pagination.current_offset,
                    })
                } else {
                    None
                }
            }
            CurrentScreen::DataModels => {
                if self.data_models_pagination.can_load_more() {
                    Some(Action::LoadDataModels {
                        count: self.data_models_pagination.page_size,
                        offset: self.data_models_pagination.current_offset,
                    })
                } else {
                    None
                }
            }
            CurrentScreen::WorkloadManagement => {
                // Load more based on current view mode
                match self.workload_view_mode {
                    crate::app::state::WorkloadViewMode::Pools => {
                        if self.workload_pools_pagination.can_load_more() {
                            Some(Action::LoadWorkloadPools {
                                count: self.workload_pools_pagination.page_size,
                                offset: self.workload_pools_pagination.current_offset,
                            })
                        } else {
                            None
                        }
                    }
                    crate::app::state::WorkloadViewMode::Rules => {
                        if self.workload_rules_pagination.can_load_more() {
                            Some(Action::LoadWorkloadRules {
                                count: self.workload_rules_pagination.page_size,
                                offset: self.workload_rules_pagination.current_offset,
                            })
                        } else {
                            None
                        }
                    }
                }
            }
            _ => None,
        }
    }

    /// Translate a LoadMore* action into a concrete Load* action with pagination params.
    ///
    /// This helper centralizes the translation logic for all pagination triggers,
    /// making it testable and reusable from both the main loop and input handlers.
    ///
    /// # Arguments
    /// * `action` - The action to translate
    ///
    /// # Returns
    /// The translated action, or the original action if no translation is needed
    pub fn translate_load_more_action(&self, action: Action) -> Action {
        match action {
            Action::LoadMoreIndexes
            | Action::LoadMoreJobs
            | Action::LoadMoreApps
            | Action::LoadMoreUsers
            | Action::LoadMoreSearchPeers
            | Action::LoadMoreForwarders
            | Action::LoadMoreLookups
            | Action::LoadMoreInputs
            | Action::LoadMoreFiredAlerts
            | Action::LoadMoreDashboards
            | Action::LoadMoreDataModels
            | Action::LoadMoreWorkloadPools
            | Action::LoadMoreWorkloadRules => {
                self.load_more_action_for_current_screen().unwrap_or(action)
            }
            Action::LoadMoreInternalLogs => Action::LoadInternalLogs {
                count: self.internal_logs_defaults.count,
                earliest: self.internal_logs_defaults.earliest_time.clone(),
            },
            _ => action,
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
            && (input::helpers::is_printable_char(key)
                || input::helpers::is_mode_switch_key(key)
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
