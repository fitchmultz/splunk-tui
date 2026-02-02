//! Input handling for the workload management screen.
//!
//! Responsibilities:
//! - Handle keyboard input for the workload management screen
//! - Dispatch actions based on key presses
//! - Support toggling between pools and rules views
//!
//! Does NOT handle:
//! - Rendering (handled by screen module)
//! - Data fetching (handled by side effects)

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::state::WorkloadViewMode;

impl App {
    /// Handle keyboard input for the workload management screen.
    ///
    /// # Arguments
    /// * `key` - The key event to process
    ///
    /// # Returns
    /// * `Some(Action)` - Action to execute
    /// * `None` - No action to execute
    pub fn handle_workload_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            // Navigation - depends on current view mode
            KeyCode::Down | KeyCode::Char('j') => {
                match self.workload_view_mode {
                    WorkloadViewMode::Pools => {
                        let next = self
                            .workload_pools_state
                            .selected()
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        let max = self.workload_pools.as_ref().map(|p| p.len()).unwrap_or(0);
                        if next < max {
                            self.workload_pools_state.select(Some(next));
                        }
                    }
                    WorkloadViewMode::Rules => {
                        let next = self
                            .workload_rules_state
                            .selected()
                            .map(|i| i + 1)
                            .unwrap_or(0);
                        let max = self.workload_rules.as_ref().map(|r| r.len()).unwrap_or(0);
                        if next < max {
                            self.workload_rules_state.select(Some(next));
                        }
                    }
                }
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                match self.workload_view_mode {
                    WorkloadViewMode::Pools => {
                        let prev = self
                            .workload_pools_state
                            .selected()
                            .map(|i| i.saturating_sub(1))
                            .unwrap_or(0);
                        self.workload_pools_state.select(Some(prev));
                    }
                    WorkloadViewMode::Rules => {
                        let prev = self
                            .workload_rules_state
                            .selected()
                            .map(|i| i.saturating_sub(1))
                            .unwrap_or(0);
                        self.workload_rules_state.select(Some(prev));
                    }
                }
                None
            }

            // Toggle view mode (Pools <-> Rules)
            KeyCode::Char('w') => Some(Action::ToggleWorkloadViewMode),

            // Refresh current view
            KeyCode::Char('r') => {
                self.loading = true;
                match self.workload_view_mode {
                    WorkloadViewMode::Pools => Some(Action::LoadWorkloadPools {
                        count: self.workload_pools_pagination.page_size,
                        offset: 0,
                    }),
                    WorkloadViewMode::Rules => Some(Action::LoadWorkloadRules {
                        count: self.workload_rules_pagination.page_size,
                        offset: 0,
                    }),
                }
            }

            // Export
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                // Export based on current view mode
                self.begin_export(ExportTarget::Workload);
                None
            }

            // Load more (if available)
            KeyCode::Char('n') => match self.workload_view_mode {
                WorkloadViewMode::Pools => {
                    if self.workload_pools_pagination.can_load_more() {
                        Some(Action::LoadMoreWorkloadPools)
                    } else {
                        None
                    }
                }
                WorkloadViewMode::Rules => {
                    if self.workload_rules_pagination.can_load_more() {
                        Some(Action::LoadMoreWorkloadRules)
                    } else {
                        None
                    }
                }
            },

            _ => None,
        }
    }
}
