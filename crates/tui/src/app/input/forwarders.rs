//! Input handling for the forwarders screen.
//!
//! Responsibilities:
//! - Handle keyboard input for the forwarders screen
//! - Dispatch actions based on key presses
//!
//! Does NOT handle:
//! - Rendering (handled by screen module)
//! - Data fetching (handled by side effects)

use crossterm::event::{KeyCode, KeyEvent};

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::input::helpers::{handle_list_export, is_export_key, should_export_list};

impl App {
    /// Handle keyboard input for the forwarders screen.
    ///
    /// # Arguments
    /// * `key` - The key event to process
    ///
    /// # Returns
    /// * `Some(Action)` - Action to execute
    /// * `None` - No action to execute
    pub fn handle_forwarders_input(&mut self, key: KeyEvent) -> Option<Action> {
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
                Some(Action::LoadForwarders {
                    count: self.forwarders_pagination.page_size,
                    offset: 0,
                })
            }

            // Export
            KeyCode::Char('e') if is_export_key(key) => {
                let can_export = should_export_list(self.forwarders.as_ref());
                handle_list_export(self, can_export, ExportTarget::Forwarders)
            }

            // Load more (if available)
            KeyCode::Char('n') if self.forwarders_pagination.can_load_more() => {
                Some(Action::LoadMoreForwarders)
            }

            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::ConnectionContext;
    use crate::ui::popup::PopupType;

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), crossterm::event::KeyModifiers::CONTROL)
    }

    #[test]
    fn test_forwarders_export_requires_non_empty_collection() {
        let mut app = App::new(None, ConnectionContext::default());
        app.forwarders = Some(vec![]);

        let action = app.handle_forwarders_input(ctrl_key('e'));

        assert!(action.is_none());
        assert!(app.popup.is_none());
        assert!(app.export_target.is_none());
    }

    #[test]
    fn test_forwarders_export_opens_popup_when_data_is_present() {
        let mut app = App::new(None, ConnectionContext::default());
        app.forwarders = Some(vec![
            serde_json::from_value(serde_json::json!({
                "guid": "peer-1",
                "hostname": "uf01",
                "status": "up"
            }))
            .unwrap(),
        ]);

        let action = app.handle_forwarders_input(ctrl_key('e'));

        assert!(action.is_none());
        assert_eq!(app.export_target, Some(ExportTarget::Forwarders));
        assert!(matches!(
            app.popup.as_ref().map(|popup| &popup.kind),
            Some(PopupType::ExportSearch)
        ));
    }
}
