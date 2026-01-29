//! Internal logs screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C copy of selected log message
//! - Handle Ctrl+E export of internal logs
//! - Handle auto-refresh toggle (a key)
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch internal logs data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the internal logs screen.
    pub fn handle_internal_logs_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy selected log message
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self.internal_logs.as_ref().and_then(|logs| {
                self.internal_logs_state
                    .selected()
                    .and_then(|i| logs.get(i))
                    .map(|l| l.message.clone())
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
                    && self
                        .internal_logs
                        .as_ref()
                        .map(|v| !v.is_empty())
                        .unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::InternalLogs);
                None
            }
            KeyCode::Char('a') => {
                self.auto_refresh = !self.auto_refresh;
                None
            }
            _ => None,
        }
    }
}
