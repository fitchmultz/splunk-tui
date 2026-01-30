//! Fired alerts screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C copy of selected alert name
//! - Handle Ctrl+E export of fired alerts list
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch fired alerts data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the fired alerts screen.
    pub fn handle_fired_alerts_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy selected alert name
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self.fired_alerts.as_ref().and_then(|alerts| {
                self.fired_alerts_state
                    .selected()
                    .and_then(|i| alerts.get(i))
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
                    && self
                        .fired_alerts
                        .as_ref()
                        .map(|v| !v.is_empty())
                        .unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::FiredAlerts);
                None
            }
            _ => None,
        }
    }
}
