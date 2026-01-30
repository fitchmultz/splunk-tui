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

mod alerts;
mod apps;
mod cluster;
mod configs;
mod export;
mod health;
mod indexes;
mod inputs;
mod jobs;
mod kvstore;
mod license;
mod logs;
mod overview;
mod profiles;
mod search_peers;
mod searches;
mod users;

use crate::action::Action;
use splunk_client::SplunkClient;
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
            indexes::handle_load_indexes(client, tx, count, offset).await;
        }
        Action::LoadJobs { count, offset } => {
            jobs::handle_load_jobs(client, tx, count, offset).await;
        }
        Action::LoadClusterInfo => {
            cluster::handle_load_cluster_info(client, tx).await;
        }
        Action::LoadClusterPeers => {
            cluster::handle_load_cluster_peers(client, tx).await;
        }
        Action::LoadSavedSearches => {
            searches::handle_load_saved_searches(client, tx).await;
        }
        Action::LoadInternalLogs { count, earliest } => {
            logs::handle_load_internal_logs(client, tx, count, earliest).await;
        }
        Action::LoadApps { count, offset } => {
            apps::handle_load_apps(client, tx, count, offset).await;
        }
        Action::LoadUsers { count, offset } => {
            users::handle_load_users(client, tx, count, offset).await;
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
        Action::LoadMoreInternalLogs => {
            // This action is handled by the main loop which has access to state
            // It reads internal_logs_defaults and sends LoadInternalLogs with parameters
        }
        Action::LoadSearchPeers { count, offset } => {
            search_peers::handle_load_search_peers(client, tx, count, offset).await;
        }
        Action::LoadMoreSearchPeers => {
            // This action is handled by the main loop which has access to state
            // It reads search_peers_pagination and sends LoadSearchPeers with updated offset
        }
        Action::LoadInputs { count, offset } => {
            inputs::handle_load_inputs(client, tx, count, offset).await;
        }
        Action::LoadMoreInputs => {
            // This action is handled by the main loop which has access to state
            // It reads inputs_pagination and sends LoadInputs with updated offset
        }
        Action::LoadConfigFiles => {
            configs::handle_load_config_files(client, tx).await;
        }
        Action::LoadFiredAlerts => {
            alerts::handle_load_fired_alerts(client, tx).await;
        }
        Action::LoadMoreFiredAlerts => {
            // This action is handled by the main loop which has access to state
            // It reads fired_alerts_pagination and sends LoadFiredAlerts with updated offset
        }
        Action::LoadConfigStanzas {
            config_file,
            count,
            offset,
        } => {
            configs::handle_load_config_stanzas(client, tx, config_file, count, offset).await;
        }
        Action::EnableInput { input_type, name } => {
            inputs::handle_enable_input(client, tx, input_type, name).await;
        }
        Action::DisableInput { input_type, name } => {
            inputs::handle_disable_input(client, tx, input_type, name).await;
        }
        Action::SwitchToSettings => {
            profiles::handle_switch_to_settings(config_manager, tx).await;
        }
        Action::RunSearch {
            query,
            search_defaults,
        } => {
            searches::handle_run_search(client, tx, query, search_defaults).await;
        }
        Action::LoadMoreSearchResults { sid, offset, count } => {
            searches::handle_load_more_search_results(client, tx, sid, offset, count).await;
        }
        Action::ValidateSpl { search } => {
            searches::handle_validate_spl(client, tx, search).await;
        }
        Action::CancelJob(sid) => {
            jobs::handle_cancel_job(client, tx, sid).await;
        }
        Action::DeleteJob(sid) => {
            jobs::handle_delete_job(client, tx, sid).await;
        }
        Action::CancelJobsBatch(sids) => {
            jobs::handle_cancel_jobs_batch(client, tx, sids).await;
        }
        Action::DeleteJobsBatch(sids) => {
            jobs::handle_delete_jobs_batch(client, tx, sids).await;
        }
        Action::EnableApp(name) => {
            apps::handle_enable_app(client, tx, name).await;
        }
        Action::DisableApp(name) => {
            apps::handle_disable_app(client, tx, name).await;
        }
        Action::LoadHealth => {
            health::handle_load_health(client, tx).await;
        }
        Action::LoadLicense => {
            license::handle_load_license(client, tx).await;
        }
        Action::LoadKvstore => {
            kvstore::handle_load_kvstore(client, tx).await;
        }
        Action::LoadOverview => {
            overview::handle_load_overview(client, tx).await;
        }
        Action::ExportData(data, path, format) => {
            export::handle_export_data(data, path, format, tx).await;
        }
        // Profile switching actions
        Action::OpenProfileSwitcher => {
            profiles::handle_open_profile_switcher(config_manager, tx).await;
        }
        Action::ProfileSelected(profile_name) => {
            profiles::handle_profile_selected(client, config_manager, tx, profile_name).await;
        }
        _ => {}
    }
}
