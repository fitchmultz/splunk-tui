//! Miscellaneous popup handlers.
//!
//! Responsibilities:
//! - Handle Help popup (scrolling, closing)
//! - Handle ErrorDetails popup (scrolling, closing)
//! - Handle InstallAppDialog popup (file input)
//!
//! Does NOT handle:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT install apps (just returns Action::InstallApp)

use crate::action::Action;
use crate::app::App;
use crate::error_details::AuthRecoveryKind;
use crate::ui::popup::{Popup, PopupType};
use crate::ux_telemetry::AuthRecoveryAction;
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

    /// Handle AuthRecovery popup.
    pub fn handle_auth_recovery_popup(&mut self, key: KeyEvent) -> Option<Action> {
        // Get the recovery kind from the current popup if available
        let recovery_kind = self.get_auth_recovery_kind();

        match key.code {
            // Close popup
            KeyCode::Esc | KeyCode::Char('q') => {
                self.popup = None;
                // Record dismiss action
                if let (Some(collector), Some(kind)) = (&self.ux_telemetry, recovery_kind) {
                    collector.record_auth_recovery_action(kind, AuthRecoveryAction::Dismiss, false);
                }
                None
            }
            // Retry current screen load
            KeyCode::Char('r') => {
                self.popup = None;
                // Record retry action (success will be determined by data load result)
                if let (Some(collector), Some(kind)) = (&self.ux_telemetry, recovery_kind) {
                    collector.record_auth_recovery_action(kind, AuthRecoveryAction::Retry, true);
                }
                self.load_action_for_screen()
            }
            // Open profile selector
            KeyCode::Char('p') => {
                self.popup = None;
                if let (Some(collector), Some(kind)) = (&self.ux_telemetry, recovery_kind) {
                    collector.record_auth_recovery_action(
                        kind,
                        AuthRecoveryAction::SwitchProfile,
                        true,
                    );
                }
                Some(Action::OpenProfileSwitcher)
            }
            // Open create profile dialog
            KeyCode::Char('n') => {
                self.popup = None;
                if let (Some(collector), Some(kind)) = (&self.ux_telemetry, recovery_kind) {
                    collector.record_auth_recovery_action(
                        kind,
                        AuthRecoveryAction::CreateProfile,
                        true,
                    );
                }
                Some(Action::OpenCreateProfileDialog {
                    from_tutorial: false,
                })
            }
            // Show raw error details
            KeyCode::Char('e') => {
                if let (Some(collector), Some(kind)) = (&self.ux_telemetry, recovery_kind) {
                    collector.record_auth_recovery_action(
                        kind,
                        AuthRecoveryAction::ViewError,
                        true,
                    );
                }
                // Keep popup open but switch to error details
                self.popup = Some(Popup::builder(PopupType::ErrorDetails).build());
                None
            }
            _ => None,
        }
    }

    /// Helper to extract AuthRecoveryKind from current popup
    fn get_auth_recovery_kind(&self) -> Option<AuthRecoveryKind> {
        if let Some(ref popup) = self.popup {
            if let PopupType::AuthRecovery { kind } = popup.kind.clone() {
                return Some(kind);
            }
        }
        None
    }
}
