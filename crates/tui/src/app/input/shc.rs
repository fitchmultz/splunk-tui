//! SHC screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C or 'y' copy of SHC info (vim-style)
//! - Handle Ctrl+E export of SHC info
//! - Handle SHC management actions (rolling restart, set captain)
//!
//! Does NOT handle:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch SHC data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the SHC screen.
    pub fn handle_shc_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C or 'y': copy captain URI if available (vim-style)
        let is_copy = (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c')))
            || (key.modifiers.is_empty() && matches!(key.code, KeyCode::Char('y')));
        if is_copy {
            if let Some(status) = &self.shc_status
                && let Some(uri) = &status.captain_uri
            {
                return Some(Action::CopyToClipboard(uri.clone()));
            }
            self.toasts.push(Toast::info("Nothing to copy"));
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL) && self.shc_status.is_some() =>
            {
                self.begin_export(ExportTarget::ShcStatus);
                None
            }
            // Rolling restart (r)
            KeyCode::Char('r') => Some(Action::RollingRestartShc { force: false }),
            // Force rolling restart (R)
            KeyCode::Char('R') => Some(Action::RollingRestartShc { force: true }),
            // Set selected member as captain (s) - only in Members view
            KeyCode::Char('s') => {
                if let Some(member) = self.get_selected_shc_member() {
                    return Some(Action::SetShcCaptain {
                        member_guid: member.guid.clone(),
                    });
                }
                self.toasts.push(Toast::info("No member selected"));
                None
            }
            // Remove member (x) - only in Members view
            KeyCode::Char('x') => {
                if let Some(member) = self.get_selected_shc_member() {
                    return Some(Action::RemoveShcMember {
                        member_guid: member.guid.clone(),
                    });
                }
                self.toasts.push(Toast::info("No member selected"));
                None
            }
            _ => None,
        }
    }
}
