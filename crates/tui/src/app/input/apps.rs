//! Apps screen input handler.
//!
//! Responsibilities:
//! - Handle 'e' key to enable selected app (if disabled)
//! - Handle 'd' key to disable selected app (if enabled)
//! - Handle Ctrl+C copy of selected app name
//! - Handle Ctrl+E export of apps list
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT execute enable/disable operations (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::popup::{Popup, PopupType};
use crate::ui::{Toast, ToastLevel};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the apps screen.
    pub fn handle_apps_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy selected app name
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self.apps.as_ref().and_then(|apps| {
                self.apps_state
                    .selected()
                    .and_then(|i| apps.get(i))
                    .map(|a| a.name.clone())
            });

            if let Some(content) = content.filter(|s| !s.trim().is_empty()) {
                return Some(Action::CopyToClipboard(content));
            }

            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && self.apps.as_ref().map(|v| !v.is_empty()).unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::Apps);
                None
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
            _ => None,
        }
    }
}
