//! Settings screen input handler.
//!
//! Responsibilities:
//! - Handle 'a' key to toggle auto-refresh
//! - Handle 's' key to cycle sort column
//! - Handle 'd' key to toggle sort direction
//! - Handle 'c' key to clear search history
//!
//! Does NOT handle:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT persist settings (handled by actions)

use crate::action::Action;
use crate::app::App;
// Note: SortColumn and SortDirection methods are used but the types themselves
// are not directly referenced in this file (used through self.sort_state)
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle input for the settings screen.
    pub fn handle_settings_input(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Char('a') => self.toggle_auto_refresh(),
            KeyCode::Char('s') => self.cycle_sort_column(),
            KeyCode::Char('d') => self.toggle_sort_direction(),
            KeyCode::Char('c') => self.clear_search_history(),
            KeyCode::Char('e') => self.open_edit_profile(),
            KeyCode::Char('x') => self.open_delete_profile(),
            _ => None,
        }
    }

    /// Toggle auto-refresh on/off.
    fn toggle_auto_refresh(&mut self) -> Option<Action> {
        self.auto_refresh = !self.auto_refresh;
        self.toasts.push(Toast::info(format!(
            "Auto-refresh: {}",
            if self.auto_refresh { "On" } else { "Off" }
        )));
        None
    }

    /// Cycle to the next sort column.
    fn cycle_sort_column(&mut self) -> Option<Action> {
        self.sort_state.column = self.sort_state.column.next();
        self.toasts.push(Toast::info(format!(
            "Sort column: {}",
            self.sort_state.column.as_str()
        )));
        None
    }

    /// Toggle sort direction between ascending and descending.
    fn toggle_sort_direction(&mut self) -> Option<Action> {
        self.sort_state.direction = self.sort_state.direction.toggle();
        self.toasts.push(Toast::info(format!(
            "Sort direction: {}",
            self.sort_state.direction.as_str()
        )));
        None
    }

    /// Clear the search history.
    fn clear_search_history(&mut self) -> Option<Action> {
        self.search_history.clear();
        self.toasts.push(Toast::info("Search history cleared"));
        None
    }

    /// Open edit profile dialog for the current profile.
    fn open_edit_profile(&mut self) -> Option<Action> {
        if let Some(profile_name) = &self.profile_name {
            Some(Action::OpenEditProfileDialog {
                name: profile_name.clone(),
            })
        } else {
            self.toasts.push(Toast::warning(
                "No profile selected to edit. Use 'p' to switch to a profile first.",
            ));
            None
        }
    }

    /// Open delete confirmation for the current profile.
    fn open_delete_profile(&mut self) -> Option<Action> {
        if let Some(profile_name) = &self.profile_name {
            Some(Action::OpenDeleteProfileConfirm {
                name: profile_name.clone(),
            })
        } else {
            self.toasts.push(Toast::warning(
                "No profile selected to delete. Use 'p' to switch to a profile first.",
            ));
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::app::ConnectionContext;
    use crate::app::state::{SortColumn, SortDirection};
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn create_test_app() -> App {
        App::new(None, ConnectionContext::default())
    }

    fn key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    #[test]
    fn test_toggle_auto_refresh() {
        let mut app = create_test_app();
        let initial = app.auto_refresh;

        app.toggle_auto_refresh();

        assert_eq!(app.auto_refresh, !initial);
        assert_eq!(app.toasts.len(), 1);
    }

    #[test]
    fn test_cycle_sort_column() {
        let mut app = create_test_app();
        app.sort_state.column = SortColumn::Sid;

        app.cycle_sort_column();
        assert_eq!(app.sort_state.column, SortColumn::Status);

        app.cycle_sort_column();
        assert_eq!(app.sort_state.column, SortColumn::Duration);

        app.cycle_sort_column();
        assert_eq!(app.sort_state.column, SortColumn::Results);

        app.cycle_sort_column();
        assert_eq!(app.sort_state.column, SortColumn::Events);

        app.cycle_sort_column();
        assert_eq!(app.sort_state.column, SortColumn::Sid); // Wrap around
    }

    #[test]
    fn test_toggle_sort_direction() {
        let mut app = create_test_app();
        app.sort_state.direction = SortDirection::Asc;

        app.toggle_sort_direction();
        assert_eq!(app.sort_state.direction, SortDirection::Desc);

        app.toggle_sort_direction();
        assert_eq!(app.sort_state.direction, SortDirection::Asc);
    }

    #[test]
    fn test_clear_search_history() {
        let mut app = create_test_app();
        app.search_history = vec!["query1".to_string(), "query2".to_string()];

        app.clear_search_history();

        assert!(app.search_history.is_empty());
        assert_eq!(app.toasts.len(), 1);
    }

    #[test]
    fn test_open_edit_profile_with_profile() {
        let mut app = create_test_app();
        app.profile_name = Some("test_profile".to_string());

        let action = app.open_edit_profile();

        assert!(
            matches!(action, Some(Action::OpenEditProfileDialog { name }) if name == "test_profile")
        );
    }

    #[test]
    fn test_open_edit_profile_without_profile() {
        let mut app = create_test_app();
        app.profile_name = None;

        let action = app.open_edit_profile();

        assert!(action.is_none());
        assert_eq!(app.toasts.len(), 1);
    }

    #[test]
    fn test_open_delete_profile_with_profile() {
        let mut app = create_test_app();
        app.profile_name = Some("test_profile".to_string());

        let action = app.open_delete_profile();

        assert!(
            matches!(action, Some(Action::OpenDeleteProfileConfirm { name }) if name == "test_profile")
        );
    }

    #[test]
    fn test_open_delete_profile_without_profile() {
        let mut app = create_test_app();
        app.profile_name = None;

        let action = app.open_delete_profile();

        assert!(action.is_none());
        assert_eq!(app.toasts.len(), 1);
    }

    #[test]
    fn test_handle_settings_input_auto_refresh() {
        let mut app = create_test_app();
        let initial = app.auto_refresh;

        let action = app.handle_settings_input(key('a'));

        assert!(action.is_none());
        assert_eq!(app.auto_refresh, !initial);
    }

    #[test]
    fn test_handle_settings_input_sort_column() {
        let mut app = create_test_app();
        app.sort_state.column = SortColumn::Sid;

        app.handle_settings_input(key('s'));

        assert_eq!(app.sort_state.column, SortColumn::Status);
    }

    #[test]
    fn test_handle_settings_input_sort_direction() {
        let mut app = create_test_app();
        app.sort_state.direction = SortDirection::Asc;

        app.handle_settings_input(key('d'));

        assert_eq!(app.sort_state.direction, SortDirection::Desc);
    }

    #[test]
    fn test_handle_settings_input_clear_history() {
        let mut app = create_test_app();
        app.search_history = vec!["query1".to_string()];

        app.handle_settings_input(key('c'));

        assert!(app.search_history.is_empty());
    }
}
