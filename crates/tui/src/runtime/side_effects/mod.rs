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
//! Invariants:
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
mod audit;
mod cluster;
mod configs;
mod dashboards;
mod datamodels;
mod export;
mod forwarders;
mod health;
mod indexes;
mod inputs;
mod jobs;
mod kvstore;
mod license;
mod logs;
mod lookups;
mod macros;
mod multi_instance;
mod overview;
mod overview_fetch;
mod profiles;
mod roles;
mod search_peers;
mod searches;
mod shc;
mod users;
mod workload;

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
        // Cluster management actions
        Action::SetMaintenanceMode { enable } => {
            cluster::handle_set_maintenance_mode(client, tx, enable).await;
        }
        Action::RebalanceCluster => {
            cluster::handle_rebalance_cluster(client, tx).await;
        }
        Action::DecommissionPeer { peer_guid } => {
            cluster::handle_decommission_peer(client, tx, peer_guid).await;
        }
        Action::RemovePeer { peer_guid } => {
            cluster::handle_remove_peer(client, tx, peer_guid).await;
        }
        Action::LoadSavedSearches => {
            searches::handle_load_saved_searches(client, tx).await;
        }
        Action::LoadMacros => {
            macros::handle_load_macros(client, tx).await;
        }
        Action::CreateMacro {
            name,
            definition,
            args,
            description,
            disabled,
            iseval,
        } => {
            let params = macros::CreateMacroEffectParams {
                name,
                definition,
                args,
                description,
                disabled,
                iseval,
            };
            macros::handle_create_macro(client, tx, params).await;
        }
        Action::UpdateMacro {
            name,
            definition,
            args,
            description,
            disabled,
            iseval,
        } => {
            let params = macros::UpdateMacroEffectParams {
                name,
                definition,
                args,
                description,
                disabled,
                iseval,
            };
            macros::handle_update_macro(client, tx, params).await;
        }
        Action::DeleteMacro { name } => {
            macros::handle_delete_macro(client, tx, name).await;
        }
        Action::UpdateSavedSearch {
            name,
            search,
            description,
            disabled,
        } => {
            searches::handle_update_saved_search(client, tx, name, search, description, disabled)
                .await;
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
        Action::LoadForwarders { count, offset } => {
            forwarders::handle_load_forwarders(client, tx, count, offset).await;
        }
        Action::LoadMoreForwarders => {
            // This action is handled by the main loop which has access to state
            // It reads forwarders_pagination and sends LoadForwarders with updated offset
        }
        Action::LoadLookups { count, offset } => {
            lookups::handle_load_lookups(client, tx, count, offset).await;
        }
        Action::LoadMoreLookups => {
            // This action is handled by the main loop which has access to state
            // It reads lookups_pagination and sends LoadLookups with updated offset
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
        Action::LoadFiredAlerts { count, offset } => {
            alerts::handle_load_fired_alerts(client, tx, count, offset).await;
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
            search_mode,
            realtime_window,
        } => {
            searches::handle_run_search(
                client,
                tx,
                query,
                search_defaults,
                search_mode,
                realtime_window,
            )
            .await;
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
        Action::InstallApp { file_path } => {
            apps::handle_install_app(client, tx, file_path).await;
        }
        Action::RemoveApp { app_name } => {
            apps::handle_remove_app(client, tx, app_name).await;
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
        Action::LoadMultiInstanceOverview => {
            multi_instance::handle_load_multi_instance_overview(config_manager, tx).await;
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
        // Index operations
        Action::CreateIndex { params } => {
            indexes::handle_create_index(client, tx, params).await;
        }
        Action::ModifyIndex { name, params } => {
            indexes::handle_modify_index(client, tx, name, params).await;
        }
        Action::DeleteIndex { name } => {
            indexes::handle_delete_index(client, tx, name).await;
        }
        // User operations
        Action::CreateUser { params } => {
            users::handle_create_user(client, tx, params).await;
        }
        Action::ModifyUser { name, params } => {
            users::handle_modify_user(client, tx, name, params).await;
        }
        Action::DeleteUser { name } => {
            users::handle_delete_user(client, tx, name).await;
        }
        // Role operations
        Action::LoadRoles { count, offset } => {
            roles::handle_load_roles(client, tx, count, offset).await;
        }
        Action::LoadCapabilities => {
            roles::handle_load_capabilities(client, tx).await;
        }
        Action::CreateRole { params } => {
            roles::handle_create_role(client, tx, params).await;
        }
        Action::ModifyRole { name, params } => {
            roles::handle_modify_role(client, tx, name, params).await;
        }
        Action::DeleteRole { name } => {
            roles::handle_delete_role(client, tx, name).await;
        }
        // License operations
        Action::InstallLicense { file_path } => {
            license::handle_install_license(client, file_path, tx).await;
        }
        Action::CreateLicensePool { params } => {
            license::handle_create_license_pool(client, params, tx).await;
        }
        Action::ModifyLicensePool { name, params } => {
            license::handle_modify_license_pool(client, name, params, tx).await;
        }
        Action::DeleteLicensePool { name } => {
            license::handle_delete_license_pool(client, name, tx).await;
        }
        Action::ActivateLicense { name } => {
            license::handle_activate_license(client, name, tx).await;
        }
        Action::DeactivateLicense { name } => {
            license::handle_deactivate_license(client, name, tx).await;
        }
        // Profile management actions
        Action::OpenEditProfileDialog { name } => {
            profiles::handle_open_edit_profile(config_manager.clone(), tx.clone(), name).await;
        }
        Action::SaveProfile {
            name,
            profile,
            use_keyring,
            original_name,
        } => {
            profiles::handle_save_profile(
                config_manager.clone(),
                tx.clone(),
                name,
                profile,
                use_keyring,
                original_name,
            )
            .await;
        }
        Action::DeleteProfile { name } => {
            profiles::handle_delete_profile(config_manager.clone(), tx.clone(), name).await;
        }
        Action::LoadAuditEvents {
            count,
            offset,
            earliest,
            latest,
        } => {
            audit::handle_load_audit_events(client, tx, count, offset, earliest, latest).await;
        }
        Action::LoadRecentAuditEvents { count } => {
            audit::handle_load_recent_audit_events(client, tx, count).await;
        }
        Action::LoadDashboards { count, offset } => {
            dashboards::handle_load_dashboards(client, tx, count, offset).await;
        }
        Action::LoadMoreDashboards => {
            // This action is handled by the main loop which has access to state
            // It reads dashboards_pagination and sends LoadDashboards with updated offset
        }
        Action::LoadDataModels { count, offset } => {
            datamodels::handle_load_datamodels(client, tx, count, offset).await;
        }
        Action::LoadMoreDataModels => {
            // This action is handled by the main loop which has access to state
            // It reads data_models_pagination and sends LoadDataModels with updated offset
        }
        Action::LoadWorkloadPools { count, offset } => {
            workload::handle_load_workload_pools(client, tx, count, offset).await;
        }
        Action::LoadMoreWorkloadPools => {
            // This action is handled by the main loop which has access to state
            // It reads workload_pools_pagination and sends LoadWorkloadPools with updated offset
        }
        Action::LoadWorkloadRules { count, offset } => {
            workload::handle_load_workload_rules(client, tx, count, offset).await;
        }
        Action::LoadMoreWorkloadRules => {
            // This action is handled by the main loop which has access to state
            // It reads workload_rules_pagination and sends LoadWorkloadRules with updated offset
        }
        // SHC actions
        Action::LoadShcStatus => {
            shc::handle_load_shc_status(client, tx).await;
        }
        Action::LoadShcMembers => {
            shc::handle_load_shc_members(client, tx).await;
        }
        Action::LoadShcCaptain => {
            shc::handle_load_shc_captain(client, tx).await;
        }
        Action::LoadShcConfig => {
            shc::handle_load_shc_config(client, tx).await;
        }
        Action::AddShcMember { target_uri } => {
            shc::handle_add_shc_member(client, tx, target_uri).await;
        }
        Action::RemoveShcMember { member_guid } => {
            shc::handle_remove_shc_member(client, tx, member_guid).await;
        }
        Action::RollingRestartShc { force } => {
            shc::handle_rolling_restart_shc(client, tx, force).await;
        }
        Action::SetShcCaptain { member_guid } => {
            shc::handle_set_shc_captain(client, tx, member_guid).await;
        }
        _ => {}
    }
}
