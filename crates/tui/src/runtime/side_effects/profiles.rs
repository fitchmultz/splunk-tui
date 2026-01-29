//! Profile switching side effect handlers.
//!
//! Responsibilities:
//! - Handle async operations for profile switching.
//! - Load profile configurations and rebuild clients.
//!
//! Does NOT handle:
//! - Direct state modification (sends actions for that).
//! - UI rendering.

use crate::action::Action;
use crate::app::ConnectionContext;
use crate::ui::ToastLevel;
use splunk_client::{AuthStrategy, SplunkClient};
use splunk_config::ConfigManager;
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
