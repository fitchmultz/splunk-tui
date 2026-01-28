//! Async side effect handlers for TUI actions.
//!
//! Responsibilities:
//! - Handle async API calls triggered by user actions.
//! - Spawn background tasks for data fetching to avoid blocking the UI.
//! - Send results back via the action channel for state updates.
//!
//! Does NOT handle:
//! - Direct application state modification (sends actions to do that).
//! - UI rendering or terminal management.
//! - Configuration loading or persistence.
//!
//! Invariants / Assumptions:
//! - All API calls are spawned as separate tokio tasks.
//! - Results are always sent back via the action channel.
//! - Loading state is set before API calls and cleared after.

use splunk_client::{AuthStrategy, SplunkClient, models::HealthCheckOutput};
use splunk_config::ConfigManager;
use splunk_tui::action::{Action, progress_callback_to_action_sender};
use splunk_tui::app::ConnectionContext;
use splunk_tui::error_details::{build_search_error_details, search_error_message};
use splunk_tui::ui::ToastLevel;
use std::sync::Arc;
use tokio::sync::{Mutex, mpsc::Sender};

/// Shared client wrapper for async tasks.
pub type SharedClient = Arc<Mutex<SplunkClient>>;

/// Handle side effects (async API calls) for actions.
///
/// This function spawns background tasks for API operations and sends
/// results back through the action channel. It handles:
/// - Data loading (indexes, jobs, cluster info, etc.)
/// - Search execution with progress callbacks
/// - Job operations (cancel, delete, batch operations)
/// - App operations (enable, disable)
/// - Health checks
/// - Data export
/// - Profile switching
///
/// # Arguments
///
/// * `action` - The action to handle
/// * `client` - The shared Splunk client
/// * `tx` - The action channel sender for sending results
/// * `config_manager` - The configuration manager for profile operations
pub async fn handle_side_effects(
    action: Action,
    client: SharedClient,
    tx: Sender<Action>,
    config_manager: Arc<Mutex<ConfigManager>>,
) {
    match action {
        Action::LoadIndexes => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_indexes(None, None).await {
                    Ok(indexes) => {
                        let _ = tx.send(Action::IndexesLoaded(Ok(indexes))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::IndexesLoaded(Err(e.to_string()))).await;
                    }
                }
            });
        }
        Action::LoadJobs => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_jobs(None, None).await {
                    Ok(jobs) => {
                        let _ = tx.send(Action::JobsLoaded(Ok(jobs))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::JobsLoaded(Err(e.to_string()))).await;
                    }
                }
            });
        }
        Action::LoadClusterInfo => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.get_cluster_info().await {
                    Ok(info) => {
                        let _ = tx.send(Action::ClusterInfoLoaded(Ok(info))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::ClusterInfoLoaded(Err(e.to_string()))).await;
                    }
                }
            });
        }
        Action::LoadClusterPeers => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.get_cluster_peers().await {
                    Ok(peers) => {
                        let _ = tx.send(Action::ClusterPeersLoaded(Ok(peers))).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::ClusterPeersLoaded(Err(e.to_string())))
                            .await;
                    }
                }
            });
        }
        Action::LoadSavedSearches => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_saved_searches().await {
                    Ok(searches) => {
                        let _ = tx.send(Action::SavedSearchesLoaded(Ok(searches))).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::SavedSearchesLoaded(Err(e.to_string())))
                            .await;
                    }
                }
            });
        }
        Action::LoadInternalLogs => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                // Default to last 15 minutes of logs, 100 entries
                match c.get_internal_logs(100, Some("-15m")).await {
                    Ok(logs) => {
                        let _ = tx.send(Action::InternalLogsLoaded(Ok(logs))).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::InternalLogsLoaded(Err(e.to_string())))
                            .await;
                    }
                }
            });
        }
        Action::LoadApps => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_apps(None, None).await {
                    Ok(apps) => {
                        let _ = tx.send(Action::AppsLoaded(Ok(apps))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::AppsLoaded(Err(e.to_string()))).await;
                    }
                }
            });
        }
        Action::LoadUsers => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_users(None, None).await {
                    Ok(users) => {
                        let _ = tx.send(Action::UsersLoaded(Ok(users))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::UsersLoaded(Err(e.to_string()))).await;
                    }
                }
            });
        }
        Action::SwitchToSettings => {
            let _ = tx.send(Action::Loading(true)).await;
            let config_manager_clone = config_manager.clone();
            tokio::spawn(async move {
                let cm = config_manager_clone.lock().await;
                let state = cm.load();
                let _ = tx.send(Action::SettingsLoaded(state)).await;
            });
        }
        Action::RunSearch {
            query,
            search_defaults,
        } => {
            let _ = tx.send(Action::Loading(true)).await;
            let _ = tx.send(Action::Progress(0.1)).await;

            // Store the query that is about to run for accurate status messages
            let _ = tx.send(Action::SearchStarted(query.clone())).await;

            let tx_clone = tx.clone();
            let query_clone = query.clone();
            tokio::spawn(async move {
                let mut c = client.lock().await;

                // Create progress callback that sends Action::Progress via channel
                let progress_tx = tx_clone.clone();
                let mut progress_callback = progress_callback_to_action_sender(progress_tx);

                // Use search_with_progress for unified timeout and progress handling
                match c
                    .search_with_progress(
                        &query_clone,
                        true, // wait for completion
                        Some(&search_defaults.earliest_time),
                        Some(&search_defaults.latest_time),
                        Some(search_defaults.max_results),
                        Some(&mut progress_callback),
                    )
                    .await
                {
                    Ok((results, sid, total)) => {
                        let _ = tx_clone.send(Action::Progress(1.0)).await;
                        let _ = tx_clone
                            .send(Action::SearchComplete(Ok((results, sid, total))))
                            .await;
                    }
                    Err(e) => {
                        let details = build_search_error_details(
                            &e,
                            query_clone,
                            "search_with_progress".to_string(),
                            None, // SID not available on failure
                        );
                        let error_msg = search_error_message(&e);
                        // Error details stored in SearchComplete handler; user can press 'e' to view
                        let _ = tx_clone
                            .send(Action::SearchComplete(Err((error_msg, details))))
                            .await;
                    }
                }
            });
        }
        Action::LoadMoreSearchResults { sid, offset, count } => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.get_search_results(&sid, count, offset).await {
                    Ok(results) => {
                        let _ = tx
                            .send(Action::MoreSearchResultsLoaded(Ok((
                                results.results,
                                offset,
                                results.total,
                            ))))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::MoreSearchResultsLoaded(Err(e.to_string())))
                            .await;
                    }
                }
            });
        }
        Action::CancelJob(sid) => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.cancel_job(&sid).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::JobOperationComplete(format!(
                                "Cancelled job: {}",
                                sid
                            )))
                            .await;
                        // Reload the job list
                        let _ = tx.send(Action::LoadJobs).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Error,
                                format!("Failed to cancel job: {}", e),
                            ))
                            .await;
                        let _ = tx.send(Action::Loading(false)).await;
                    }
                }
            });
        }
        Action::DeleteJob(sid) => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.delete_job(&sid).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::JobOperationComplete(format!(
                                "Deleted job: {}",
                                sid
                            )))
                            .await;
                        // Reload the job list
                        let _ = tx.send(Action::LoadJobs).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Error,
                                format!("Failed to delete job: {}", e),
                            ))
                            .await;
                        let _ = tx.send(Action::Loading(false)).await;
                    }
                }
            });
        }
        Action::CancelJobsBatch(sids) => {
            let _ = tx.send(Action::Loading(true)).await;
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                let mut success_count = 0;
                let mut error_messages = Vec::new();

                for sid in sids {
                    match c.cancel_job(&sid).await {
                        Ok(_) => {
                            success_count += 1;
                        }
                        Err(e) => {
                            error_messages.push(format!("{}: {}", sid, e));
                        }
                    }
                }

                let msg = if success_count > 0 {
                    format!("Cancelled {} job(s)", success_count)
                } else {
                    "No jobs cancelled".to_string()
                };

                if !error_messages.is_empty() {
                    for err in error_messages {
                        let _ = tx_clone.send(Action::Notify(ToastLevel::Error, err)).await;
                    }
                }

                let _ = tx_clone.send(Action::JobOperationComplete(msg)).await;
                let _ = tx_clone.send(Action::LoadJobs).await;
            });
        }
        Action::DeleteJobsBatch(sids) => {
            let _ = tx.send(Action::Loading(true)).await;
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                let mut success_count = 0;
                let mut error_messages = Vec::new();

                for sid in sids {
                    match c.delete_job(&sid).await {
                        Ok(_) => {
                            success_count += 1;
                        }
                        Err(e) => {
                            error_messages.push(format!("{}: {}", sid, e));
                        }
                    }
                }

                let msg = if success_count > 0 {
                    format!("Deleted {} job(s)", success_count)
                } else {
                    "No jobs deleted".to_string()
                };

                if !error_messages.is_empty() {
                    for err in error_messages {
                        let _ = tx_clone.send(Action::Notify(ToastLevel::Error, err)).await;
                    }
                }

                let _ = tx_clone.send(Action::JobOperationComplete(msg)).await;
                let _ = tx_clone.send(Action::LoadJobs).await;
            });
        }
        Action::EnableApp(name) => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.enable_app(&name).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Success,
                                format!("App '{}' enabled successfully", name),
                            ))
                            .await;
                        // Refresh apps list
                        let _ = tx.send(Action::LoadApps).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Error,
                                format!("Failed to enable app '{}': {}", name, e),
                            ))
                            .await;
                        let _ = tx.send(Action::Loading(false)).await;
                    }
                }
            });
        }
        Action::DisableApp(name) => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.disable_app(&name).await {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Success,
                                format!("App '{}' disabled successfully", name),
                            ))
                            .await;
                        // Refresh apps list
                        let _ = tx.send(Action::LoadApps).await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Error,
                                format!("Failed to disable app '{}': {}", name, e),
                            ))
                            .await;
                        let _ = tx.send(Action::Loading(false)).await;
                    }
                }
            });
        }
        Action::LoadHealth => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;

                // Construct the HealthCheckOutput
                let mut health_output = HealthCheckOutput {
                    server_info: None,
                    splunkd_health: None,
                    license_usage: None,
                    kvstore_status: None,
                    log_parsing_health: None,
                };

                let mut has_error = false;
                let mut error_messages = Vec::new();

                // Collect health info sequentially (due to &mut self requirement)
                match c.get_server_info().await {
                    Ok(info) => health_output.server_info = Some(info),
                    Err(e) => {
                        has_error = true;
                        error_messages.push(format!("Server info: {}", e));
                    }
                }

                match c.get_health().await {
                    Ok(health) => health_output.splunkd_health = Some(health),
                    Err(e) => {
                        has_error = true;
                        error_messages.push(format!("Splunkd health: {}", e));
                    }
                }

                match c.get_license_usage().await {
                    Ok(license) => health_output.license_usage = Some(license),
                    Err(e) => {
                        has_error = true;
                        error_messages.push(format!("License usage: {}", e));
                    }
                }

                match c.get_kvstore_status().await {
                    Ok(kvstore) => health_output.kvstore_status = Some(kvstore),
                    Err(e) => {
                        has_error = true;
                        error_messages.push(format!("KVStore status: {}", e));
                    }
                }

                match c.check_log_parsing_health().await {
                    Ok(log_parsing) => health_output.log_parsing_health = Some(log_parsing),
                    Err(e) => {
                        has_error = true;
                        error_messages.push(format!("Log parsing health: {}", e));
                    }
                }

                if has_error {
                    let combined_error = error_messages.join("; ");
                    let _ = tx
                        .send(Action::HealthLoaded(Box::new(Err(combined_error))))
                        .await;
                } else {
                    let _ = tx
                        .send(Action::HealthLoaded(Box::new(Ok(health_output))))
                        .await;
                }
            });
        }
        Action::ExportData(data, path, format) => {
            tokio::spawn(async move {
                let result = splunk_tui::export::export_value(&data, &path, format);

                match result {
                    Ok(_) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Info,
                                format!("Exported to {}", path.display()),
                            ))
                            .await;
                    }
                    Err(e) => {
                        let _ = tx
                            .send(Action::Notify(
                                ToastLevel::Error,
                                format!("Export failed: {}", e),
                            ))
                            .await;
                    }
                }
            });
        }
        // Profile switching actions
        Action::OpenProfileSwitcher => {
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
        Action::ProfileSelected(profile_name) => {
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
        _ => {}
    }
}
