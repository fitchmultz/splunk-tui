//! Miscellaneous popup handlers.
//!
//! Responsibilities:
//! - Handle Help popup (scrolling, closing)
//! - Handle ErrorDetails popup (scrolling, closing)
//! - Handle InstallAppDialog popup (file input)
//!
//! Non-responsibilities:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT install apps (just returns Action::InstallApp)

use crate::action::Action;
use crate::app::App;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle Help popup.
    pub fn handle_help_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?') => {
                self.popup = None;
                self.help_scroll_offset = 0;
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.help_scroll_offset = self.help_scroll_offset.saturating_add(1);
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.help_scroll_offset = self.help_scroll_offset.saturating_sub(1);
                None
            }
            KeyCode::PageDown => {
                self.help_scroll_offset = self.help_scroll_offset.saturating_add(10);
                None
            }
            KeyCode::PageUp => {
                self.help_scroll_offset = self.help_scroll_offset.saturating_sub(10);
                None
            }
            _ => None,
        }
    }

    /// Handle ErrorDetails popup.
    pub fn handle_error_details_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match key.code {
            KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('e') => {
                self.popup = None;
                None
            }
            KeyCode::Char('j') | KeyCode::Down => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_add(1);
                None
            }
            KeyCode::Char('k') | KeyCode::Up => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_sub(1);
                None
            }
            KeyCode::PageDown => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_add(10);
                None
            }
            KeyCode::PageUp => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_sub(10);
                None
            }
            _ => None,
        }
    }

    /// Handle InstallAppDialog popup.
    pub fn handle_install_app_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match (self.popup.as_ref().map(|p| &p.kind), key.code) {
            (Some(PopupType::InstallAppDialog { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (Some(PopupType::InstallAppDialog { file_input }), KeyCode::Enter) => {
                if file_input.is_empty() {
                    return None;
                }
                let path = std::path::PathBuf::from(file_input);
                self.popup = None;
                Some(Action::InstallApp { file_path: path })
            }
            (Some(PopupType::InstallAppDialog { file_input }), KeyCode::Char(c)) => {
                let mut new_input = file_input.clone();
                new_input.push(c);
                self.popup = Some(
                    Popup::builder(PopupType::InstallAppDialog {
                        file_input: new_input,
                    })
                    .build(),
                );
                None
            }
            (Some(PopupType::InstallAppDialog { file_input }), KeyCode::Backspace) => {
                let mut new_input = file_input.clone();
                new_input.pop();
                self.popup = Some(
                    Popup::builder(PopupType::InstallAppDialog {
                        file_input: new_input,
                    })
                    .build(),
                );
                None
            }
            _ => None,
        }
    }
}
