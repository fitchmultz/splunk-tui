//! Users screen input handler.
//!
//! Responsibilities:
//! - Handle Ctrl+C or 'y' copy of selected username (vim-style)
//! - Handle Ctrl+E export of users list
//! - Handle 'c' to open create user dialog
//! - Handle 'm' to open modify user dialog
//! - Handle 'd' to open delete user confirmation
//!
//! Non-responsibilities:
//! - Does NOT handle global navigation (handled by keymap)
//! - Does NOT render the UI (handled by render module)
//! - Does NOT fetch users data (handled by actions)

use crate::action::Action;
use crate::app::App;
use crate::app::export::ExportTarget;
use crate::ui::Toast;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle input for the users screen.
    pub fn handle_users_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Ctrl+C or 'y': copy selected username (vim-style)
        let is_copy = (key.modifiers.contains(KeyModifiers::CONTROL)
            && matches!(key.code, KeyCode::Char('c')))
            || (key.modifiers.is_empty() && matches!(key.code, KeyCode::Char('y')));
        if is_copy {
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
            KeyCode::Char('c') => {
                // Open create user dialog
                self.popup = Some(
                    Popup::builder(PopupType::CreateUser {
                        name_input: String::new(),
                        password_input: String::new(),
                        roles_input: String::new(),
                        realname_input: String::new(),
                        email_input: String::new(),
                        default_app_input: String::new(),
                    })
                    .build(),
                );
                None
            }
            KeyCode::Char('m') => {
                // Open modify user dialog for selected user
                if let Some(users) = &self.users
                    && let Some(selected) = self.users_state.selected()
                    && let Some(user) = users.get(selected)
                {
                    self.popup = Some(
                        Popup::builder(PopupType::ModifyUser {
                            user_name: user.name.clone(),
                            current_roles: user.roles.clone(),
                            current_realname: user.realname.clone(),
                            current_email: user.email.clone(),
                            current_default_app: user.default_app.clone(),
                            password_input: String::new(),
                            roles_input: user.roles.join(","),
                            realname_input: user.realname.clone().unwrap_or_default(),
                            email_input: user.email.clone().unwrap_or_default(),
                            default_app_input: user.default_app.clone().unwrap_or_default(),
                        })
                        .build(),
                    );
                } else {
                    self.toasts.push(Toast::info("No user selected"));
                }
                None
            }
            KeyCode::Char('d') => {
                // Open delete user confirmation for selected user
                if let Some(users) = &self.users
                    && let Some(selected) = self.users_state.selected()
                    && let Some(user) = users.get(selected)
                {
                    self.popup = Some(
                        Popup::builder(PopupType::DeleteUserConfirm {
                            user_name: user.name.clone(),
                        })
                        .build(),
                    );
                } else {
                    self.toasts.push(Toast::info("No user selected"));
                }
                None
            }
            _ => None,
        }
    }
}
