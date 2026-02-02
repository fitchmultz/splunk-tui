//! Profile switching and management side effect handlers.
//!
//! Responsibilities:
//! - Handle async operations for profile switching.
//! - Handle profile CRUD operations (create, update, delete).
//! - Load profile configurations and rebuild clients.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use crate::app::ConnectionContext;
use crate::ui::ToastLevel;
use splunk_client::{AuthStrategy, SplunkClient};
use splunk_config::{ConfigManager, ProfileConfig};
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc::Sender};

use super::SharedClient;

/// Handle switching to settings screen.
pub async fn handle_switch_to_settings(
    config_manager: Arc<Mutex<ConfigManager>>,
    tx: Sender<Action>,
) {
    let _ = tx.send(Action::Loading(true)).await;
    tokio::spawn(async move {
        let cm = config_manager.lock().await;
        let state = cm.load();
        let _ = tx.send(Action::SettingsLoaded(state)).await;
    });
}

/// Handle opening the profile switcher popup.
pub async fn handle_open_profile_switcher(
    config_manager: Arc<Mutex<ConfigManager>>,
    tx: Sender<Action>,
) {
    let config_manager_clone = config_manager.clone();
    let tx_popup = tx.clone();
    tokio::spawn(async move {
        let cm = config_manager_clone.lock().await;
        let profiles: Vec<String> = cm.list_profiles().keys().cloned().collect();
        drop(cm); // Release lock before sending actions

        if profiles.is_empty() {
            let _ = tx_popup
                .send(Action::Notify(
                    ToastLevel::Error,
                    "No profiles configured. Add profiles using splunk-cli.".to_string(),
                ))
                .await;
        } else {
            // Send the profile list to be opened as a popup
            let _ = tx_popup
                .send(Action::OpenProfileSelectorWithList(profiles))
                .await;
        }
    });
}

/// Handle profile selection and client rebuild.
pub async fn handle_profile_selected(
    client: SharedClient,
    config_manager: Arc<Mutex<ConfigManager>>,
    tx: Sender<Action>,
    profile_name: String,
) {
    let _ = tx.send(Action::Loading(true)).await;
    let config_manager_clone = config_manager.clone();
    let client_clone = client.clone();
    tokio::spawn(async move {
        let cm = config_manager_clone.lock().await;

        // Get the profile config
        let profiles = cm.list_profiles();
        let Some(profile_config) = profiles.get(&profile_name) else {
            let _ = tx
                .send(Action::ProfileSwitchResult(Err(format!(
                    "Profile '{}' not found",
                    profile_name
                ))))
                .await;
            return;
        };

        // Build new config from profile
        let Some(base_url) = profile_config.base_url.clone() else {
            let _ = tx
                .send(Action::ProfileSwitchResult(Err(
                    "Profile has no base_url configured".to_string(),
                )))
                .await;
            return;
        };
        let auth_strategy = if let Some(token) = &profile_config.api_token {
            // API token auth
            match token.resolve() {
                Ok(resolved_token) => AuthStrategy::ApiToken {
                    token: resolved_token,
                },
                Err(e) => {
                    let _ = tx
                        .send(Action::ProfileSwitchResult(Err(format!(
                            "Failed to resolve API token: {}",
                            e
                        ))))
                        .await;
                    return;
                }
            }
        } else if let (Some(username), Some(password)) =
            (&profile_config.username, &profile_config.password)
        {
            // Session token auth
            match password.resolve() {
                Ok(resolved_password) => AuthStrategy::SessionToken {
                    username: username.clone(),
                    password: resolved_password,
                },
                Err(e) => {
                    let _ = tx
                        .send(Action::ProfileSwitchResult(Err(format!(
                            "Failed to resolve password: {}",
                            e
                        ))))
                        .await;
                    return;
                }
            }
        } else {
            let _ = tx
                .send(Action::ProfileSwitchResult(Err(
                    "Profile has no authentication configured".to_string(),
                )))
                .await;
            return;
        };

        // Build new client
        let mut new_client = match SplunkClient::builder()
            .base_url(base_url.clone())
            .auth_strategy(auth_strategy)
            .skip_verify(profile_config.skip_verify.unwrap_or(false))
            .timeout(std::time::Duration::from_secs(
                profile_config.timeout_seconds.unwrap_or(30),
            ))
            .build()
        {
            Ok(c) => c,
            Err(e) => {
                let _ = tx
                    .send(Action::ProfileSwitchResult(Err(format!(
                        "Failed to build client: {}",
                        e
                    ))))
                    .await;
                return;
            }
        };

        // Authenticate if using session tokens
        if !new_client.is_api_token_auth()
            && let Err(e) = new_client.login().await
        {
            let _ = tx
                .send(Action::ProfileSwitchResult(Err(format!(
                    "Authentication failed: {}",
                    e
                ))))
                .await;
            return;
        }

        // Replace the shared client
        let mut client_guard = client_clone.lock().await;
        *client_guard = new_client;
        drop(client_guard);

        // Determine auth mode display string
        let auth_mode = if profile_config.api_token.is_some() {
            "token".to_string()
        } else if let Some(username) = &profile_config.username {
            format!("session ({})", username)
        } else {
            "unknown".to_string()
        };

        // Send success result
        let ctx = ConnectionContext {
            profile_name: Some(profile_name.clone()),
            base_url,
            auth_mode,
        };
        let _ = tx.send(Action::ProfileSwitchResult(Ok(ctx))).await;

        // Clear all cached data
        let _ = tx.send(Action::ClearAllData).await;

        // Trigger reload for current screen
        let _ = tx.send(Action::Loading(false)).await;
    });
}

/// Handle opening the profile edit dialog by loading profile data.
pub async fn handle_open_edit_profile(
    config_manager: Arc<Mutex<ConfigManager>>,
    tx: Sender<Action>,
    profile_name: String,
) {
    let config_manager_clone = config_manager.clone();
    let tx_clone = tx.clone();
    let name_clone = profile_name.clone();

    tokio::spawn(async move {
        let cm = config_manager_clone.lock().await;

        // Get the profile config
        let profiles = cm.list_profiles();
        let Some(profile_config) = profiles.get(&name_clone) else {
            let _ = tx_clone
                .send(Action::Notify(
                    ToastLevel::Error,
                    format!("Profile '{}' not found", name_clone),
                ))
                .await;
            return;
        };

        // Send action to open dialog with profile data
        let _ = tx_clone
            .send(Action::OpenEditProfileDialogWithData {
                original_name: name_clone.clone(),
                name_input: name_clone,
                base_url_input: profile_config.base_url.clone().unwrap_or_default(),
                username_input: profile_config.username.clone().unwrap_or_default(),
                skip_verify: profile_config.skip_verify.unwrap_or(false),
                timeout_seconds: profile_config.timeout_seconds.unwrap_or(30),
                max_retries: profile_config.max_retries.unwrap_or(3),
            })
            .await;
    });
}

/// Handle saving/creating a profile.
pub async fn handle_save_profile(
    config_manager: Arc<Mutex<ConfigManager>>,
    tx: Sender<Action>,
    name: String,
    profile: ProfileConfig,
    use_keyring: bool,
    original_name: Option<String>,
) {
    let config_manager_clone = config_manager.clone();
    let tx_clone = tx.clone();
    let name_clone = name.clone();
    let original_name_clone = original_name.clone();

    tokio::spawn(async move {
        let mut cm = config_manager_clone.lock().await;

        // If use_keyring is enabled, store credentials in keyring before saving
        let mut profile_to_save = profile.clone();

        if use_keyring {
            // Store password in keyring if it's a plain value
            if let (Some(username), Some(splunk_config::types::SecureValue::Plain(pw))) =
                (&profile.username, &profile.password)
            {
                match cm.store_password_in_keyring(&name_clone, username, pw) {
                    Ok(keyring_value) => {
                        profile_to_save.password = Some(keyring_value);
                    }
                    Err(e) => {
                        let _ = tx_clone
                            .send(Action::Notify(
                                ToastLevel::Warning,
                                format!(
                                    "Failed to store password in keyring: {}. Saving as plaintext.",
                                    e
                                ),
                            ))
                            .await;
                    }
                }
            }

            // Store API token in keyring if it's a plain value
            if let Some(splunk_config::types::SecureValue::Plain(token)) = &profile.api_token {
                match cm.store_token_in_keyring(&name_clone, token) {
                    Ok(keyring_value) => {
                        profile_to_save.api_token = Some(keyring_value);
                    }
                    Err(e) => {
                        let _ = tx_clone
                            .send(Action::Notify(
                                ToastLevel::Warning,
                                format!("Failed to store API token in keyring: {}. Saving as plaintext.", e),
                            ))
                            .await;
                    }
                }
            }
        }

        // Save the profile
        match cm.save_profile(&name_clone, profile_to_save) {
            Ok(()) => {
                // Handle rename: delete old profile after saving new one
                if let Some(old_name) = original_name_clone
                    && old_name != name_clone
                {
                    if let Err(e) = cm.delete_profile(&old_name) {
                        // Log error but don't fail the save operation
                        let _ = tx_clone
                            .send(Action::Notify(
                                ToastLevel::Warning,
                                format!(
                                    "Profile saved but failed to remove old profile '{}': {}",
                                    old_name, e
                                ),
                            ))
                            .await;
                    } else {
                        let _ = tx_clone
                            .send(Action::Notify(
                                ToastLevel::Info,
                                format!("Old profile '{}' removed after rename", old_name),
                            ))
                            .await;
                    }
                }

                let _ = tx_clone
                    .send(Action::ProfileSaved(Ok(name_clone.clone())))
                    .await;
                let _ = tx_clone
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("Profile '{}' saved successfully", name_clone),
                    ))
                    .await;
            }
            Err(e) => {
                let error_msg = format!("Failed to save profile '{}': {}", name_clone, e);
                let _ = tx_clone
                    .send(Action::ProfileSaved(Err(error_msg.clone())))
                    .await;
                let _ = tx_clone
                    .send(Action::Notify(ToastLevel::Error, error_msg))
                    .await;
            }
        }
    });
}

/// Handle deleting a profile.
pub async fn handle_delete_profile(
    config_manager: Arc<Mutex<ConfigManager>>,
    tx: Sender<Action>,
    name: String,
) {
    let config_manager_clone = config_manager.clone();
    let tx_clone = tx.clone();
    let name_clone = name.clone();

    tokio::spawn(async move {
        let mut cm = config_manager_clone.lock().await;

        match cm.delete_profile(&name_clone) {
            Ok(()) => {
                let _ = tx_clone
                    .send(Action::ProfileDeleted(Ok(name_clone.clone())))
                    .await;
                let _ = tx_clone
                    .send(Action::Notify(
                        ToastLevel::Success,
                        format!("Profile '{}' deleted successfully", name_clone),
                    ))
                    .await;
            }
            Err(e) => {
                let error_msg = format!("Failed to delete profile '{}': {}", name_clone, e);
                let _ = tx_clone
                    .send(Action::ProfileDeleted(Err(error_msg.clone())))
                    .await;
                let _ = tx_clone
                    .send(Action::Notify(ToastLevel::Error, error_msg))
                    .await;
            }
        }
    });
}
