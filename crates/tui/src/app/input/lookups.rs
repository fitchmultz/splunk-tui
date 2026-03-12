//! Input handling for the lookups screen.
//!
//! Responsibilities:
//! - Handle keyboard input for the lookups screen
//! - Dispatch actions based on key presses
//!
//! Does NOT handle:
//! - Rendering (handled by screen module)
//! - Data fetching (handled by side effects)

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::input::helpers::{
    handle_copy_with_toast, handle_list_export, is_copy_key, is_export_key, should_export_list,
};
use splunk_client::models::LookupTable;

impl App {
    /// Handle keyboard input for the lookups screen.
    ///
    /// # Arguments
    /// * `key` - The key event to process
    ///
    /// # Returns
    /// * `Some(Action)` - Action to execute
    /// * `None` - No action to execute
    pub fn handle_lookups_input(&mut self, key: KeyEvent) -> Option<Action> {
        if is_copy_key(key) {
            let content = self.selected_lookup().map(|lookup| lookup.name.clone());

            return handle_copy_with_toast(self, content);
        }

        match key.code {
            // Navigation
            KeyCode::Down | KeyCode::Char('j') => {
                self.next_item();
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                self.previous_item();
                None
            }
            KeyCode::PageDown => {
                self.next_page();
                None
            }
            KeyCode::PageUp => {
                self.previous_page();
                None
            }
            KeyCode::Home => {
                self.go_to_top();
                None
            }
            KeyCode::End => {
                self.go_to_bottom();
                None
            }

            // Refresh
            KeyCode::Char('r') => {
                self.loading = true;
                Some(Action::LoadLookups {
                    count: self.lookups_pagination.page_size,
                    offset: 0,
                })
            }

            // Export
            KeyCode::Char('e') if is_export_key(key) => {
                let can_export = should_export_list(self.lookups.as_ref());
                handle_list_export(self, can_export, ExportTarget::Lookups)
            }

            // Download selected lookup (Ctrl+D or 'd')
            KeyCode::Char('d')
                if key.modifiers.is_empty() || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.selected_lookup_download_action()
            }

            // Delete selected lookup (Ctrl+X or 'x')
            KeyCode::Char('x')
                if key.modifiers.is_empty() || key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                self.selected_lookup_delete_action()
            }

            // Load more (if available)
            KeyCode::Char('n') if self.lookups_pagination.can_load_more() => {
                Some(Action::LoadMoreLookups)
            }

            _ => None,
        }
    }

    fn selected_lookup(&self) -> Option<&LookupTable> {
        self.lookups.as_ref().and_then(|lookups| {
            self.lookups_state
                .selected()
                .and_then(|index| lookups.get(index))
        })
    }

    fn selected_lookup_download_action(&self) -> Option<Action> {
        self.selected_lookup().map(|lookup| Action::DownloadLookup {
            name: lookup.name.clone(),
            app: None,
            owner: None,
            output_path: std::path::PathBuf::from(format!("{}.csv", lookup.name)),
        })
    }

    fn selected_lookup_delete_action(&self) -> Option<Action> {
        self.selected_lookup()
            .map(|lookup| Action::OpenDeleteLookupConfirm {
                name: lookup.name.clone(),
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::state::CurrentScreen;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn lookup(name: &str, size: usize) -> LookupTable {
        LookupTable {
            name: name.to_string(),
            filename: format!("{name}.csv"),
            owner: "admin".to_string(),
            app: "search".to_string(),
            sharing: "app".to_string(),
            size,
        }
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_lookups_navigation_down() {
        let mut app = App::default();
        app.current_screen = CurrentScreen::Lookups;
        app.lookups = Some(vec![lookup("lookup1", 1024), lookup("lookup2", 2048)]);
        app.lookups_state.select(Some(0));

        // Press 'j' to go down
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_lookups_input(key);

        assert_eq!(app.lookups_state.selected(), Some(1));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_lookups_navigation_up() {
        let mut app = App::default();
        app.current_screen = CurrentScreen::Lookups;
        app.lookups = Some(vec![lookup("lookup1", 1024), lookup("lookup2", 2048)]);
        app.lookups_state.select(Some(1));

        // Press 'k' to go up
        let key = KeyEvent::new(KeyCode::Char('k'), KeyModifiers::NONE);
        app.handle_lookups_input(key);

        assert_eq!(app.lookups_state.selected(), Some(0));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_lookups_refresh() {
        let mut app = App::default();
        app.current_screen = CurrentScreen::Lookups;

        // Press 'r' to refresh
        let key = KeyEvent::new(KeyCode::Char('r'), KeyModifiers::NONE);
        let action = app.handle_lookups_input(key);

        assert!(matches!(
            action,
            Some(Action::LoadLookups {
                count: _,
                offset: 0
            })
        ));
        assert!(app.loading);
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_lookups_export() {
        let mut app = App::default();
        app.current_screen = CurrentScreen::Lookups;
        app.lookups = Some(vec![lookup("lookup1", 1024)]);

        // Press Ctrl+e to export
        let key = ctrl_key('e');
        app.handle_lookups_input(key);

        assert_eq!(app.export_target, Some(ExportTarget::Lookups));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_lookups_export_requires_non_empty_collection() {
        let mut app = App::default();
        app.current_screen = CurrentScreen::Lookups;
        app.lookups = Some(vec![]);

        let key = ctrl_key('e');
        let action = app.handle_lookups_input(key);

        assert!(action.is_none());
        assert!(app.popup.is_none());
        assert!(app.export_target.is_none());
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_lookups_copy() {
        let mut app = App::default();
        app.current_screen = CurrentScreen::Lookups;
        app.lookups = Some(vec![lookup("lookup1", 1024)]);
        app.lookups_state.select(Some(0));

        // Press Ctrl+c to copy
        let key = ctrl_key('c');
        let action = app.handle_lookups_input(key);

        assert!(
            matches!(action, Some(Action::CopyToClipboard(ref s)) if s == "lookup1"),
            "Expected CopyToClipboard action with 'lookup1'"
        );
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_lookups_download_shortcuts_share_behavior() {
        let mut app = App::default();
        app.current_screen = CurrentScreen::Lookups;
        app.lookups = Some(vec![lookup("lookup1", 1024)]);
        app.lookups_state.select(Some(0));

        let plain = app.handle_lookups_input(KeyEvent::new(KeyCode::Char('d'), KeyModifiers::NONE));
        let ctrl = app.handle_lookups_input(ctrl_key('d'));

        assert!(matches!(
            plain,
            Some(Action::DownloadLookup {
                ref name,
                app: None,
                owner: None,
                ref output_path,
            }) if name == "lookup1" && output_path == &std::path::PathBuf::from("lookup1.csv")
        ));
        assert!(matches!(
            ctrl,
            Some(Action::DownloadLookup {
                ref name,
                app: None,
                owner: None,
                ref output_path,
            }) if name == "lookup1" && output_path == &std::path::PathBuf::from("lookup1.csv")
        ));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_lookups_delete_shortcuts_share_behavior() {
        let mut app = App::default();
        app.current_screen = CurrentScreen::Lookups;
        app.lookups = Some(vec![lookup("lookup1", 1024)]);
        app.lookups_state.select(Some(0));

        let plain = app.handle_lookups_input(KeyEvent::new(KeyCode::Char('x'), KeyModifiers::NONE));
        let ctrl = app.handle_lookups_input(ctrl_key('x'));

        assert!(matches!(
            plain,
            Some(Action::OpenDeleteLookupConfirm { ref name }) if name == "lookup1"
        ));
        assert!(matches!(
            ctrl,
            Some(Action::OpenDeleteLookupConfirm { ref name }) if name == "lookup1"
        ));
    }
}
