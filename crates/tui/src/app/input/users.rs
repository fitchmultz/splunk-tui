//! Users screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C copy of selected username
//! - Handle Ctrl+E export of users list
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch users data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the users screen.
    pub fn handle_users_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C: copy selected username
        if key.modifiers.contains(KeyModifiers::CONTROL) && matches!(key.code, KeyCode::Char('c')) {
            let content = self.users.as_ref().and_then(|users| {
                self.users_state
                    .selected()
                    .and_then(|i| users.get(i))
                    .map(|u| u.name.clone())
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
                    && self.users.as_ref().map(|v| !v.is_empty()).unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::Users);
                None
            }
            _ => None,
        }
    }
}
