//! Users screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C or 'y' copy of selected username (vim-style)
//! - Handle Ctrl+E export of users list
//! - Handle 'c' to open create user dialog
//! - Handle 'm' to open modify user dialog
//! - Handle 'd' to open delete user confirmation
//!
//! Does NOT handle:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch users data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::app::input::helpers::{
    handle_copy_with_toast, handle_list_export, is_copy_key, is_export_key, should_export_list,
};
use crate::ui::Toast;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle input for the users screen.
    pub fn handle_users_input(&mut self, key: KeyEvent) -> Option<Action> {
        if is_copy_key(key) {
            let content = self.users.as_ref().and_then(|users| {
                self.users_state
                    .selected()
                    .and_then(|i| users.get(i))
                    .map(|u| u.name.clone())
            });

            return handle_copy_with_toast(self, content);
        }

        match key.code {
            KeyCode::Char('e') if is_export_key(key) => {
                let can_export = should_export_list(self.users.as_ref());
                handle_list_export(self, can_export, ExportTarget::Users)
            }
            KeyCode::Char('c') => {
                // Open create user dialog
                Some(Action::OpenCreateUserDialog)
            }
            KeyCode::Char('m') => {
                // Open modify user dialog for selected user
                if let Some(users) = &self.users
                    && let Some(selected) = self.users_state.selected()
                    && let Some(user) = users.get(selected)
                {
                    return Some(Action::OpenModifyUserDialog {
                        name: user.name.clone(),
                    });
                }
                self.toasts.push(Toast::info("No user selected"));
                None
            }
            KeyCode::Char('d') => {
                // Open delete user confirmation for selected user
                if let Some(users) = &self.users
                    && let Some(selected) = self.users_state.selected()
                    && let Some(user) = users.get(selected)
                {
                    return Some(Action::OpenDeleteUserConfirm {
                        name: user.name.clone(),
                    });
                }
                self.toasts.push(Toast::info("No user selected"));
                None
            }
            _ => None,
        }
    }
}
