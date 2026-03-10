//! Apps screen input handler.
//!
//! Responsibilities:
//! - Handle 'e' key to enable selected app (if disabled)
//! - Handle 'd' key to disable selected app (if enabled)
//! - Handle 'i' key to install app from .spl file
//! - Handle 'x' key to remove selected app (with confirmation)
//! - Handle Ctrl+C copy of selected app name
//! - Handle Ctrl+E export of apps list
//!
//! Does NOT handle:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT execute enable/disable/install/remove operations (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::input::helpers::{
    handle_copy_with_toast, handle_list_export, is_copy_key, is_export_key, should_export_list,
};
use crate::ui::ToastLevel;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle input for the apps screen.
    pub fn handle_apps_input(&mut self, key: KeyEvent) -> Option<Action> {
        if is_copy_key(key) {
            let content = self.apps.as_ref().and_then(|apps| {
                self.apps_state
                    .selected()
                    .and_then(|i| apps.get(i))
                    .map(|a| a.name.clone())
            });

            return handle_copy_with_toast(self, content);
        }

        match key.code {
            KeyCode::Char('e') if is_export_key(key) => {
                let can_export = should_export_list(self.apps.as_ref());
                handle_list_export(self, can_export, ExportTarget::Apps)
            }
            KeyCode::Char('e') => {
                // Enable selected app (if disabled)
                if let Some(app) = self
                    .apps
                    .as_ref()
                    .and_then(|apps| self.apps_state.selected().and_then(|i| apps.get(i)))
                {
                    if app.disabled {
                        self.popup = Some(
                            Popup::builder(PopupType::ConfirmEnableApp(app.name.clone())).build(),
                        );
                    } else {
                        // App is already enabled, show info toast
                        return Some(Action::Notify(
                            ToastLevel::Info,
                            format!("App '{}' is already enabled", app.name),
                        ));
                    }
                }
                None
            }
            KeyCode::Char('d') => {
                // Disable selected app (if enabled)
                if let Some(app) = self
                    .apps
                    .as_ref()
                    .and_then(|apps| self.apps_state.selected().and_then(|i| apps.get(i)))
                {
                    if !app.disabled {
                        self.popup = Some(
                            Popup::builder(PopupType::ConfirmDisableApp(app.name.clone())).build(),
                        );
                    } else {
                        // App is already disabled, show info toast
                        return Some(Action::Notify(
                            ToastLevel::Info,
                            format!("App '{}' is already disabled", app.name),
                        ));
                    }
                }
                None
            }
            KeyCode::Char('i') => {
                // Open install app dialog
                self.popup = Some(
                    Popup::builder(PopupType::InstallAppDialog {
                        file_input: String::new(),
                    })
                    .build(),
                );
                None
            }
            KeyCode::Char('x') => {
                // Remove selected app (with confirmation)
                if let Some(app) = self
                    .apps
                    .as_ref()
                    .and_then(|apps| self.apps_state.selected().and_then(|i| apps.get(i)))
                {
                    self.popup =
                        Some(Popup::builder(PopupType::ConfirmRemoveApp(app.name.clone())).build());
                }
                None
            }
            _ => None,
        }
    }
}
