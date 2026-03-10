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
use crate::app::input::helpers::{
    handle_copy_with_toast, handle_list_export, is_copy_key, is_export_key, should_export_list,
};
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle input for the roles screen.
    pub fn handle_roles_input(&mut self, key: KeyEvent) -> Option<Action> {
        if is_copy_key(key) {
            let content = self.roles.as_ref().and_then(|roles| {
                self.roles_state
                    .selected()
                    .and_then(|i| roles.get(i))
                    .map(|r| r.name.clone())
            });

            return handle_copy_with_toast(self, content);
        }

        match key.code {
            KeyCode::Char('e') if is_export_key(key) => {
                let can_export = should_export_list(self.roles.as_ref());
                handle_list_export(self, can_export, ExportTarget::Roles)
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
