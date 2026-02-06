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

        // Get profile config
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

        // Build auth strategy
        let auth_strategy = match build_auth_strategy_from_profile(profile_config) {
            Ok(s) => s,
            Err(e) => {
                let _ = tx.send(Action::ProfileSwitchResult(Err(e))).await;
                return;
            }
        };

        // Get base_url
        let Some(base_url) = profile_config.base_url.clone() else {
            let _ = tx
                .send(Action::ProfileSwitchResult(Err(
                    "Profile has no base_url configured".to_string(),
                )))
                .await;
            return;
        };

        // Build client
        let mut new_client = match build_client_for_profile(profile_config, auth_strategy) {
            Ok(c) => c,
            Err(e) => {
                let _ = tx.send(Action::ProfileSwitchResult(Err(e))).await;
                return;
            }
        };

        // Authenticate if needed
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

        // Replace shared client
        let mut client_guard = client_clone.lock().await;
        *client_guard = new_client;
        drop(client_guard);

        // Build success result
        let auth_mode = get_auth_mode_display(profile_config);
        let ctx = ConnectionContext {
            profile_name: Some(profile_name.clone()),
            base_url,
            auth_mode,
        };
        let _ = tx.send(Action::ProfileSwitchResult(Ok(ctx))).await;
        let _ = tx.send(Action::ClearAllData).await;
        let _ = tx.send(Action::Loading(false)).await;
    });
}

/// Build authentication strategy from profile configuration.
/// Returns Err with user-friendly message if auth cannot be built.
fn build_auth_strategy_from_profile(
    profile_config: &ProfileConfig,
) -> Result<AuthStrategy, String> {
    // Check for API token first
    if let Some(ref token_secure) = profile_config.api_token {
        match token_secure.resolve() {
            Ok(token) => return Ok(AuthStrategy::ApiToken { token }),
            Err(e) => return Err(format!("Failed to resolve API token: {}", e)),
        }
    }

    // Fall back to username/password
    if let (Some(username), Some(password_secure)) =
        (&profile_config.username, &profile_config.password)
    {
        match password_secure.resolve() {
            Ok(password) => {
                return Ok(AuthStrategy::SessionToken {
                    username: username.clone(),
                    password,
                });
            }
            Err(e) => return Err(format!("Failed to resolve password: {}", e)),
        }
    }

    Err("Profile has no authentication configured".to_string())
}

/// Build SplunkClient from profile configuration.
fn build_client_for_profile(
    profile_config: &ProfileConfig,
    auth_strategy: AuthStrategy,
) -> Result<SplunkClient, String> {
    let base_url = profile_config
        .base_url
        .clone()
        .ok_or_else(|| "Profile has no base_url configured".to_string())?;

    SplunkClient::builder()
        .base_url(base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(profile_config.skip_verify.unwrap_or(false))
        .timeout(std::time::Duration::from_secs(
            profile_config.timeout_seconds.unwrap_or(30),
        ))
        .build()
        .map_err(|e| format!("Failed to build client: {}", e))
}

/// Determine auth mode display string for connection context.
fn get_auth_mode_display(profile_config: &ProfileConfig) -> String {
    if profile_config.api_token.is_some() {
        "token".to_string()
    } else if let Some(username) = &profile_config.username {
        format!("session ({})", username)
    } else {
        "unknown".to_string()
    }
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

        // Store credentials in keyring if enabled
        let profile_to_save = if use_keyring {
            store_profile_credentials_in_keyring(&cm, &tx_clone, &name_clone, profile).await
        } else {
            profile
        };

        // Save the profile
        let result = cm
            .save_profile(&name_clone, profile_to_save)
            .map_err(|e| e.to_string());

        // Handle rename if needed
        if result.is_ok() {
            if let Some(old_name) = original_name_clone.filter(|old| old != &name_clone) {
                handle_profile_rename(&mut cm, &tx_clone, &old_name, &name_clone).await;
            }
        }

        send_profile_save_result(&tx_clone, &name_clone, result).await;
    });
}

/// Store profile credentials in keyring if enabled.
/// Returns the profile with credentials potentially converted to keyring storage.
/// Sends warning notifications for any failures.
async fn store_profile_credentials_in_keyring(
    cm: &ConfigManager,
    tx: &Sender<Action>,
    profile_name: &str,
    mut profile: ProfileConfig,
) -> ProfileConfig {
    // Store password in keyring
    if let (Some(username), Some(splunk_config::types::SecureValue::Plain(pw))) =
        (&profile.username, &profile.password)
    {
        let keyring_value = cm.try_store_password_in_keyring(profile_name, username, pw);
        if matches!(keyring_value, splunk_config::types::SecureValue::Plain(_)) {
            let _ = tx
                .send(Action::Notify(
                    ToastLevel::Warning,
                    "Failed to store password in keyring. Saving as plaintext.".to_string(),
                ))
                .await;
        }
        profile.password = Some(keyring_value);
    }

    // Store API token in keyring
    if let Some(splunk_config::types::SecureValue::Plain(token)) = &profile.api_token {
        let keyring_value = cm.try_store_token_in_keyring(profile_name, token);
        if matches!(keyring_value, splunk_config::types::SecureValue::Plain(_)) {
            let _ = tx
                .send(Action::Notify(
                    ToastLevel::Warning,
                    "Failed to store API token in keyring. Saving as plaintext.".to_string(),
                ))
                .await;
        }
        profile.api_token = Some(keyring_value);
    }

    profile
}

/// Handle profile rename: delete old profile after successful save.
async fn handle_profile_rename(
    cm: &mut ConfigManager,
    tx: &Sender<Action>,
    old_name: &str,
    _new_name: &str,
) {
    if let Err(e) = cm.delete_profile(old_name) {
        let _ = tx
            .send(Action::Notify(
                ToastLevel::Warning,
                format!(
                    "Profile saved but failed to remove old profile '{}': {}",
                    old_name, e
                ),
            ))
            .await;
    } else {
        let _ = tx
            .send(Action::Notify(
                ToastLevel::Info,
                format!("Old profile '{}' removed after rename", old_name),
            ))
            .await;
    }
}

/// Send notifications for profile save result.
async fn send_profile_save_result(
    tx: &Sender<Action>,
    profile_name: &str,
    result: Result<(), String>,
) {
    match result {
        Ok(()) => {
            let _ = tx
                .send(Action::ProfileSaved(Ok(profile_name.to_string())))
                .await;
            let _ = tx
                .send(Action::Notify(
                    ToastLevel::Success,
                    format!("Profile '{}' saved successfully", profile_name),
                ))
                .await;
        }
        Err(e) => {
            let error_msg = format!("Failed to save profile '{}': {}", profile_name, e);
            let _ = tx.send(Action::ProfileSaved(Err(error_msg.clone()))).await;
            let _ = tx.send(Action::Notify(ToastLevel::Error, error_msg)).await;
        }
    }
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
