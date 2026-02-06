//! User management popup handlers.
//!
//! Responsibilities:
//! - Handle user creation, modification, and deletion popups
//! - Manage form input for user fields (name, password, roles, etc.)
//!
//! Does NOT handle:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT execute the actual user operations (just returns Action variants)

use crate::action::Action;
use crate::app::App;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle user-related popups (CreateUser, ModifyUser, DeleteUserConfirm).
    pub fn handle_user_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match (self.popup.as_ref().map(|p| &p.kind), key.code) {
            // CreateUser - close
            (Some(PopupType::CreateUser { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // CreateUser - submit
            (Some(PopupType::CreateUser { name_input, .. }), KeyCode::Enter) => {
                if name_input.is_empty() {
                    return None;
                }
                let name = name_input.clone();
                // Extract other fields from the popup state
                if let Some(Popup {
                    kind:
                        PopupType::CreateUser {
                            password_input,
                            roles_input,
                            realname_input,
                            email_input,
                            default_app_input,
                            ..
                        },
                    ..
                }) = self.popup.take()
                {
                    let password = secrecy::SecretString::from(password_input);
                    let roles: Vec<String> = roles_input
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let realname = if realname_input.is_empty() {
                        None
                    } else {
                        Some(realname_input)
                    };
                    let email = if email_input.is_empty() {
                        None
                    } else {
                        Some(email_input)
                    };
                    let default_app = if default_app_input.is_empty() {
                        None
                    } else {
                        Some(default_app_input)
                    };
                    let params = splunk_client::CreateUserParams {
                        name,
                        password,
                        roles,
                        realname,
                        email,
                        default_app,
                    };
                    Some(Action::CreateUser { params })
                } else {
                    None
                }
            }
            // CreateUser - character input
            (
                Some(PopupType::CreateUser {
                    name_input,
                    password_input,
                    roles_input,
                    realname_input,
                    email_input,
                    default_app_input,
                }),
                KeyCode::Char(c),
            ) => {
                let mut new_name = name_input.clone();
                new_name.push(c);
                self.popup = Some(
                    Popup::builder(PopupType::CreateUser {
                        name_input: new_name,
                        password_input: password_input.clone(),
                        roles_input: roles_input.clone(),
                        realname_input: realname_input.clone(),
                        email_input: email_input.clone(),
                        default_app_input: default_app_input.clone(),
                    })
                    .build(),
                );
                None
            }
            // CreateUser - backspace
            (
                Some(PopupType::CreateUser {
                    name_input,
                    password_input,
                    roles_input,
                    realname_input,
                    email_input,
                    default_app_input,
                }),
                KeyCode::Backspace,
            ) => {
                let mut new_name = name_input.clone();
                new_name.pop();
                self.popup = Some(
                    Popup::builder(PopupType::CreateUser {
                        name_input: new_name,
                        password_input: password_input.clone(),
                        roles_input: roles_input.clone(),
                        realname_input: realname_input.clone(),
                        email_input: email_input.clone(),
                        default_app_input: default_app_input.clone(),
                    })
                    .build(),
                );
                None
            }
            // ModifyUser - close
            (Some(PopupType::ModifyUser { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // ModifyUser - submit
            (Some(PopupType::ModifyUser { user_name, .. }), KeyCode::Enter) => {
                let name = user_name.clone();
                if let Some(Popup {
                    kind:
                        PopupType::ModifyUser {
                            password_input,
                            roles_input,
                            realname_input,
                            email_input,
                            default_app_input,
                            ..
                        },
                    ..
                }) = self.popup.take()
                {
                    let password = if password_input.is_empty() {
                        None
                    } else {
                        Some(secrecy::SecretString::from(password_input))
                    };
                    let roles: Vec<String> = roles_input
                        .split(',')
                        .map(|s| s.trim().to_string())
                        .filter(|s| !s.is_empty())
                        .collect();
                    let roles = if roles.is_empty() { None } else { Some(roles) };
                    let realname = if realname_input.is_empty() {
                        None
                    } else {
                        Some(realname_input)
                    };
                    let email = if email_input.is_empty() {
                        None
                    } else {
                        Some(email_input)
                    };
                    let default_app = if default_app_input.is_empty() {
                        None
                    } else {
                        Some(default_app_input)
                    };
                    let params = splunk_client::ModifyUserParams {
                        password,
                        roles,
                        realname,
                        email,
                        default_app,
                    };
                    Some(Action::ModifyUser { name, params })
                } else {
                    None
                }
            }
            // DeleteUserConfirm - confirm
            (
                Some(PopupType::DeleteUserConfirm { user_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = user_name.clone();
                self.popup = None;
                Some(Action::DeleteUser { name })
            }
            // DeleteUserConfirm - cancel
            (Some(PopupType::DeleteUserConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            _ => None,
        }
    }
}
