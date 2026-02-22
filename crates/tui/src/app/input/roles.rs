//! Roles screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C or 'y' copy of selected role name (vim-style)
//! - Handle Ctrl+E export of roles list
//! - Handle 'c' to open create role dialog
//! - Handle 'm' to open modify role dialog
//! - Handle 'd' to open delete role confirmation
//!
//! Does NOT handle:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch roles data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the roles screen.
    pub fn handle_roles_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C or 'y': copy selected role name (vim-style)
        let is_copy = (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c')))
            || (key.modifiers.is_empty() && matches!(key.code, KeyCode::Char('y')));
        if is_copy {
            let content = self.roles.as_ref().and_then(|roles| {
                self.roles_state
                    .selected()
                    .and_then(|i| roles.get(i))
                    .map(|r| r.name.clone())
            });

            if let Some(content) = content.filter(|s| !s.trim().is_empty()) {
                return Some(Action::CopyToClipboard(content));
            }

            self.push_info_toast_once("Nothing to copy");
            return None;
        }

        match key.code {
            KeyCode::Char('e')
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && self.roles.as_ref().map(|v| !v.is_empty()).unwrap_or(false) =>
            {
                self.begin_export(ExportTarget::Roles);
                None
            }
            KeyCode::Char('c') => {
                // Open create role dialog
                Some(Action::OpenCreateRoleDialog)
            }
            KeyCode::Char('m') => {
                // Open modify role dialog for selected role
                if let Some(roles) = &self.roles
                    && let Some(selected) = self.roles_state.selected()
                    && let Some(role) = roles.get(selected)
                {
                    return Some(Action::OpenModifyRoleDialog {
                        name: role.name.clone(),
                    });
                }
                self.toasts.push(Toast::info("No role selected"));
                None
            }
            KeyCode::Char('d') => {
                // Open delete role confirmation for selected role
                if let Some(roles) = &self.roles
                    && let Some(selected) = self.roles_state.selected()
                    && let Some(role) = roles.get(selected)
                {
                    return Some(Action::OpenDeleteRoleConfirm {
                        name: role.name.clone(),
                    });
                }
                self.toasts.push(Toast::info("No role selected"));
                None
            }
            _ => None,
        }
    }
}
