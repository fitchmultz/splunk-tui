//! Popup input handling for the TUI app.
//!
//! Responsibilities:
//! - Handle keyboard input when popups are active
//! - Manage export popup state and input
//!
//! Non-responsibilities:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT define popup types (handled by ui::popup module)

use crate::action::{Action, ExportFormat};
use crate::app::App;
use crate::ui::popup::{Popup, PopupType};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

impl App {
    /// Handle keyboard input when a popup is active.
    pub fn handle_popup_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Check for global quit first (Ctrl+Q works from any popup)
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(Action::Quit);
        }

        match (self.popup.as_ref().map(|p| &p.kind), key.code) {
            (Some(PopupType::Help), KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('?')) => {
                self.popup = None;
                self.help_scroll_offset = 0;
                None
            }
            (Some(PopupType::Help), KeyCode::Char('j') | KeyCode::Down) => {
                self.help_scroll_offset = self.help_scroll_offset.saturating_add(1);
                None
            }
            (Some(PopupType::Help), KeyCode::Char('k') | KeyCode::Up) => {
                self.help_scroll_offset = self.help_scroll_offset.saturating_sub(1);
                None
            }
            (Some(PopupType::Help), KeyCode::PageDown) => {
                self.help_scroll_offset = self.help_scroll_offset.saturating_add(10);
                None
            }
            (Some(PopupType::Help), KeyCode::PageUp) => {
                self.help_scroll_offset = self.help_scroll_offset.saturating_sub(10);
                None
            }
            (Some(PopupType::ConfirmCancel(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sid = if let Some(Popup {
                    kind: PopupType::ConfirmCancel(s),
                    ..
                }) = self.popup.take()
                {
                    s
                } else {
                    unreachable!()
                };
                Some(Action::CancelJob(sid))
            }
            (Some(PopupType::ConfirmDelete(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sid = if let Some(Popup {
                    kind: PopupType::ConfirmDelete(s),
                    ..
                }) = self.popup.take()
                {
                    s
                } else {
                    unreachable!()
                };
                Some(Action::DeleteJob(sid))
            }
            (Some(PopupType::ConfirmCancelBatch(sids)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sids = sids.clone();
                self.popup = None;
                Some(Action::CancelJobsBatch(sids))
            }
            (Some(PopupType::ConfirmDeleteBatch(sids)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sids = sids.clone();
                self.popup = None;
                Some(Action::DeleteJobsBatch(sids))
            }
            (Some(PopupType::ConfirmEnableApp(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let name = if let Some(Popup {
                    kind: PopupType::ConfirmEnableApp(n),
                    ..
                }) = self.popup.take()
                {
                    n
                } else {
                    unreachable!()
                };
                Some(Action::EnableApp(name))
            }
            (Some(PopupType::ConfirmDisableApp(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let name = if let Some(Popup {
                    kind: PopupType::ConfirmDisableApp(n),
                    ..
                }) = self.popup.take()
                {
                    n
                } else {
                    unreachable!()
                };
                Some(Action::DisableApp(name))
            }
            (Some(PopupType::ExportSearch), KeyCode::Esc) => {
                self.popup = None;
                self.export_target = None;
                None
            }
            (Some(PopupType::ExportSearch), KeyCode::Enter) => {
                if self.export_input.is_empty() {
                    return None;
                }

                if let Some(data) = self.collect_export_data() {
                    let path = std::path::PathBuf::from(&self.export_input);
                    let format = self.export_format;
                    self.popup = None;
                    self.export_target = None;
                    Some(Action::ExportData(data, path, format))
                } else {
                    None
                }
            }
            (Some(PopupType::ExportSearch), KeyCode::Tab) => {
                self.export_format = match self.export_format {
                    ExportFormat::Json => ExportFormat::Csv,
                    ExportFormat::Csv => ExportFormat::Json,
                };
                // Automatically update extension if it matches the previous format
                match self.export_format {
                    ExportFormat::Json => {
                        if self.export_input.ends_with(".csv") {
                            self.export_input.truncate(self.export_input.len() - 4);
                            self.export_input.push_str(".json");
                        }
                    }
                    ExportFormat::Csv => {
                        if self.export_input.ends_with(".json") {
                            self.export_input.truncate(self.export_input.len() - 5);
                            self.export_input.push_str(".csv");
                        }
                    }
                }
                self.update_export_popup();
                None
            }
            (Some(PopupType::ExportSearch), KeyCode::Backspace) => {
                self.export_input.pop();
                self.update_export_popup();
                None
            }
            (Some(PopupType::ExportSearch), KeyCode::Char(c)) => {
                self.export_input.push(c);
                self.update_export_popup();
                None
            }
            (
                Some(PopupType::ErrorDetails),
                KeyCode::Esc | KeyCode::Char('q') | KeyCode::Char('e'),
            ) => {
                self.popup = None;
                None
            }
            (Some(PopupType::ErrorDetails), KeyCode::Char('j') | KeyCode::Down) => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_add(1);
                None
            }
            (Some(PopupType::ErrorDetails), KeyCode::Char('k') | KeyCode::Up) => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_sub(1);
                None
            }
            (Some(PopupType::ErrorDetails), KeyCode::PageDown) => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_add(10);
                None
            }
            (Some(PopupType::ErrorDetails), KeyCode::PageUp) => {
                self.error_scroll_offset = self.error_scroll_offset.saturating_sub(10);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::Esc | KeyCode::Char('q')) => {
                self.popup = None;
                self.index_details_scroll_offset = 0;
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::Char('j') | KeyCode::Down) => {
                self.index_details_scroll_offset =
                    self.index_details_scroll_offset.saturating_add(1);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::Char('k') | KeyCode::Up) => {
                self.index_details_scroll_offset =
                    self.index_details_scroll_offset.saturating_sub(1);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::PageDown) => {
                self.index_details_scroll_offset =
                    self.index_details_scroll_offset.saturating_add(10);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::PageUp) => {
                self.index_details_scroll_offset =
                    self.index_details_scroll_offset.saturating_sub(10);
                None
            }
            (Some(PopupType::IndexDetails), KeyCode::Char('c'))
                if key.modifiers.contains(KeyModifiers::CONTROL) =>
            {
                // Copy index JSON to clipboard
                if let Some(indexes) = &self.indexes
                    && let Some(selected) = self.indexes_state.selected()
                    && let Some(index) = indexes.get(selected)
                    && let Ok(json) = serde_json::to_string_pretty(index)
                {
                    return Some(Action::CopyToClipboard(json));
                }
                None
            }
            (
                Some(
                    PopupType::ConfirmCancel(_)
                    | PopupType::ConfirmDelete(_)
                    | PopupType::ConfirmCancelBatch(_)
                    | PopupType::ConfirmDeleteBatch(_)
                    | PopupType::ConfirmEnableApp(_)
                    | PopupType::ConfirmDisableApp(_),
                ),
                KeyCode::Char('n') | KeyCode::Esc,
            ) => {
                self.popup = None;
                None
            }
            // Profile selector popup handling
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
            // Index creation popup handling
            (Some(PopupType::CreateIndex { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (Some(PopupType::CreateIndex { name_input, .. }), KeyCode::Enter) => {
                if name_input.is_empty() {
                    return None;
                }
                let name = name_input.clone();
                // Extract other fields from the popup state
                if let Some(Popup {
                    kind:
                        PopupType::CreateIndex {
                            max_data_size_mb,
                            max_hot_buckets,
                            max_warm_db_count,
                            frozen_time_period_secs,
                            home_path,
                            cold_db_path,
                            thawed_path,
                            cold_to_frozen_dir,
                            ..
                        },
                    ..
                }) = self.popup.take()
                {
                    let params = splunk_client::CreateIndexParams {
                        name,
                        max_data_size_mb,
                        max_hot_buckets,
                        max_warm_db_count,
                        frozen_time_period_in_secs: frozen_time_period_secs,
                        home_path,
                        cold_db_path,
                        thawed_path,
                        cold_to_frozen_dir,
                    };
                    Some(Action::CreateIndex { params })
                } else {
                    None
                }
            }
            (
                Some(PopupType::CreateIndex {
                    name_input,
                    max_data_size_mb,
                    max_hot_buckets,
                    max_warm_db_count,
                    frozen_time_period_secs,
                    home_path,
                    cold_db_path,
                    thawed_path,
                    cold_to_frozen_dir,
                }),
                KeyCode::Char(c),
            ) => {
                let mut new_name = name_input.clone();
                new_name.push(c);
                self.popup = Some(
                    Popup::builder(PopupType::CreateIndex {
                        name_input: new_name,
                        max_data_size_mb: *max_data_size_mb,
                        max_hot_buckets: *max_hot_buckets,
                        max_warm_db_count: *max_warm_db_count,
                        frozen_time_period_secs: *frozen_time_period_secs,
                        home_path: home_path.clone(),
                        cold_db_path: cold_db_path.clone(),
                        thawed_path: thawed_path.clone(),
                        cold_to_frozen_dir: cold_to_frozen_dir.clone(),
                    })
                    .build(),
                );
                None
            }
            (
                Some(PopupType::CreateIndex {
                    name_input,
                    max_data_size_mb,
                    max_hot_buckets,
                    max_warm_db_count,
                    frozen_time_period_secs,
                    home_path,
                    cold_db_path,
                    thawed_path,
                    cold_to_frozen_dir,
                }),
                KeyCode::Backspace,
            ) => {
                let mut new_name = name_input.clone();
                new_name.pop();
                self.popup = Some(
                    Popup::builder(PopupType::CreateIndex {
                        name_input: new_name,
                        max_data_size_mb: *max_data_size_mb,
                        max_hot_buckets: *max_hot_buckets,
                        max_warm_db_count: *max_warm_db_count,
                        frozen_time_period_secs: *frozen_time_period_secs,
                        home_path: home_path.clone(),
                        cold_db_path: cold_db_path.clone(),
                        thawed_path: thawed_path.clone(),
                        cold_to_frozen_dir: cold_to_frozen_dir.clone(),
                    })
                    .build(),
                );
                None
            }
            // Index modification popup handling
            (Some(PopupType::ModifyIndex { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (Some(PopupType::ModifyIndex { index_name, .. }), KeyCode::Enter) => {
                let name = index_name.clone();
                if let Some(Popup {
                    kind:
                        PopupType::ModifyIndex {
                            new_max_data_size_mb,
                            new_max_hot_buckets,
                            new_max_warm_db_count,
                            new_frozen_time_period_secs,
                            new_home_path,
                            new_cold_db_path,
                            new_thawed_path,
                            new_cold_to_frozen_dir,
                            ..
                        },
                    ..
                }) = self.popup.take()
                {
                    let params = splunk_client::ModifyIndexParams {
                        max_data_size_mb: new_max_data_size_mb,
                        max_hot_buckets: new_max_hot_buckets,
                        max_warm_db_count: new_max_warm_db_count,
                        frozen_time_period_in_secs: new_frozen_time_period_secs,
                        home_path: new_home_path,
                        cold_db_path: new_cold_db_path,
                        thawed_path: new_thawed_path,
                        cold_to_frozen_dir: new_cold_to_frozen_dir,
                    };
                    Some(Action::ModifyIndex { name, params })
                } else {
                    None
                }
            }
            // Index deletion confirmation popup handling
            (
                Some(PopupType::DeleteIndexConfirm { index_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = index_name.clone();
                self.popup = None;
                Some(Action::DeleteIndex { name })
            }
            (Some(PopupType::DeleteIndexConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // User creation popup handling
            (Some(PopupType::CreateUser { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
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
            // User modification popup handling
            (Some(PopupType::ModifyUser { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
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
            // User deletion confirmation popup handling
            (
                Some(PopupType::DeleteUserConfirm { user_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = user_name.clone();
                self.popup = None;
                Some(Action::DeleteUser { name })
            }
            (Some(PopupType::DeleteUserConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // Confirm remove app popup handling
            (Some(PopupType::ConfirmRemoveApp(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let name = if let Some(Popup {
                    kind: PopupType::ConfirmRemoveApp(n),
                    ..
                }) = self.popup.take()
                {
                    n
                } else {
                    unreachable!()
                };
                Some(Action::RemoveApp { app_name: name })
            }
            (Some(PopupType::ConfirmRemoveApp(_)), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // Install app dialog handling
            (Some(PopupType::InstallAppDialog { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (Some(PopupType::InstallAppDialog { file_input }), KeyCode::Enter) => {
                if file_input.is_empty() {
                    return None;
                }
                let path = std::path::PathBuf::from(file_input);
                self.popup = None;
                Some(Action::InstallApp { file_path: path })
            }
            (Some(PopupType::InstallAppDialog { file_input }), KeyCode::Char(c)) => {
                let mut new_input = file_input.clone();
                new_input.push(c);
                self.popup = Some(
                    Popup::builder(PopupType::InstallAppDialog {
                        file_input: new_input,
                    })
                    .build(),
                );
                None
            }
            (Some(PopupType::InstallAppDialog { file_input }), KeyCode::Backspace) => {
                let mut new_input = file_input.clone();
                new_input.pop();
                self.popup = Some(
                    Popup::builder(PopupType::InstallAppDialog {
                        file_input: new_input,
                    })
                    .build(),
                );
                None
            }
            // Profile creation popup handling
            (Some(PopupType::CreateProfile { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
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
                    })
                } else {
                    None
                }
            }
            // Profile field navigation with Tab
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
            // Profile field navigation with Up/Down arrows
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
            // Character input for profile creation
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
            ) => {
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
                                        skip_verify: *skip_verify,
                                        timeout_seconds: val,
                                        max_retries: *max_retries,
                                        use_keyring: *use_keyring,
                                        selected_field: *selected_field,
                                    })
                                    .build(),
                                );
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
                                        skip_verify: *skip_verify,
                                        timeout_seconds: *timeout_seconds,
                                        max_retries: val,
                                        use_keyring: *use_keyring,
                                        selected_field: *selected_field,
                                    })
                                    .build(),
                                );
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
                                    skip_verify: !*skip_verify,
                                    timeout_seconds: *timeout_seconds,
                                    max_retries: *max_retries,
                                    use_keyring: *use_keyring,
                                    selected_field: *selected_field,
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
                                    skip_verify: *skip_verify,
                                    timeout_seconds: *timeout_seconds,
                                    max_retries: *max_retries,
                                    use_keyring: !*use_keyring,
                                    selected_field: *selected_field,
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
                        skip_verify: *skip_verify,
                        timeout_seconds: *timeout_seconds,
                        max_retries: *max_retries,
                        use_keyring: *use_keyring,
                        selected_field: *selected_field,
                    })
                    .build(),
                );
                None
            }
            // Backspace for profile creation
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
            ) => {
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
                                    skip_verify: *skip_verify,
                                    timeout_seconds: val,
                                    max_retries: *max_retries,
                                    use_keyring: *use_keyring,
                                    selected_field: *selected_field,
                                })
                                .build(),
                            );
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
                                    skip_verify: *skip_verify,
                                    timeout_seconds: *timeout_seconds,
                                    max_retries: val,
                                    use_keyring: *use_keyring,
                                    selected_field: *selected_field,
                                })
                                .build(),
                            );
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
                        skip_verify: *skip_verify,
                        timeout_seconds: *timeout_seconds,
                        max_retries: *max_retries,
                        use_keyring: *use_keyring,
                        selected_field: *selected_field,
                    })
                    .build(),
                );
                None
            }
            // Profile edit popup handling - Enter key to save
            (Some(PopupType::EditProfile { name_input, .. }), KeyCode::Enter) => {
                if name_input.is_empty() {
                    return None;
                }
                // Extract all fields from the popup state
                if let Some(Popup {
                    kind:
                        PopupType::EditProfile {
                            original_name: _,
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

                    // If name changed, delete old profile and create new one
                    // This is handled by saving with new name - the old one remains
                    // TODO: Handle rename by deleting old profile after saving new one
                    Some(Action::SaveProfile {
                        name: name_input,
                        profile,
                        use_keyring,
                    })
                } else {
                    None
                }
            }
            (Some(PopupType::EditProfile { .. }), KeyCode::Esc) => {
                self.popup = None;
                None
            }
            // Profile edit field navigation with Tab
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
            // Profile edit field navigation with Up/Down arrows
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
            // Character input for profile edit
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
            ) => {
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
                                        skip_verify: *skip_verify,
                                        timeout_seconds: val,
                                        max_retries: *max_retries,
                                        use_keyring: *use_keyring,
                                        selected_field: *selected_field,
                                    })
                                    .build(),
                                );
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
                                        skip_verify: *skip_verify,
                                        timeout_seconds: *timeout_seconds,
                                        max_retries: val,
                                        use_keyring: *use_keyring,
                                        selected_field: *selected_field,
                                    })
                                    .build(),
                                );
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
                                    skip_verify: !*skip_verify,
                                    timeout_seconds: *timeout_seconds,
                                    max_retries: *max_retries,
                                    use_keyring: *use_keyring,
                                    selected_field: *selected_field,
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
                                    skip_verify: *skip_verify,
                                    timeout_seconds: *timeout_seconds,
                                    max_retries: *max_retries,
                                    use_keyring: !*use_keyring,
                                    selected_field: *selected_field,
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
                        skip_verify: *skip_verify,
                        timeout_seconds: *timeout_seconds,
                        max_retries: *max_retries,
                        use_keyring: *use_keyring,
                        selected_field: *selected_field,
                    })
                    .build(),
                );
                None
            }
            // Backspace for profile edit
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
            ) => {
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
                                    skip_verify: *skip_verify,
                                    timeout_seconds: val,
                                    max_retries: *max_retries,
                                    use_keyring: *use_keyring,
                                    selected_field: *selected_field,
                                })
                                .build(),
                            );
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
                                    skip_verify: *skip_verify,
                                    timeout_seconds: *timeout_seconds,
                                    max_retries: val,
                                    use_keyring: *use_keyring,
                                    selected_field: *selected_field,
                                })
                                .build(),
                            );
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
                        skip_verify: *skip_verify,
                        timeout_seconds: *timeout_seconds,
                        max_retries: *max_retries,
                        use_keyring: *use_keyring,
                        selected_field: *selected_field,
                    })
                    .build(),
                );
                None
            }
            // Profile deletion confirmation
            (
                Some(PopupType::DeleteProfileConfirm { profile_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = profile_name.clone();
                self.popup = None;
                Some(Action::DeleteProfile { name })
            }
            (Some(PopupType::DeleteProfileConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            _ => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::empty())
    }

    #[test]
    fn test_popup_help_close() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::Help).build());

        // Close with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());

        // Reopen and close with 'q'
        app.popup = Some(Popup::builder(PopupType::Help).build());
        let action = app.handle_popup_input(key(KeyCode::Char('q')));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_help_scroll() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::Help).build());
        app.help_scroll_offset = 0;

        // Scroll down with 'j'
        let action = app.handle_popup_input(key(KeyCode::Char('j')));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 1);

        // Scroll down with Down arrow
        let action = app.handle_popup_input(key(KeyCode::Down));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 2);

        // Scroll up with 'k'
        let action = app.handle_popup_input(key(KeyCode::Char('k')));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 1);

        // Scroll up with Up arrow
        let action = app.handle_popup_input(key(KeyCode::Up));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 0);

        // Scroll up at 0 should stay at 0 (saturating_sub)
        let action = app.handle_popup_input(key(KeyCode::Up));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 0);

        // Page down
        let action = app.handle_popup_input(key(KeyCode::PageDown));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 10);

        // Page up
        let action = app.handle_popup_input(key(KeyCode::PageUp));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 0);

        // Page up at 0 should stay at 0 (saturating_sub)
        let action = app.handle_popup_input(key(KeyCode::PageUp));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 0);
    }

    #[test]
    fn test_popup_help_close_resets_scroll() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::Help).build());
        app.help_scroll_offset = 5;

        // Close with Esc should reset scroll offset
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
        assert_eq!(app.help_scroll_offset, 0);

        // Reopen and scroll
        app.popup = Some(Popup::builder(PopupType::Help).build());
        app.help_scroll_offset = 3;

        // Close with 'q' should reset scroll offset
        let action = app.handle_popup_input(key(KeyCode::Char('q')));
        assert!(action.is_none());
        assert!(app.popup.is_none());
        assert_eq!(app.help_scroll_offset, 0);
    }

    #[test]
    fn test_popup_confirm_cancel() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());

        // Confirm with 'y'
        let action = app.handle_popup_input(key(KeyCode::Char('y')));
        assert!(matches!(action, Some(Action::CancelJob(sid)) if sid == "test-sid"));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_confirm_cancel_with_enter() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());

        // Confirm with Enter
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(matches!(action, Some(Action::CancelJob(sid)) if sid == "test-sid"));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_confirm_cancel_reject() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());

        // Reject with 'n'
        let action = app.handle_popup_input(key(KeyCode::Char('n')));
        assert!(action.is_none());
        assert!(app.popup.is_none());

        // Reopen and reject with Esc
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_export_search_input() {
        use crate::app::export::ExportTarget;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ExportSearch).build());
        app.export_target = Some(ExportTarget::SearchResults);
        app.export_input = String::new();

        // Type some characters
        app.handle_popup_input(key(KeyCode::Char('t')));
        app.handle_popup_input(key(KeyCode::Char('e')));
        app.handle_popup_input(key(KeyCode::Char('s')));
        app.handle_popup_input(key(KeyCode::Char('t')));

        assert_eq!(app.export_input, "test");

        // Backspace
        app.handle_popup_input(key(KeyCode::Backspace));
        assert_eq!(app.export_input, "tes");
    }

    #[test]
    fn test_popup_export_search_format_toggle() {
        use crate::app::export::ExportTarget;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ExportSearch).build());
        app.export_target = Some(ExportTarget::SearchResults);
        app.export_input = "test.json".to_string();
        app.export_format = ExportFormat::Json;

        // Toggle format with Tab
        app.handle_popup_input(key(KeyCode::Tab));
        assert_eq!(app.export_format, ExportFormat::Csv);
        assert_eq!(app.export_input, "test.csv");

        // Toggle back
        app.handle_popup_input(key(KeyCode::Tab));
        assert_eq!(app.export_format, ExportFormat::Json);
        assert_eq!(app.export_input, "test.json");
    }

    #[test]
    fn test_popup_export_search_cancel() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ExportSearch).build());
        app.export_target = Some(crate::app::export::ExportTarget::SearchResults);

        // Cancel with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
        assert!(app.export_target.is_none());
    }

    #[test]
    fn test_popup_error_details_navigation() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ErrorDetails).build());
        app.error_scroll_offset = 0;

        // Scroll down
        app.handle_popup_input(key(KeyCode::Char('j')));
        assert_eq!(app.error_scroll_offset, 1);

        // Scroll down more
        app.handle_popup_input(key(KeyCode::Down));
        assert_eq!(app.error_scroll_offset, 2);

        // Page down
        app.handle_popup_input(key(KeyCode::PageDown));
        assert_eq!(app.error_scroll_offset, 12);

        // Scroll up
        app.handle_popup_input(key(KeyCode::Char('k')));
        assert_eq!(app.error_scroll_offset, 11);

        // Page up
        app.handle_popup_input(key(KeyCode::PageUp));
        assert_eq!(app.error_scroll_offset, 1);

        // Close
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_error_details_close_with_e() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ErrorDetails).build());

        // Close with 'e' key (should close the popup)
        let action = app.handle_popup_input(key(KeyCode::Char('e')));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    // Global quit tests (Ctrl+Q from any popup)

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn test_global_quit_from_help_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::Help).build());

        // Ctrl+Q should quit even from help popup
        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_error_details_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ErrorDetails).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_confirm_cancel_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_confirm_delete_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmDelete("test-sid".to_string())).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_confirm_cancel_batch_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::ConfirmCancelBatch(vec![
                "sid1".to_string(),
                "sid2".to_string(),
            ]))
            .build(),
        );

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_confirm_delete_batch_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::ConfirmDeleteBatch(vec![
                "sid1".to_string(),
                "sid2".to_string(),
            ]))
            .build(),
        );

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_confirm_enable_app_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup =
            Some(Popup::builder(PopupType::ConfirmEnableApp("test-app".to_string())).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_confirm_disable_app_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup =
            Some(Popup::builder(PopupType::ConfirmDisableApp("test-app".to_string())).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_export_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ExportSearch).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_index_details_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::IndexDetails).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    // Index creation popup tests

    #[test]
    fn test_popup_create_index_input() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateIndex {
                name_input: String::new(),
                max_data_size_mb: None,
                max_hot_buckets: None,
                max_warm_db_count: None,
                frozen_time_period_secs: None,
                home_path: None,
                cold_db_path: None,
                thawed_path: None,
                cold_to_frozen_dir: None,
            })
            .build(),
        );

        // Type some characters
        app.handle_popup_input(key(KeyCode::Char('t')));
        app.handle_popup_input(key(KeyCode::Char('e')));
        app.handle_popup_input(key(KeyCode::Char('s')));
        app.handle_popup_input(key(KeyCode::Char('t')));

        assert!(
            matches!(app.popup, Some(Popup { kind: PopupType::CreateIndex { ref name_input, .. }, .. }) if name_input == "test")
        );

        // Backspace
        app.handle_popup_input(key(KeyCode::Backspace));
        assert!(
            matches!(app.popup, Some(Popup { kind: PopupType::CreateIndex { ref name_input, .. }, .. }) if name_input == "tes")
        );
    }

    #[test]
    fn test_popup_create_index_submit() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateIndex {
                name_input: "test_index".to_string(),
                max_data_size_mb: Some(1000),
                max_hot_buckets: Some(10),
                max_warm_db_count: None,
                frozen_time_period_secs: None,
                home_path: None,
                cold_db_path: None,
                thawed_path: None,
                cold_to_frozen_dir: None,
            })
            .build(),
        );

        // Submit with Enter
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(
            matches!(action, Some(Action::CreateIndex { params }) if params.name == "test_index" && params.max_data_size_mb == Some(1000))
        );
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_create_index_empty_name() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateIndex {
                name_input: String::new(),
                max_data_size_mb: None,
                max_hot_buckets: None,
                max_warm_db_count: None,
                frozen_time_period_secs: None,
                home_path: None,
                cold_db_path: None,
                thawed_path: None,
                cold_to_frozen_dir: None,
            })
            .build(),
        );

        // Submit with empty name should not emit action
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(action.is_none());
        assert!(app.popup.is_some());
    }

    #[test]
    fn test_popup_create_index_cancel() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateIndex {
                name_input: "test".to_string(),
                max_data_size_mb: None,
                max_hot_buckets: None,
                max_warm_db_count: None,
                frozen_time_period_secs: None,
                home_path: None,
                cold_db_path: None,
                thawed_path: None,
                cold_to_frozen_dir: None,
            })
            .build(),
        );

        // Cancel with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    // Index modification popup tests

    #[test]
    fn test_popup_modify_index_submit() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::ModifyIndex {
                index_name: "main".to_string(),
                current_max_data_size_mb: Some(500000),
                current_max_hot_buckets: Some(10),
                current_max_warm_db_count: Some(300),
                current_frozen_time_period_secs: Some(15552000),
                current_home_path: Some("/splunk/main/db".to_string()),
                current_cold_db_path: Some("/splunk/main/colddb".to_string()),
                current_thawed_path: Some("/splunk/main/thaweddb".to_string()),
                current_cold_to_frozen_dir: None,
                new_max_data_size_mb: Some(2000),
                new_max_hot_buckets: Some(15),
                new_max_warm_db_count: Some(400),
                new_frozen_time_period_secs: Some(2592000),
                new_home_path: None,
                new_cold_db_path: None,
                new_thawed_path: None,
                new_cold_to_frozen_dir: None,
            })
            .build(),
        );

        // Submit with Enter
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(
            matches!(action, Some(Action::ModifyIndex { name, params }) if name == "main" && params.max_data_size_mb == Some(2000))
        );
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_modify_index_cancel() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::ModifyIndex {
                index_name: "main".to_string(),
                current_max_data_size_mb: None,
                current_max_hot_buckets: None,
                current_max_warm_db_count: None,
                current_frozen_time_period_secs: None,
                current_home_path: None,
                current_cold_db_path: None,
                current_thawed_path: None,
                current_cold_to_frozen_dir: None,
                new_max_data_size_mb: None,
                new_max_hot_buckets: None,
                new_max_warm_db_count: None,
                new_frozen_time_period_secs: None,
                new_home_path: None,
                new_cold_db_path: None,
                new_thawed_path: None,
                new_cold_to_frozen_dir: None,
            })
            .build(),
        );

        // Cancel with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    // Index deletion popup tests

    #[test]
    fn test_popup_delete_index_confirm() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::DeleteIndexConfirm {
                index_name: "test_index".to_string(),
            })
            .build(),
        );

        // Confirm with 'y'
        let action = app.handle_popup_input(key(KeyCode::Char('y')));
        assert!(matches!(action, Some(Action::DeleteIndex { name }) if name == "test_index"));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_delete_index_confirm_with_enter() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::DeleteIndexConfirm {
                index_name: "test_index".to_string(),
            })
            .build(),
        );

        // Confirm with Enter
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(matches!(action, Some(Action::DeleteIndex { name }) if name == "test_index"));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_delete_index_cancel() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::DeleteIndexConfirm {
                index_name: "test_index".to_string(),
            })
            .build(),
        );

        // Cancel with 'n'
        let action = app.handle_popup_input(key(KeyCode::Char('n')));
        assert!(action.is_none());
        assert!(app.popup.is_none());

        // Reopen and cancel with Esc
        app.popup = Some(
            Popup::builder(PopupType::DeleteIndexConfirm {
                index_name: "test_index".to_string(),
            })
            .build(),
        );
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_global_quit_from_create_index_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateIndex {
                name_input: String::new(),
                max_data_size_mb: None,
                max_hot_buckets: None,
                max_warm_db_count: None,
                frozen_time_period_secs: None,
                home_path: None,
                cold_db_path: None,
                thawed_path: None,
                cold_to_frozen_dir: None,
            })
            .build(),
        );

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_modify_index_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::ModifyIndex {
                index_name: "main".to_string(),
                current_max_data_size_mb: None,
                current_max_hot_buckets: None,
                current_max_warm_db_count: None,
                current_frozen_time_period_secs: None,
                current_home_path: None,
                current_cold_db_path: None,
                current_thawed_path: None,
                current_cold_to_frozen_dir: None,
                new_max_data_size_mb: None,
                new_max_hot_buckets: None,
                new_max_warm_db_count: None,
                new_frozen_time_period_secs: None,
                new_home_path: None,
                new_cold_db_path: None,
                new_thawed_path: None,
                new_cold_to_frozen_dir: None,
            })
            .build(),
        );

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_delete_index_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::DeleteIndexConfirm {
                index_name: "test_index".to_string(),
            })
            .build(),
        );

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }
}
