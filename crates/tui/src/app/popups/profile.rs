//! Profile management popup handlers.
//!
//! Responsibilities:
//! - Handle profile creation, editing, deletion, and selection popups
//! - Manage multi-field form navigation (Tab, Up/Down arrows)
//! - Handle character input and backspace for text fields
//! - Handle boolean toggles for checkboxes (Space key)
//!
//! Does NOT handle:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT persist profiles (just returns Action::SaveProfile/DeleteProfile)

use crate::action::Action;
use crate::app::App;
use crate::ui::ToastLevel;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle profile-related popups (ProfileSelector, CreateProfile, EditProfile, DeleteProfileConfirm).
    pub fn handle_profile_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match (self.popup.as_ref().map(|p| &p.kind), key.code) {
            // ProfileSelector - close
            (Some(PopupType::ProfileSelector { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // ProfileSelector - navigate up
            (
                Some(PopupType::ProfileSelector {
                    profiles,
                    selected_index,
                }),
                KeyCode::Up | KeyCode::Char('k'),
            ) => {
                let new_index = selected_index.saturating_sub(1);
                self.popup = Some(
                    Popup::builder(PopupType::ProfileSelector {
                        profiles: profiles.clone(),
                        selected_index: new_index,
                    })
                    .build(),
                );
                None
            }
            // ProfileSelector - navigate down
            (
                Some(PopupType::ProfileSelector {
                    profiles,
                    selected_index,
                }),
                KeyCode::Down | KeyCode::Char('j'),
            ) => {
                let new_index = (selected_index + 1).min(profiles.len().saturating_sub(1));
                self.popup = Some(
                    Popup::builder(PopupType::ProfileSelector {
                        profiles: profiles.clone(),
                        selected_index: new_index,
                    })
                    .build(),
                );
                None
            }
            // ProfileSelector - select
            (
                Some(PopupType::ProfileSelector {
                    profiles,
                    selected_index,
                }),
                KeyCode::Enter,
            ) => {
                if let Some(profile_name) = profiles.get(*selected_index) {
                    let name = profile_name.clone();
                    self.popup = None;
                    Some(Action::ProfileSelected(name))
                } else {
                    self.popup = None;
                    None
                }
            }
            // CreateProfile - close
            (Some(PopupType::CreateProfile { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // CreateProfile - submit
            (Some(PopupType::CreateProfile { name_input, .. }), KeyCode::Enter) => {
                if name_input.is_empty() {
                    return None;
                }
                // Extract all fields from the popup state
                if let Some(Popup {
                    kind:
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
                            ..
                        },
                    ..
                }) = self.popup.take()
                {
                    // Build the profile config
                    let base_url = if base_url_input.is_empty() {
                        None
                    } else {
                        Some(base_url_input)
                    };
                    let username = if username_input.is_empty() {
                        None
                    } else {
                        Some(username_input)
                    };
                    let password = if password_input.is_empty() {
                        None
                    } else {
                        Some(splunk_config::types::SecureValue::Plain(
                            secrecy::SecretString::new(password_input.into()),
                        ))
                    };
                    let api_token = if api_token_input.is_empty() {
                        None
                    } else {
                        Some(splunk_config::types::SecureValue::Plain(
                            secrecy::SecretString::new(api_token_input.into()),
                        ))
                    };

                    let profile = splunk_config::types::ProfileConfig {
                        base_url,
                        username,
                        password,
                        api_token,
                        skip_verify: Some(skip_verify),
                        timeout_seconds: Some(timeout_seconds),
                        max_retries: Some(max_retries as usize),
                        ..Default::default()
                    };

                    Some(Action::SaveProfile {
                        name: name_input,
                        profile,
                        use_keyring,
                        original_name: None,
                    })
                } else {
                    None
                }
            }
            // CreateProfile - Tab navigation
            (
                Some(PopupType::CreateProfile {
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
                }),
                KeyCode::Tab,
            ) => {
                let new_field = if key.modifiers.contains(KeyModifiers::SHIFT) {
                    selected_field.previous()
                } else {
                    selected_field.next()
                };
                self.popup = Some(
                    Popup::builder(PopupType::CreateProfile {
                        name_input: name_input.clone(),
                        base_url_input: base_url_input.clone(),
                        username_input: username_input.clone(),
                        password_input: password_input.clone(),
                        api_token_input: api_token_input.clone(),
                        skip_verify: *skip_verify,
                        timeout_seconds: *timeout_seconds,
                        max_retries: *max_retries,
                        use_keyring: *use_keyring,
                        selected_field: new_field,
                    })
                    .build(),
                );
                None
            }
            // CreateProfile - Up navigation
            (
                Some(PopupType::CreateProfile {
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
                }),
                KeyCode::Up,
            ) => {
                self.popup = Some(
                    Popup::builder(PopupType::CreateProfile {
                        name_input: name_input.clone(),
                        base_url_input: base_url_input.clone(),
                        username_input: username_input.clone(),
                        password_input: password_input.clone(),
                        api_token_input: api_token_input.clone(),
                        skip_verify: *skip_verify,
                        timeout_seconds: *timeout_seconds,
                        max_retries: *max_retries,
                        use_keyring: *use_keyring,
                        selected_field: selected_field.previous(),
                    })
                    .build(),
                );
                None
            }
            // CreateProfile - Down navigation
            (
                Some(PopupType::CreateProfile {
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
                }),
                KeyCode::Down,
            ) => {
                self.popup = Some(
                    Popup::builder(PopupType::CreateProfile {
                        name_input: name_input.clone(),
                        base_url_input: base_url_input.clone(),
                        username_input: username_input.clone(),
                        password_input: password_input.clone(),
                        api_token_input: api_token_input.clone(),
                        skip_verify: *skip_verify,
                        timeout_seconds: *timeout_seconds,
                        max_retries: *max_retries,
                        use_keyring: *use_keyring,
                        selected_field: selected_field.next(),
                    })
                    .build(),
                );
                None
            }
            // CreateProfile - character input
            (
                Some(PopupType::CreateProfile {
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
                }),
                KeyCode::Char(c),
            ) => self.handle_create_profile_char_input(
                name_input.clone(),
                base_url_input.clone(),
                username_input.clone(),
                password_input.clone(),
                api_token_input.clone(),
                *skip_verify,
                *timeout_seconds,
                *max_retries,
                *use_keyring,
                *selected_field,
                c,
            ),
            // CreateProfile - backspace
            (
                Some(PopupType::CreateProfile {
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
                }),
                KeyCode::Backspace,
            ) => self.handle_create_profile_backspace(
                name_input.clone(),
                base_url_input.clone(),
                username_input.clone(),
                password_input.clone(),
                api_token_input.clone(),
                *skip_verify,
                *timeout_seconds,
                *max_retries,
                *use_keyring,
                *selected_field,
            ),
            // EditProfile - submit
            (Some(PopupType::EditProfile { name_input, .. }), KeyCode::Enter) => {
                self.handle_edit_profile_submit(name_input.clone())
            }
            // EditProfile - close
            (Some(PopupType::EditProfile { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // EditProfile - Tab navigation
            (
                Some(PopupType::EditProfile {
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
                    selected_field,
                }),
                KeyCode::Tab,
            ) => {
                let new_field = if key.modifiers.contains(KeyModifiers::SHIFT) {
                    selected_field.previous()
                } else {
                    selected_field.next()
                };
                self.popup = Some(
                    Popup::builder(PopupType::EditProfile {
                        original_name: original_name.clone(),
                        name_input: name_input.clone(),
                        base_url_input: base_url_input.clone(),
                        username_input: username_input.clone(),
                        password_input: password_input.clone(),
                        api_token_input: api_token_input.clone(),
                        skip_verify: *skip_verify,
                        timeout_seconds: *timeout_seconds,
                        max_retries: *max_retries,
                        use_keyring: *use_keyring,
                        selected_field: new_field,
                    })
                    .build(),
                );
                None
            }
            // EditProfile - Up navigation
            (
                Some(PopupType::EditProfile {
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
                    selected_field,
                }),
                KeyCode::Up,
            ) => {
                self.popup = Some(
                    Popup::builder(PopupType::EditProfile {
                        original_name: original_name.clone(),
                        name_input: name_input.clone(),
                        base_url_input: base_url_input.clone(),
                        username_input: username_input.clone(),
                        password_input: password_input.clone(),
                        api_token_input: api_token_input.clone(),
                        skip_verify: *skip_verify,
                        timeout_seconds: *timeout_seconds,
                        max_retries: *max_retries,
                        use_keyring: *use_keyring,
                        selected_field: selected_field.previous(),
                    })
                    .build(),
                );
                None
            }
            // EditProfile - Down navigation
            (
                Some(PopupType::EditProfile {
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
                    selected_field,
                }),
                KeyCode::Down,
            ) => {
                self.popup = Some(
                    Popup::builder(PopupType::EditProfile {
                        original_name: original_name.clone(),
                        name_input: name_input.clone(),
                        base_url_input: base_url_input.clone(),
                        username_input: username_input.clone(),
                        password_input: password_input.clone(),
                        api_token_input: api_token_input.clone(),
                        skip_verify: *skip_verify,
                        timeout_seconds: *timeout_seconds,
                        max_retries: *max_retries,
                        use_keyring: *use_keyring,
                        selected_field: selected_field.next(),
                    })
                    .build(),
                );
                None
            }
            // EditProfile - character input
            (
                Some(PopupType::EditProfile {
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
                    selected_field,
                }),
                KeyCode::Char(c),
            ) => self.handle_edit_profile_char_input(
                original_name.clone(),
                name_input.clone(),
                base_url_input.clone(),
                username_input.clone(),
                password_input.clone(),
                api_token_input.clone(),
                *skip_verify,
                *timeout_seconds,
                *max_retries,
                *use_keyring,
                *selected_field,
                c,
            ),
            // EditProfile - backspace
            (
                Some(PopupType::EditProfile {
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
                    selected_field,
                }),
                KeyCode::Backspace,
            ) => self.handle_edit_profile_backspace(
                original_name.clone(),
                name_input.clone(),
                base_url_input.clone(),
                username_input.clone(),
                password_input.clone(),
                api_token_input.clone(),
                *skip_verify,
                *timeout_seconds,
                *max_retries,
                *use_keyring,
                *selected_field,
            ),
            // DeleteProfileConfirm - confirm
            (
                Some(PopupType::DeleteProfileConfirm { profile_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = profile_name.clone();
                self.popup = None;
                Some(Action::DeleteProfile { name })
            }
            // DeleteProfileConfirm - cancel
            (Some(PopupType::DeleteProfileConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            _ => None,
        }
    }

    /// Handle character input for CreateProfile popup.
    #[allow(clippy::too_many_arguments)]
    fn handle_create_profile_char_input(
        &mut self,
        name_input: String,
        base_url_input: String,
        username_input: String,
        password_input: String,
        api_token_input: String,
        skip_verify: bool,
        timeout_seconds: u64,
        max_retries: u64,
        use_keyring: bool,
        selected_field: crate::ui::popup::ProfileField,
        c: char,
    ) -> Option<Action> {
        let mut new_name = name_input.clone();
        let mut new_base_url = base_url_input.clone();
        let mut new_username = username_input.clone();
        let mut new_password = password_input.clone();
        let mut new_api_token = api_token_input.clone();

        match selected_field {
            crate::ui::popup::ProfileField::Name => new_name.push(c),
            crate::ui::popup::ProfileField::BaseUrl => new_base_url.push(c),
            crate::ui::popup::ProfileField::Username => new_username.push(c),
            crate::ui::popup::ProfileField::Password => new_password.push(c),
            crate::ui::popup::ProfileField::ApiToken => new_api_token.push(c),
            crate::ui::popup::ProfileField::SkipVerify => {
                // Toggle on space - handled below
            }
            crate::ui::popup::ProfileField::Timeout => {
                if c.is_ascii_digit() {
                    let current = timeout_seconds.to_string();
                    let new_val = format!("{}{}", current, c);
                    if let Ok(val) = new_val.parse::<u64>() {
                        self.popup = Some(
                            Popup::builder(PopupType::CreateProfile {
                                name_input: name_input.clone(),
                                base_url_input: base_url_input.clone(),
                                username_input: username_input.clone(),
                                password_input: password_input.clone(),
                                api_token_input: api_token_input.clone(),
                                skip_verify,
                                timeout_seconds: val,
                                max_retries,
                                use_keyring,
                                selected_field,
                            })
                            .build(),
                        );
                    } else {
                        return Some(Action::Notify(
                            ToastLevel::Error,
                            "Invalid timeout value: number too large".to_string(),
                        ));
                    }
                }
                return None;
            }
            crate::ui::popup::ProfileField::MaxRetries => {
                if c.is_ascii_digit() {
                    let current = max_retries.to_string();
                    let new_val = format!("{}{}", current, c);
                    if let Ok(val) = new_val.parse::<u64>() {
                        self.popup = Some(
                            Popup::builder(PopupType::CreateProfile {
                                name_input: name_input.clone(),
                                base_url_input: base_url_input.clone(),
                                username_input: username_input.clone(),
                                password_input: password_input.clone(),
                                api_token_input: api_token_input.clone(),
                                skip_verify,
                                timeout_seconds,
                                max_retries: val,
                                use_keyring,
                                selected_field,
                            })
                            .build(),
                        );
                    } else {
                        return Some(Action::Notify(
                            ToastLevel::Error,
                            "Invalid max retries value: number too large".to_string(),
                        ));
                    }
                }
                return None;
            }
            crate::ui::popup::ProfileField::UseKeyring => {
                // Toggle on space - handled below
            }
        }

        // Handle space for boolean fields
        if c == ' ' {
            match selected_field {
                crate::ui::popup::ProfileField::SkipVerify => {
                    self.popup = Some(
                        Popup::builder(PopupType::CreateProfile {
                            name_input: name_input.clone(),
                            base_url_input: base_url_input.clone(),
                            username_input: username_input.clone(),
                            password_input: password_input.clone(),
                            api_token_input: api_token_input.clone(),
                            skip_verify: !skip_verify,
                            timeout_seconds,
                            max_retries,
                            use_keyring,
                            selected_field,
                        })
                        .build(),
                    );
                }
                crate::ui::popup::ProfileField::UseKeyring => {
                    self.popup = Some(
                        Popup::builder(PopupType::CreateProfile {
                            name_input: name_input.clone(),
                            base_url_input: base_url_input.clone(),
                            username_input: username_input.clone(),
                            password_input: password_input.clone(),
                            api_token_input: api_token_input.clone(),
                            skip_verify,
                            timeout_seconds,
                            max_retries,
                            use_keyring: !use_keyring,
                            selected_field,
                        })
                        .build(),
                    );
                }
                _ => {}
            }
            return None;
        }

        self.popup = Some(
            Popup::builder(PopupType::CreateProfile {
                name_input: new_name,
                base_url_input: new_base_url,
                username_input: new_username,
                password_input: new_password,
                api_token_input: new_api_token,
                skip_verify,
                timeout_seconds,
                max_retries,
                use_keyring,
                selected_field,
            })
            .build(),
        );
        None
    }

    /// Handle backspace for CreateProfile popup.
    #[allow(clippy::too_many_arguments)]
    fn handle_create_profile_backspace(
        &mut self,
        name_input: String,
        base_url_input: String,
        username_input: String,
        password_input: String,
        api_token_input: String,
        skip_verify: bool,
        timeout_seconds: u64,
        max_retries: u64,
        use_keyring: bool,
        selected_field: crate::ui::popup::ProfileField,
    ) -> Option<Action> {
        let mut new_name = name_input.clone();
        let mut new_base_url = base_url_input.clone();
        let mut new_username = username_input.clone();
        let mut new_password = password_input.clone();
        let mut new_api_token = api_token_input.clone();

        match selected_field {
            crate::ui::popup::ProfileField::Name => new_name.pop(),
            crate::ui::popup::ProfileField::BaseUrl => new_base_url.pop(),
            crate::ui::popup::ProfileField::Username => new_username.pop(),
            crate::ui::popup::ProfileField::Password => new_password.pop(),
            crate::ui::popup::ProfileField::ApiToken => new_api_token.pop(),
            crate::ui::popup::ProfileField::Timeout => {
                let current = timeout_seconds.to_string();
                let new_val = current[..current.len().saturating_sub(1)].to_string();
                if let Ok(val) = new_val.parse::<u64>() {
                    self.popup = Some(
                        Popup::builder(PopupType::CreateProfile {
                            name_input: name_input.clone(),
                            base_url_input: base_url_input.clone(),
                            username_input: username_input.clone(),
                            password_input: password_input.clone(),
                            api_token_input: api_token_input.clone(),
                            skip_verify,
                            timeout_seconds: val,
                            max_retries,
                            use_keyring,
                            selected_field,
                        })
                        .build(),
                    );
                } else {
                    return Some(Action::Notify(
                        ToastLevel::Error,
                        "Invalid timeout value".to_string(),
                    ));
                }
                return None;
            }
            crate::ui::popup::ProfileField::MaxRetries => {
                let current = max_retries.to_string();
                let new_val = current[..current.len().saturating_sub(1)].to_string();
                if let Ok(val) = new_val.parse::<u64>() {
                    self.popup = Some(
                        Popup::builder(PopupType::CreateProfile {
                            name_input: name_input.clone(),
                            base_url_input: base_url_input.clone(),
                            username_input: username_input.clone(),
                            password_input: password_input.clone(),
                            api_token_input: api_token_input.clone(),
                            skip_verify,
                            timeout_seconds,
                            max_retries: val,
                            use_keyring,
                            selected_field,
                        })
                        .build(),
                    );
                } else {
                    return Some(Action::Notify(
                        ToastLevel::Error,
                        "Invalid max retries value".to_string(),
                    ));
                }
                return None;
            }
            _ => None,
        };

        self.popup = Some(
            Popup::builder(PopupType::CreateProfile {
                name_input: new_name,
                base_url_input: new_base_url,
                username_input: new_username,
                password_input: new_password,
                api_token_input: new_api_token,
                skip_verify,
                timeout_seconds,
                max_retries,
                use_keyring,
                selected_field,
            })
            .build(),
        );
        None
    }

    /// Handle submit for EditProfile popup.
    fn handle_edit_profile_submit(&mut self, name_input: String) -> Option<Action> {
        if name_input.is_empty() {
            return None;
        }
        // Extract all fields from the popup state
        if let Some(Popup {
            kind:
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
                },
            ..
        }) = self.popup.take()
        {
            // Build the profile config
            let base_url = if base_url_input.is_empty() {
                None
            } else {
                Some(base_url_input)
            };
            let username = if username_input.is_empty() {
                None
            } else {
                Some(username_input)
            };
            // For edit: empty password/token means "keep existing" - use a placeholder
            // The side effect handler will need to merge with existing profile
            let password = if password_input.is_empty() {
                None
            } else {
                Some(splunk_config::types::SecureValue::Plain(
                    secrecy::SecretString::new(password_input.into()),
                ))
            };
            let api_token = if api_token_input.is_empty() {
                None
            } else {
                Some(splunk_config::types::SecureValue::Plain(
                    secrecy::SecretString::new(api_token_input.into()),
                ))
            };

            let profile = splunk_config::types::ProfileConfig {
                base_url,
                username,
                password,
                api_token,
                skip_verify: Some(skip_verify),
                timeout_seconds: Some(timeout_seconds),
                max_retries: Some(max_retries as usize),
                ..Default::default()
            };

            // Detect rename: if name changed, pass original_name for cleanup
            let is_rename = name_input != original_name;
            Some(Action::SaveProfile {
                name: name_input,
                profile,
                use_keyring,
                original_name: if is_rename { Some(original_name) } else { None },
            })
        } else {
            None
        }
    }

    /// Handle character input for EditProfile popup.
    #[allow(clippy::too_many_arguments)]
    fn handle_edit_profile_char_input(
        &mut self,
        original_name: String,
        name_input: String,
        base_url_input: String,
        username_input: String,
        password_input: String,
        api_token_input: String,
        skip_verify: bool,
        timeout_seconds: u64,
        max_retries: u64,
        use_keyring: bool,
        selected_field: crate::ui::popup::ProfileField,
        c: char,
    ) -> Option<Action> {
        let mut new_name = name_input.clone();
        let mut new_base_url = base_url_input.clone();
        let mut new_username = username_input.clone();
        let mut new_password = password_input.clone();
        let mut new_api_token = api_token_input.clone();

        match selected_field {
            crate::ui::popup::ProfileField::Name => new_name.push(c),
            crate::ui::popup::ProfileField::BaseUrl => new_base_url.push(c),
            crate::ui::popup::ProfileField::Username => new_username.push(c),
            crate::ui::popup::ProfileField::Password => new_password.push(c),
            crate::ui::popup::ProfileField::ApiToken => new_api_token.push(c),
            crate::ui::popup::ProfileField::SkipVerify => {
                // Toggle on space - handled below
            }
            crate::ui::popup::ProfileField::Timeout => {
                if c.is_ascii_digit() {
                    let current = timeout_seconds.to_string();
                    let new_val = format!("{}{}", current, c);
                    if let Ok(val) = new_val.parse::<u64>() {
                        self.popup = Some(
                            Popup::builder(PopupType::EditProfile {
                                original_name: original_name.clone(),
                                name_input: name_input.clone(),
                                base_url_input: base_url_input.clone(),
                                username_input: username_input.clone(),
                                password_input: password_input.clone(),
                                api_token_input: api_token_input.clone(),
                                skip_verify,
                                timeout_seconds: val,
                                max_retries,
                                use_keyring,
                                selected_field,
                            })
                            .build(),
                        );
                    } else {
                        return Some(Action::Notify(
                            ToastLevel::Error,
                            "Invalid timeout value: number too large".to_string(),
                        ));
                    }
                }
                return None;
            }
            crate::ui::popup::ProfileField::MaxRetries => {
                if c.is_ascii_digit() {
                    let current = max_retries.to_string();
                    let new_val = format!("{}{}", current, c);
                    if let Ok(val) = new_val.parse::<u64>() {
                        self.popup = Some(
                            Popup::builder(PopupType::EditProfile {
                                original_name: original_name.clone(),
                                name_input: name_input.clone(),
                                base_url_input: base_url_input.clone(),
                                username_input: username_input.clone(),
                                password_input: password_input.clone(),
                                api_token_input: api_token_input.clone(),
                                skip_verify,
                                timeout_seconds,
                                max_retries: val,
                                use_keyring,
                                selected_field,
                            })
                            .build(),
                        );
                    } else {
                        return Some(Action::Notify(
                            ToastLevel::Error,
                            "Invalid max retries value: number too large".to_string(),
                        ));
                    }
                }
                return None;
            }
            crate::ui::popup::ProfileField::UseKeyring => {
                // Toggle on space - handled below
            }
        }

        // Handle space for boolean fields
        if c == ' ' {
            match selected_field {
                crate::ui::popup::ProfileField::SkipVerify => {
                    self.popup = Some(
                        Popup::builder(PopupType::EditProfile {
                            original_name: original_name.clone(),
                            name_input: name_input.clone(),
                            base_url_input: base_url_input.clone(),
                            username_input: username_input.clone(),
                            password_input: password_input.clone(),
                            api_token_input: api_token_input.clone(),
                            skip_verify: !skip_verify,
                            timeout_seconds,
                            max_retries,
                            use_keyring,
                            selected_field,
                        })
                        .build(),
                    );
                }
                crate::ui::popup::ProfileField::UseKeyring => {
                    self.popup = Some(
                        Popup::builder(PopupType::EditProfile {
                            original_name: original_name.clone(),
                            name_input: name_input.clone(),
                            base_url_input: base_url_input.clone(),
                            username_input: username_input.clone(),
                            password_input: password_input.clone(),
                            api_token_input: api_token_input.clone(),
                            skip_verify,
                            timeout_seconds,
                            max_retries,
                            use_keyring: !use_keyring,
                            selected_field,
                        })
                        .build(),
                    );
                }
                _ => {}
            }
            return None;
        }

        self.popup = Some(
            Popup::builder(PopupType::EditProfile {
                original_name: original_name.clone(),
                name_input: new_name,
                base_url_input: new_base_url,
                username_input: new_username,
                password_input: new_password,
                api_token_input: new_api_token,
                skip_verify,
                timeout_seconds,
                max_retries,
                use_keyring,
                selected_field,
            })
            .build(),
        );
        None
    }

    /// Handle backspace for EditProfile popup.
    #[allow(clippy::too_many_arguments)]
    fn handle_edit_profile_backspace(
        &mut self,
        original_name: String,
        name_input: String,
        base_url_input: String,
        username_input: String,
        password_input: String,
        api_token_input: String,
        skip_verify: bool,
        timeout_seconds: u64,
        max_retries: u64,
        use_keyring: bool,
        selected_field: crate::ui::popup::ProfileField,
    ) -> Option<Action> {
        let mut new_name = name_input.clone();
        let mut new_base_url = base_url_input.clone();
        let mut new_username = username_input.clone();
        let mut new_password = password_input.clone();
        let mut new_api_token = api_token_input.clone();

        match selected_field {
            crate::ui::popup::ProfileField::Name => new_name.pop(),
            crate::ui::popup::ProfileField::BaseUrl => new_base_url.pop(),
            crate::ui::popup::ProfileField::Username => new_username.pop(),
            crate::ui::popup::ProfileField::Password => new_password.pop(),
            crate::ui::popup::ProfileField::ApiToken => new_api_token.pop(),
            crate::ui::popup::ProfileField::Timeout => {
                let current = timeout_seconds.to_string();
                let new_val = current[..current.len().saturating_sub(1)].to_string();
                if let Ok(val) = new_val.parse::<u64>() {
                    self.popup = Some(
                        Popup::builder(PopupType::EditProfile {
                            original_name: original_name.clone(),
                            name_input: name_input.clone(),
                            base_url_input: base_url_input.clone(),
                            username_input: username_input.clone(),
                            password_input: password_input.clone(),
                            api_token_input: api_token_input.clone(),
                            skip_verify,
                            timeout_seconds: val,
                            max_retries,
                            use_keyring,
                            selected_field,
                        })
                        .build(),
                    );
                } else {
                    return Some(Action::Notify(
                        ToastLevel::Error,
                        "Invalid timeout value".to_string(),
                    ));
                }
                return None;
            }
            crate::ui::popup::ProfileField::MaxRetries => {
                let current = max_retries.to_string();
                let new_val = current[..current.len().saturating_sub(1)].to_string();
                if let Ok(val) = new_val.parse::<u64>() {
                    self.popup = Some(
                        Popup::builder(PopupType::EditProfile {
                            original_name: original_name.clone(),
                            name_input: name_input.clone(),
                            base_url_input: base_url_input.clone(),
                            username_input: username_input.clone(),
                            password_input: password_input.clone(),
                            api_token_input: api_token_input.clone(),
                            skip_verify,
                            timeout_seconds,
                            max_retries: val,
                            use_keyring,
                            selected_field,
                        })
                        .build(),
                    );
                } else {
                    return Some(Action::Notify(
                        ToastLevel::Error,
                        "Invalid max retries value".to_string(),
                    ));
                }
                return None;
            }
            _ => None,
        };

        self.popup = Some(
            Popup::builder(PopupType::EditProfile {
                original_name: original_name.clone(),
                name_input: new_name,
                base_url_input: new_base_url,
                username_input: new_username,
                password_input: new_password,
                api_token_input: new_api_token,
                skip_verify,
                timeout_seconds,
                max_retries,
                use_keyring,
                selected_field,
            })
            .build(),
        );
        None
    }
}
