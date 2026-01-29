//! Health screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C copy of health status or server name
//! - Handle Ctrl+E export of health info
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch health data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the health screen.
    pub fn handle_health_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy health status
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self.health_info.as_ref().and_then(|h| {
                h.splunkd_health
                    .as_ref()
                    .map(|sh| sh.health.clone())
                    .or_else(|| h.server_info.as_ref().map(|s| s.server_name.clone()))
            });

            if let Some(content) = content {
                return Some(Action::CopyToClipboard(content));
            }
            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL) && self.health_info.is_some() =>
            {
                self.begin_export(ExportTarget::Health);
                None
            }
            _ => None,
        }
    }
}
