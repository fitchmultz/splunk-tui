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
        match key.code {
            // Navigation
            KeyCode::Down | KeyCode::Char('j') => {
                let next = self.lookups_state.selected().map(|i| i + 1).unwrap_or(0);
                let max = self.lookups.as_ref().map(|l| l.len()).unwrap_or(0);
                if next < max {
                    self.lookups_state.select(Some(next));
                }
                None
            }
            KeyCode::Up | KeyCode::Char('k') => {
                let prev = self
                    .lookups_state
                    .selected()
                    .map(|i| i.saturating_sub(1))
                    .unwrap_or(0);
                self.lookups_state.select(Some(prev));
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
            KeyCode::Char('e') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                self.begin_export(ExportTarget::Lookups);
                None
            }

            // Copy selected lookup name (Ctrl+C or 'y' vim-style)
            KeyCode::Char('c') if key.modifiers.contains(KeyModifiers::CONTROL) => {
                if let Some(index) = self.lookups_state.selected()
                    && let Some(lookups) = &self.lookups
                    && let Some(lookup) = lookups.get(index)
                {
                    return Some(Action::CopyToClipboard(lookup.name.clone()));
                }
                None
            }
            KeyCode::Char('y') if key.modifiers.is_empty() => {
                if let Some(index) = self.lookups_state.selected()
                    && let Some(lookups) = &self.lookups
                    && let Some(lookup) = lookups.get(index)
                {
                    return Some(Action::CopyToClipboard(lookup.name.clone()));
                }
                None
            }

            // Load more (if available)
            KeyCode::Char('n') if self.lookups_pagination.can_load_more() => {
                Some(Action::LoadMoreLookups)
            }

            _ => None,
        }
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

    #[test]
    fn test_lookups_navigation_down() {
        let mut app = App::default();
        app.current_screen = CurrentScreen::Lookups;
        app.lookups = Some(vec![
            LookupTable {
                name: "lookup1".to_string(),
                filename: "lookup1.csv".to_string(),
                owner: "admin".to_string(),
                app: "search".to_string(),
                sharing: "app".to_string(),
                size: 1024,
            },
            LookupTable {
                name: "lookup2".to_string(),
                filename: "lookup2.csv".to_string(),
                owner: "admin".to_string(),
                app: "search".to_string(),
                sharing: "app".to_string(),
                size: 2048,
            },
        ]);
        app.lookups_state.select(Some(0));

        // Press 'j' to go down
        let key = KeyEvent::new(KeyCode::Char('j'), KeyModifiers::NONE);
        app.handle_lookups_input(key);

        assert_eq!(app.lookups_state.selected(), Some(1));
    }

    #[test]
    fn test_lookups_navigation_up() {
        let mut app = App::default();
        app.current_screen = CurrentScreen::Lookups;
        app.lookups = Some(vec![
            LookupTable {
                name: "lookup1".to_string(),
                filename: "lookup1.csv".to_string(),
                owner: "admin".to_string(),
                app: "search".to_string(),
                sharing: "app".to_string(),
                size: 1024,
            },
            LookupTable {
                name: "lookup2".to_string(),
                filename: "lookup2.csv".to_string(),
                owner: "admin".to_string(),
                app: "search".to_string(),
                sharing: "app".to_string(),
                size: 2048,
            },
        ]);
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
        app.lookups = Some(vec![LookupTable {
            name: "lookup1".to_string(),
            filename: "lookup1.csv".to_string(),
            owner: "admin".to_string(),
            app: "search".to_string(),
            sharing: "app".to_string(),
            size: 1024,
        }]);

        // Press Ctrl+e to export
        let key = ctrl_key('e');
        app.handle_lookups_input(key);

        assert_eq!(app.export_target, Some(ExportTarget::Lookups));
    }

    #[test]
    #[allow(clippy::field_reassign_with_default)]
    fn test_lookups_copy() {
        let mut app = App::default();
        app.current_screen = CurrentScreen::Lookups;
        app.lookups = Some(vec![LookupTable {
            name: "lookup1".to_string(),
            filename: "lookup1.csv".to_string(),
            owner: "admin".to_string(),
            app: "search".to_string(),
            sharing: "app".to_string(),
            size: 1024,
        }]);
        app.lookups_state.select(Some(0));

        // Press Ctrl+c to copy
        let key = ctrl_key('c');
        let action = app.handle_lookups_input(key);

        assert!(
            matches!(action, Some(Action::CopyToClipboard(ref s)) if s == "lookup1"),
            "Expected CopyToClipboard action with 'lookup1'"
        );
    }

    use splunk_client::models::LookupTable;
}
