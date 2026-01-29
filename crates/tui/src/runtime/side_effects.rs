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
//!
//! # Design Rationale: Task Spawning Pattern
//!
//! This module uses `tokio::spawn` for all async operations (21 handlers as of
//! this writing). This design is intentional and addresses specific constraints:
//!
//! ## Why Spawn Tasks?
//!
//! 1. **UI Responsiveness**: The TUI event loop must never block. Even brief
//!    async operations (like acquiring a mutex) can cause frame drops if they
//!    contend with the render thread.
//!
//! 2. **Consistent Error Boundaries**: Each spawned task is an isolated failure
//!    domain. A panic in one API call handler won't crash the entire application.
//!
//! 3. **Cancellation Safety**: Tasks can be dropped without cleanup concerns
//!    (the client mutex is released on drop, and API calls are stateless).
//!
//! ## The Mutex Bottleneck
//!
//! All API calls share a single `Arc<Mutex<SplunkClient>>`. This means:
//! - **API calls are serialized** regardless of how many tasks are spawned
//! - Multiple concurrent tasks simply queue for the client lock
//! - Task spawn overhead is negligible compared to network I/O latency
//!
//! This is a deliberate trade-off: the SplunkClient requires `&mut self` for
//! session token refresh, so true parallel API calls would require significant
//! architectural changes (e.g., connection pooling or token refresh decoupling).
//!
//! ## Sequential Operations
//!
//! Some operations intentionally sequential:
//!
//! - **Health checks** (`LoadHealth`): 5 API calls run sequentially within one
//!   spawned task due to the `&mut self` requirement. Parallelizing would require
//!   either spawning 5 separate tasks (each waiting for the lock) or refactoring
//!   the client to support concurrent access.
//!
//! - **Batch operations** (`CancelJobsBatch`, `DeleteJobsBatch`): Jobs are
//!   processed sequentially to avoid overwhelming the Splunk API and to provide
//!   clear per-job error reporting.
//!
//! ## Performance Considerations
//!
//! Tokio task spawning has minimal overhead (~microseconds). Given that:
//! - Network I/O dominates latency (milliseconds to seconds)
//! - The client mutex serializes actual API calls
//! - No measured bottleneck exists in task scheduling
//!
//! The current pattern is not a performance concern. Optimization would only be
//! warranted if profiling shows significant time in task scheduling overhead.
//!
//! ## Future Optimization Paths
//!
//! If performance data indicates a need:
//!
//! 1. **Semaphore-based limiting**: Add a `tokio::sync::Semaphore` to cap
//!    concurrent spawned tasks (prevents unbounded memory growth under load).
//!
//! 2. **Non-API operations**: `SwitchToSettings`, `ExportData`, and
//!    `OpenProfileSwitcher` don't make API calls and could run without spawn.
//!
//! 3. **Parallel health checks**: Spawn separate tasks per health endpoint
//!    (each would still serialize on the client lock, but they'd pipeline better).
//!
//! 4. **Parallel batch operations**: Use `futures::future::join_all` for batch
//!    job operations (with rate limiting to avoid API throttling).

use crate::action::{Action, progress_callback_to_action_sender};
use crate::app::ConnectionContext;
use crate::error_details::{build_search_error_details, search_error_message};
use crate::ui::ToastLevel;
use splunk_client::{AuthStrategy, ClientError, SplunkClient, models::HealthCheckOutput};
use splunk_config::ConfigManager;
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
        Action::LoadIndexes { count, offset } => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_indexes(Some(count), Some(offset)).await {
                    Ok(indexes) => {
                        let _ = tx.send(Action::IndexesLoaded(Ok(indexes))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::IndexesLoaded(Err(Arc::new(e)))).await;
                    }
                }
            });
        }
        Action::LoadJobs { count, offset } => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_jobs(Some(count), Some(offset)).await {
                    Ok(jobs) => {
                        let _ = tx.send(Action::JobsLoaded(Ok(jobs))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::JobsLoaded(Err(Arc::new(e)))).await;
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
                        let _ = tx.send(Action::ClusterInfoLoaded(Err(Arc::new(e)))).await;
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
                        let _ = tx.send(Action::ClusterPeersLoaded(Err(Arc::new(e)))).await;
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
                        let _ = tx.send(Action::SavedSearchesLoaded(Err(Arc::new(e)))).await;
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
                        let _ = tx.send(Action::InternalLogsLoaded(Err(Arc::new(e)))).await;
                    }
                }
            });
        }
        Action::LoadApps { count, offset } => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_apps(Some(count), Some(offset)).await {
                    Ok(apps) => {
                        let _ = tx.send(Action::AppsLoaded(Ok(apps))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::AppsLoaded(Err(Arc::new(e)))).await;
                    }
                }
            });
        }
        Action::LoadUsers { count, offset } => {
            let _ = tx.send(Action::Loading(true)).await;
            tokio::spawn(async move {
                let mut c = client.lock().await;
                match c.list_users(Some(count), Some(offset)).await {
                    Ok(users) => {
                        let _ = tx.send(Action::UsersLoaded(Ok(users))).await;
                    }
                    Err(e) => {
                        let _ = tx.send(Action::UsersLoaded(Err(Arc::new(e)))).await;
                    }
                }
            });
        }
        // LoadMore actions for pagination - these require state access, handled in main loop
        Action::LoadMoreIndexes => {
            // This action is handled by the main loop which has access to state
            // It reads current pagination state and sends LoadIndexes with updated offset
        }
        Action::LoadMoreJobs => {
            // This action is handled by the main loop which has access to state
        }
        Action::LoadMoreApps => {
            // This action is handled by the main loop which has access to state
        }
        Action::LoadMoreUsers => {
            // This action is handled by the main loop which has access to state
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
                            .send(Action::MoreSearchResultsLoaded(Err(Arc::new(e))))
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
                        // Reload the job list (reset pagination)
                        let _ = tx
                            .send(Action::LoadJobs {
                                count: 100,
                                offset: 0,
                            })
                            .await;
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
                        // Reload the job list (reset pagination)
                        let _ = tx
                            .send(Action::LoadJobs {
                                count: 100,
                                offset: 0,
                            })
                            .await;
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

                // Process jobs sequentially to avoid overwhelming the API
                // and to provide clear per-job error reporting.
                // Parallelizing with join_all would require careful rate limiting
                // to avoid triggering Splunk's API throttling.
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
                let _ = tx_clone
                    .send(Action::LoadJobs {
                        count: 100,
                        offset: 0,
                    })
                    .await;
            });
        }
        Action::DeleteJobsBatch(sids) => {
            let _ = tx.send(Action::Loading(true)).await;
            let tx_clone = tx.clone();
            tokio::spawn(async move {
                let mut c = client.lock().await;
                let mut success_count = 0;
                let mut error_messages = Vec::new();

                // Process jobs sequentially to avoid overwhelming the API
                // and to provide clear per-job error reporting.
                // See CancelJobsBatch for parallelization considerations.
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
                let _ = tx_clone
                    .send(Action::LoadJobs {
                        count: 100,
                        offset: 0,
                    })
                    .await;
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
                        // Refresh apps list (reset pagination)
                        let _ = tx
                            .send(Action::LoadApps {
                                count: 100,
                                offset: 0,
                            })
                            .await;
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
                        // Refresh apps list (reset pagination)
                        let _ = tx
                            .send(Action::LoadApps {
                                count: 100,
                                offset: 0,
                            })
                            .await;
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

                let mut first_error: Option<ClientError> = None;

                // Collect health info sequentially due to the &mut self requirement
                // on client methods. Each call may need to refresh the session token,
                // requiring exclusive access to the client.
                //
                // Parallelization options:
                // 1. Spawn 5 separate tasks (each waits for the same mutex - minimal gain)
                // 2. Refactor client to support concurrent calls (significant effort)
                // 3. Use a connection pool (adds complexity for health checks only)
                //
                // Given that health checks run infrequently and network latency
                // dominates, sequential execution is the pragmatic choice.
                match c.get_server_info().await {
                    Ok(info) => health_output.server_info = Some(info),
                    Err(e) => {
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                    }
                }

                match c.get_health().await {
                    Ok(health) => health_output.splunkd_health = Some(health),
                    Err(e) => {
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                    }
                }

                match c.get_license_usage().await {
                    Ok(license) => health_output.license_usage = Some(license),
                    Err(e) => {
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                    }
                }

                match c.get_kvstore_status().await {
                    Ok(kvstore) => health_output.kvstore_status = Some(kvstore),
                    Err(e) => {
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                    }
                }

                match c.check_log_parsing_health().await {
                    Ok(log_parsing) => health_output.log_parsing_health = Some(log_parsing),
                    Err(e) => {
                        if first_error.is_none() {
                            first_error = Some(e);
                        }
                    }
                }

                if let Some(e) = first_error {
                    let _ = tx
                        .send(Action::HealthLoaded(Box::new(Err(Arc::new(e)))))
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
                let result = crate::export::export_value(&data, &path, format);

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
