//! Popup input handling for the TUI app.
//!
//! Responsibilities:
//! - Handle keyboard input when popups are active
//! - Manage export popup state and input
//!
//! Non-responsibilities:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT define popup types (handled by ui::popup module)

use crate::action::{Action, ExportFormat};
use crate::app::App;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle keyboard input when a popup is active.
    pub fn handle_popup_input(&mut self, key: KeyEvent) -> Option<Action> {
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
            (Some(PopupType::ConfirmCancelBatch(sids)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sids = sids.clone();
                self.popup = None;
                Some(Action::CancelJobsBatch(sids))
            }
            (Some(PopupType::ConfirmDeleteBatch(sids)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sids = sids.clone();
                self.popup = None;
                Some(Action::DeleteJobsBatch(sids))
            }
            (Some(PopupType::ConfirmEnableApp(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let name = if let Some(Popup {
                    kind: PopupType::ConfirmEnableApp(n),
                    ..
                }) = self.popup.take()
                {
                    n
                } else {
                    unreachable!()
                };
                Some(Action::EnableApp(name))
            }
            (Some(PopupType::ConfirmDisableApp(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let name = if let Some(Popup {
                    kind: PopupType::ConfirmDisableApp(n),
                    ..
                }) = self.popup.take()
                {
                    n
                } else {
                    unreachable!()
                };
                Some(Action::DisableApp(name))
            }
            (Some(PopupType::ExportSearch), KeyCode::Esc) => {
                self.popup = None;
                self.export_target = None;
                None
            }
            (Some(PopupType::ExportSearch), KeyCode::Enter) => {
                if self.export_input.is_empty() {
                    return None;
                }

                if let Some(data) = self.collect_export_data() {
                    let path = std::path::PathBuf::from(&self.export_input);
                    let format = self.export_format;
                    self.popup = None;
                    self.export_target = None;
                    Some(Action::ExportData(data, path, format))
                } else {
                    None
                }
            }
            (Some(PopupType::ExportSearch), KeyCode::Tab) => {
                self.export_format = match self.export_format {
                    ExportFormat::Json => ExportFormat::Csv,
                    ExportFormat::Csv => ExportFormat::Json,
                };
                // Automatically update extension if it matches the previous format
                match self.export_format {
                    ExportFormat::Json => {
                        if self.export_input.ends_with(".csv") {
                            self.export_input.truncate(self.export_input.len() - 4);
                            self.export_input.push_str(".json");
                        }
                    }
                    ExportFormat::Csv => {
                        if self.export_input.ends_with(".json") {
                            self.export_input.truncate(self.export_input.len() - 5);
                            self.export_input.push_str(".csv");
                        }
                    }
                }
                self.update_export_popup();
                None
            }
            (Some(PopupType::ExportSearch), KeyCode::Backspace) => {
                self.export_input.pop();
                self.update_export_popup();
                None
            }
            (Some(PopupType::ExportSearch), KeyCode::Char(c)) => {
                self.export_input.push(c);
                self.update_export_popup();
                None
            }
            (
                Some(PopupType::ErrorDetails),
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('e'),
            ) => {
                self.popup = None;
                None
            }
            (Some(PopupType::ErrorDetails), KeyCode::Char('j') | KeyCode::Down) => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_add(1);
                None
            }
            (Some(PopupType::ErrorDetails), KeyCode::Char('k') | KeyCode::Up) => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_sub(1);
                None
            }
            (Some(PopupType::ErrorDetails), KeyCode::PageDown) => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_add(10);
                None
            }
            (Some(PopupType::ErrorDetails), KeyCode::PageUp) => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_sub(10);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::Esc | KeyCode::Char('q')) => {
                self.popup = None;
                self.index_details_scroll_offset = 0;
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::Char('j') | KeyCode::Down) => {
                self.index_details_scroll_offset =
                    self.index_details_scroll_offset.saturating_add(1);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::Char('k') | KeyCode::Up) => {
                self.index_details_scroll_offset =
                    self.index_details_scroll_offset.saturating_sub(1);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::PageDown) => {
                self.index_details_scroll_offset =
                    self.index_details_scroll_offset.saturating_add(10);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::PageUp) => {
                self.index_details_scroll_offset =
                    self.index_details_scroll_offset.saturating_sub(10);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::Char('c'))
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                // Copy index JSON to clipboard
                if let Some(indexes) = &self.indexes
                    && let Some(selected) = self.indexes_state.selected()
                    && let Some(index) = indexes.get(selected)
                    && let Ok(json) = serde_json::to_string_pretty(index)
                {
                    return Some(Action::CopyToClipboard(json));
                }
                None
            }
            (
                Some(
                    PopupType::ConfirmCancel(_)
                    | PopupType::ConfirmDelete(_)
                    | PopupType::ConfirmCancelBatch(_)
                    | PopupType::ConfirmDeleteBatch(_)
                    | PopupType::ConfirmEnableApp(_)
                    | PopupType::ConfirmDisableApp(_),
                ),
                KeyCode::Char('n') | KeyCode::Esc,
            ) => {
                self.popup = None;
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::empty())
    }

    #[test]
    fn test_popup_help_close() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::Help).build());

        // Close with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());

        // Reopen and close with 'q'
        app.popup = Some(Popup::builder(PopupType::Help).build());
        let action = app.handle_popup_input(key(KeyCode::Char('q')));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_confirm_cancel() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());

        // Confirm with 'y'
        let action = app.handle_popup_input(key(KeyCode::Char('y')));
        assert!(matches!(action, Some(Action::CancelJob(sid)) if sid == "test-sid"));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_confirm_cancel_with_enter() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());

        // Confirm with Enter
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(matches!(action, Some(Action::CancelJob(sid)) if sid == "test-sid"));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_confirm_cancel_reject() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());

        // Reject with 'n'
        let action = app.handle_popup_input(key(KeyCode::Char('n')));
        assert!(action.is_none());
        assert!(app.popup.is_none());

        // Reopen and reject with Esc
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_export_search_input() {
        use crate::app::export::ExportTarget;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ExportSearch).build());
        app.export_target = Some(ExportTarget::SearchResults);
        app.export_input = String::new();

        // Type some characters
        app.handle_popup_input(key(KeyCode::Char('t')));
        app.handle_popup_input(key(KeyCode::Char('e')));
        app.handle_popup_input(key(KeyCode::Char('s')));
        app.handle_popup_input(key(KeyCode::Char('t')));

        assert_eq!(app.export_input, "test");

        // Backspace
        app.handle_popup_input(key(KeyCode::Backspace));
        assert_eq!(app.export_input, "tes");
    }

    #[test]
    fn test_popup_export_search_format_toggle() {
        use crate::app::export::ExportTarget;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ExportSearch).build());
        app.export_target = Some(ExportTarget::SearchResults);
        app.export_input = "test.json".to_string();
        app.export_format = ExportFormat::Json;

        // Toggle format with Tab
        app.handle_popup_input(key(KeyCode::Tab));
        assert_eq!(app.export_format, ExportFormat::Csv);
        assert_eq!(app.export_input, "test.csv");

        // Toggle back
        app.handle_popup_input(key(KeyCode::Tab));
        assert_eq!(app.export_format, ExportFormat::Json);
        assert_eq!(app.export_input, "test.json");
    }

    #[test]
    fn test_popup_export_search_cancel() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ExportSearch).build());
        app.export_target = Some(crate::app::export::ExportTarget::SearchResults);

        // Cancel with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
        assert!(app.export_target.is_none());
    }

    #[test]
    fn test_popup_error_details_navigation() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ErrorDetails).build());
        app.error_scroll_offset = 0;

        // Scroll down
        app.handle_popup_input(key(KeyCode::Char('j')));
        assert_eq!(app.error_scroll_offset, 1);

        // Scroll down more
        app.handle_popup_input(key(KeyCode::Down));
        assert_eq!(app.error_scroll_offset, 2);

        // Page down
        app.handle_popup_input(key(KeyCode::PageDown));
        assert_eq!(app.error_scroll_offset, 12);

        // Scroll up
        app.handle_popup_input(key(KeyCode::Char('k')));
        assert_eq!(app.error_scroll_offset, 11);

        // Page up
        app.handle_popup_input(key(KeyCode::PageUp));
        assert_eq!(app.error_scroll_offset, 1);

        // Close
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }
}
