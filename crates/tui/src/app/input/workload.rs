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
                self.navigate_workload_down();
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.navigate_workload_up();
                None
            }

            // Toggle view mode (Pools <-> Rules)
            KeyCode::Char('w') => Some(Action::ToggleWorkloadViewMode),

            // Refresh current view
            KeyCode::Char('r') => self.refresh_workload(),

            // Export
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.begin_export(ExportTarget::Workload);
                None
            }

            // Load more (if available)
            KeyCode::Char('n') => self.load_more_workload(),

            _ => None,
        }
    }

    /// Navigate down in the current workload view.
    fn navigate_workload_down(&mut self) {
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
    }

    /// Navigate up in the current workload view.
    fn navigate_workload_up(&mut self) {
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
    }

    /// Refresh the current workload view.
    fn refresh_workload(&mut self) -> Option<Action> {
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

    /// Load more items for the current workload view.
    fn load_more_workload(&self) -> Option<Action> {
        match self.workload_view_mode {
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
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::app::ConnectionContext;
    use crate::app::state::WorkloadViewMode;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn create_test_app() -> App {
        App::new(None, ConnectionContext::default())
    }

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn down_key() -> KeyEvent {
        KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)
    }

    fn up_key() -> KeyEvent {
        KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)
    }

    /// Create a test WorkloadPool with just a name
    fn create_test_pool(name: &str) -> splunk_client::WorkloadPool {
        splunk_client::WorkloadPool {
            name: name.to_string(),
            cpu_weight: None,
            mem_weight: None,
            default_pool: None,
            enabled: None,
            search_concurrency: None,
            search_time_range: None,
            admission_rules_enabled: None,
            cpu_cores: None,
            mem_limit: None,
        }
    }

    /// Create a test WorkloadRule with just a name
    fn create_test_rule(name: &str) -> splunk_client::WorkloadRule {
        splunk_client::WorkloadRule {
            name: name.to_string(),
            predicate: None,
            workload_pool: None,
            user: None,
            app: None,
            search_type: None,
            search_time_range: None,
            enabled: None,
            order: None,
        }
    }

    #[test]
    fn test_navigate_workload_down_pools() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Pools;
        // Simulate having 3 pools
        app.workload_pools = Some(vec![
            create_test_pool("pool1"),
            create_test_pool("pool2"),
            create_test_pool("pool3"),
        ]);
        app.workload_pools_state.select(Some(0));

        app.navigate_workload_down();
        assert_eq!(app.workload_pools_state.selected(), Some(1));

        app.navigate_workload_down();
        assert_eq!(app.workload_pools_state.selected(), Some(2));

        // Should not go beyond last item
        app.navigate_workload_down();
        assert_eq!(app.workload_pools_state.selected(), Some(2));
    }

    #[test]
    fn test_navigate_workload_down_rules() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Rules;
        // Simulate having 2 rules
        app.workload_rules = Some(vec![create_test_rule("rule1"), create_test_rule("rule2")]);
        app.workload_rules_state.select(Some(0));

        app.navigate_workload_down();
        assert_eq!(app.workload_rules_state.selected(), Some(1));

        // Should not go beyond last item
        app.navigate_workload_down();
        assert_eq!(app.workload_rules_state.selected(), Some(1));
    }

    #[test]
    fn test_navigate_workload_up_pools() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Pools;
        app.workload_pools = Some(vec![
            create_test_pool("pool1"),
            create_test_pool("pool2"),
            create_test_pool("pool3"),
        ]);
        app.workload_pools_state.select(Some(2));

        app.navigate_workload_up();
        assert_eq!(app.workload_pools_state.selected(), Some(1));

        app.navigate_workload_up();
        assert_eq!(app.workload_pools_state.selected(), Some(0));

        // Should stay at 0 (saturating_sub)
        app.navigate_workload_up();
        assert_eq!(app.workload_pools_state.selected(), Some(0));
    }

    #[test]
    fn test_navigate_workload_up_rules() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Rules;
        app.workload_rules = Some(vec![create_test_rule("rule1"), create_test_rule("rule2")]);
        app.workload_rules_state.select(Some(1));

        app.navigate_workload_up();
        assert_eq!(app.workload_rules_state.selected(), Some(0));

        // Should stay at 0
        app.navigate_workload_up();
        assert_eq!(app.workload_rules_state.selected(), Some(0));
    }

    #[test]
    fn test_refresh_workload_pools() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Pools;

        let action = app.refresh_workload();

        assert!(matches!(action, Some(Action::LoadWorkloadPools { count, offset }) if offset == 0));
        assert!(app.loading);
    }

    #[test]
    fn test_refresh_workload_rules() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Rules;

        let action = app.refresh_workload();

        assert!(matches!(action, Some(Action::LoadWorkloadRules { count, offset }) if offset == 0));
        assert!(app.loading);
    }

    #[test]
    fn test_load_more_workload_pools_when_can_load() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Pools;
        app.workload_pools_pagination.has_more = true;
        app.workload_pools_pagination.total_loaded = 10;
        app.workload_pools_pagination.page_size = 10;
        app.workload_pools_pagination.max_items = 100;

        let action = app.load_more_workload();

        assert!(matches!(action, Some(Action::LoadMoreWorkloadPools)));
    }

    #[test]
    fn test_load_more_workload_pools_when_cannot_load() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Pools;
        app.workload_pools_pagination.has_more = false;

        let action = app.load_more_workload();

        assert!(action.is_none());
    }

    #[test]
    fn test_load_more_workload_rules_when_can_load() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Rules;
        app.workload_rules_pagination.has_more = true;
        app.workload_rules_pagination.total_loaded = 10;
        app.workload_rules_pagination.page_size = 10;
        app.workload_rules_pagination.max_items = 100;

        let action = app.load_more_workload();

        assert!(matches!(action, Some(Action::LoadMoreWorkloadRules)));
    }

    #[test]
    fn test_load_more_workload_rules_when_cannot_load() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Rules;
        app.workload_rules_pagination.has_more = false;

        let action = app.load_more_workload();

        assert!(action.is_none());
    }

    #[test]
    fn test_handle_workload_input_toggle_view() {
        let mut app = create_test_app();

        let action = app.handle_workload_input(key('w'));

        assert!(matches!(action, Some(Action::ToggleWorkloadViewMode)));
    }

    #[test]
    fn test_handle_workload_input_down() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Pools;
        app.workload_pools = Some(vec![create_test_pool("pool1"), create_test_pool("pool2")]);

        let action = app.handle_workload_input(down_key());

        assert!(action.is_none());
    }

    #[test]
    fn test_handle_workload_input_up() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Pools;
        app.workload_pools = Some(vec![create_test_pool("pool1"), create_test_pool("pool2")]);
        app.workload_pools_state.select(Some(1));

        let action = app.handle_workload_input(up_key());

        assert!(action.is_none());
        assert_eq!(app.workload_pools_state.selected(), Some(0));
    }

    #[test]
    fn test_handle_workload_input_refresh() {
        let mut app = create_test_app();
        app.workload_view_mode = WorkloadViewMode::Pools;

        let action = app.handle_workload_input(key('r'));

        assert!(matches!(action, Some(Action::LoadWorkloadPools { .. })));
    }
}
