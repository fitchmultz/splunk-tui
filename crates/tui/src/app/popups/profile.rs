//! Profile management popup handlers.
//!
//! Responsibilities:
//! - Handle profile selection, creation, editing, and deletion popups.
//! - Keep profile form navigation and editing logic consistent.
//! - Build save/delete actions from popup state.
//!
//! Does NOT handle:
//! - Popup rendering.
//! - Persisting profile changes.
//! - Non-profile popup behavior.
//!
//! Invariants:
//! - Empty profile names never submit.
//! - Numeric fields use checked integer editing.
//! - Empty profile selectors never move away from index 0.

use crate::action::Action;
use crate::app::App;
use crate::ui::ToastLevel;
use crate::ui::popup::{PopupType, ProfileField};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use splunk_config::types::ProfileConfig;

use super::common::{append_digit, optional_secure_value, optional_string, pop_digit};

impl App {
    /// Handle profile-related popups (ProfileSelector, CreateProfile, EditProfile, DeleteProfileConfirm).
    pub fn handle_profile_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match (
            self.popup.as_ref().map(|popup| popup.kind.clone()),
            key.code,
        ) {
            (Some(PopupType::ProfileSelector { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (
                Some(PopupType::ProfileSelector {
                    profiles,
                    selected_index,
                }),
                KeyCode::Up | KeyCode::Char('k'),
            ) => {
                let is_empty = profiles.is_empty();
                self.replace_popup_kind(PopupType::ProfileSelector {
                    profiles,
                    selected_index: if selected_index == 0 || is_empty {
                        0
                    } else {
                        selected_index - 1
                    },
                });
                None
            }
            (
                Some(PopupType::ProfileSelector {
                    profiles,
                    selected_index,
                }),
                KeyCode::Down | KeyCode::Char('j'),
            ) => {
                let last_index = profiles.len().saturating_sub(1);
                self.replace_popup_kind(PopupType::ProfileSelector {
                    profiles,
                    selected_index: if last_index == 0 {
                        0
                    } else {
                        (selected_index + 1).min(last_index)
                    },
                });
                None
            }
            (
                Some(PopupType::ProfileSelector {
                    profiles,
                    selected_index,
                }),
                KeyCode::Enter,
            ) => {
                self.popup = None;
                profiles
                    .get(selected_index)
                    .cloned()
                    .map(Action::ProfileSelected)
            }
            (Some(kind @ PopupType::CreateProfile { .. }), KeyCode::Enter) => submit_profile(kind),
            (Some(kind @ PopupType::EditProfile { .. }), KeyCode::Enter) => submit_profile(kind),
            (
                Some(PopupType::CreateProfile { .. } | PopupType::EditProfile { .. }),
                KeyCode::Esc,
            ) => {
                self.popup = None;
                None
            }
            (Some(mut kind @ PopupType::CreateProfile { .. }), KeyCode::Tab)
            | (Some(mut kind @ PopupType::EditProfile { .. }), KeyCode::Tab) => {
                kind.navigate_fields(key.modifiers.contains(KeyModifiers::SHIFT));
                self.replace_popup_kind(kind);
                None
            }
            (Some(mut kind @ PopupType::CreateProfile { .. }), KeyCode::Up)
            | (Some(mut kind @ PopupType::EditProfile { .. }), KeyCode::Up) => {
                kind.navigate_fields(true);
                self.replace_popup_kind(kind);
                None
            }
            (Some(mut kind @ PopupType::CreateProfile { .. }), KeyCode::Down)
            | (Some(mut kind @ PopupType::EditProfile { .. }), KeyCode::Down) => {
                kind.navigate_fields(false);
                self.replace_popup_kind(kind);
                None
            }
            (Some(mut kind @ PopupType::CreateProfile { .. }), KeyCode::Char(c))
            | (Some(mut kind @ PopupType::EditProfile { .. }), KeyCode::Char(c)) => {
                match update_profile_char(&mut kind, c) {
                    (true, None) => self.replace_popup_kind(kind),
                    (false, None) => {}
                    (_, Some(field)) => return Some(invalid_number_action(field)),
                }
                None
            }
            (Some(mut kind @ PopupType::CreateProfile { .. }), KeyCode::Backspace)
            | (Some(mut kind @ PopupType::EditProfile { .. }), KeyCode::Backspace) => {
                if update_profile_backspace(&mut kind) {
                    self.replace_popup_kind(kind);
                }
                None
            }
            (
                Some(PopupType::DeleteProfileConfirm { profile_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                self.popup = None;
                Some(Action::DeleteProfile { name: profile_name })
            }
            (Some(PopupType::DeleteProfileConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            _ => None,
        }
    }
}

fn submit_profile(kind: PopupType) -> Option<Action> {
    match kind {
        PopupType::CreateProfile {
            name_input,
            base_url_input,
            username_input,
            password_input,
            api_token_input,
            skip_verify,
            timeout_seconds,
            max_retries,
            use_keyring,
            from_tutorial,
            ..
        } => {
            if name_input.is_empty() {
                return None;
            }

            Some(Action::SaveProfile {
                name: name_input,
                profile: build_profile_config(
                    base_url_input,
                    username_input,
                    password_input,
                    api_token_input,
                    skip_verify,
                    timeout_seconds,
                    max_retries,
                ),
                use_keyring,
                original_name: None,
                from_tutorial,
            })
        }
        PopupType::EditProfile {
            original_name,
            name_input,
            base_url_input,
            username_input,
            password_input,
            api_token_input,
            skip_verify,
            timeout_seconds,
            max_retries,
            use_keyring,
            ..
        } => {
            if name_input.is_empty() {
                return None;
            }

            Some(Action::SaveProfile {
                original_name: (name_input != original_name).then_some(original_name),
                name: name_input,
                profile: build_profile_config(
                    base_url_input,
                    username_input,
                    password_input,
                    api_token_input,
                    skip_verify,
                    timeout_seconds,
                    max_retries,
                ),
                use_keyring,
                from_tutorial: false,
            })
        }
        _ => None,
    }
}

fn build_profile_config(
    base_url_input: String,
    username_input: String,
    password_input: String,
    api_token_input: String,
    skip_verify: bool,
    timeout_seconds: u64,
    max_retries: u64,
) -> ProfileConfig {
    ProfileConfig {
        base_url: optional_string(base_url_input),
        username: optional_string(username_input),
        password: optional_secure_value(password_input),
        api_token: optional_secure_value(api_token_input),
        skip_verify: Some(skip_verify),
        timeout_seconds: Some(timeout_seconds),
        max_retries: Some(max_retries as usize),
        ..Default::default()
    }
}

fn update_profile_char(kind: &mut PopupType, c: char) -> (bool, Option<&'static str>) {
    match kind {
        PopupType::CreateProfile {
            name_input,
            base_url_input,
            username_input,
            password_input,
            api_token_input,
            skip_verify,
            timeout_seconds,
            max_retries,
            use_keyring,
            selected_field,
            ..
        }
        | PopupType::EditProfile {
            name_input,
            base_url_input,
            username_input,
            password_input,
            api_token_input,
            skip_verify,
            timeout_seconds,
            max_retries,
            use_keyring,
            selected_field,
            ..
        } => {
            match selected_field {
                ProfileField::Name => name_input.push(c),
                ProfileField::BaseUrl => base_url_input.push(c),
                ProfileField::Username => username_input.push(c),
                ProfileField::Password => password_input.push(c),
                ProfileField::ApiToken => api_token_input.push(c),
                ProfileField::SkipVerify if c == ' ' => *skip_verify = !*skip_verify,
                ProfileField::UseKeyring if c == ' ' => *use_keyring = !*use_keyring,
                ProfileField::Timeout if c.is_ascii_digit() => {
                    if !append_digit(timeout_seconds, c) {
                        return (false, Some("timeout"));
                    }
                }
                ProfileField::MaxRetries if c.is_ascii_digit() => {
                    if !append_digit(max_retries, c) {
                        return (false, Some("max retries"));
                    }
                }
                _ => return (false, None),
            }
            (true, None)
        }
        _ => (false, None),
    }
}

fn update_profile_backspace(kind: &mut PopupType) -> bool {
    match kind {
        PopupType::CreateProfile {
            name_input,
            base_url_input,
            username_input,
            password_input,
            api_token_input,
            timeout_seconds,
            max_retries,
            selected_field,
            ..
        }
        | PopupType::EditProfile {
            name_input,
            base_url_input,
            username_input,
            password_input,
            api_token_input,
            timeout_seconds,
            max_retries,
            selected_field,
            ..
        } => match selected_field {
            ProfileField::Name => {
                name_input.pop();
                true
            }
            ProfileField::BaseUrl => {
                base_url_input.pop();
                true
            }
            ProfileField::Username => {
                username_input.pop();
                true
            }
            ProfileField::Password => {
                password_input.pop();
                true
            }
            ProfileField::ApiToken => {
                api_token_input.pop();
                true
            }
            ProfileField::Timeout => {
                pop_digit(timeout_seconds);
                true
            }
            ProfileField::MaxRetries => {
                pop_digit(max_retries);
                true
            }
            ProfileField::SkipVerify | ProfileField::UseKeyring => false,
        },
        _ => false,
    }
}

fn invalid_number_action(field: &str) -> Action {
    Action::Notify(
        ToastLevel::Error,
        format!("Invalid {field} value: number too large"),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use crate::ui::popup::Popup;

    fn key(code: KeyCode) -> KeyEvent {
        KeyEvent::new(code, KeyModifiers::empty())
    }

    fn char_key(c: char) -> KeyEvent {
        key(KeyCode::Char(c))
    }

    #[test]
    fn test_profile_selector_empty_list_stays_at_zero() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::ProfileSelector {
                profiles: Vec::new(),
                selected_index: 0,
            })
            .build(),
        );

        app.handle_popup_input(key(KeyCode::Down));

        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::ProfileSelector {
                    selected_index: 0,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_profile_timeout_backspace_reaches_zero() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateProfile {
                name_input: String::from("dev"),
                base_url_input: String::new(),
                username_input: String::new(),
                password_input: String::new(),
                api_token_input: String::new(),
                skip_verify: false,
                timeout_seconds: 7,
                max_retries: 3,
                use_keyring: false,
                selected_field: ProfileField::Timeout,
                from_tutorial: false,
            })
            .build(),
        );

        let action = app.handle_popup_input(key(KeyCode::Backspace));
        assert!(action.is_none());
        assert!(matches!(
            app.popup,
            Some(Popup {
                kind: PopupType::CreateProfile {
                    timeout_seconds: 0,
                    ..
                },
                ..
            })
        ));
    }

    #[test]
    fn test_profile_max_retries_overflow_returns_error() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateProfile {
                name_input: String::from("dev"),
                base_url_input: String::new(),
                username_input: String::new(),
                password_input: String::new(),
                api_token_input: String::new(),
                skip_verify: false,
                timeout_seconds: 1,
                max_retries: u64::MAX,
                use_keyring: false,
                selected_field: ProfileField::MaxRetries,
                from_tutorial: false,
            })
            .build(),
        );

        let action = app.handle_popup_input(char_key('9'));
        assert!(matches!(
            action,
            Some(Action::Notify(ToastLevel::Error, ref message))
                if message.contains("max retries")
        ));
    }
}
