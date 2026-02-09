//! Side effect dispatcher.
//!
//! This module contains the main `handle_side_effects` function that routes
//! actions to their appropriate handler functions in submodules.

use crate::action::Action;
use crate::runtime::side_effects::{
    SharedClient, TaskTracker, alerts, apps, audit, cluster, configs, dashboards, datamodels,
    export, forwarders, health, indexes, inputs, jobs, kvstore, license, logs, lookups, macros,
    multi_instance, overview, profiles, roles, search_peers, searches, shc, users, workload,
};
use splunk_config::ConfigManager;
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::{Mutex, mpsc::Sender};
use tracing::{Instrument, info_span};

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
    task_tracker: TaskTracker,
) {
    let action_name = action_type_name(&action);
    let start = Instant::now();

    let span = info_span!(
        "tui.handle_action",
        action_type = action_name,
        duration_ms = tracing::field::Empty,
    );

    async move {
        handle_action(action, client, tx, config_manager, task_tracker).await;

        // Record duration at the end
        let duration = start.elapsed().as_millis() as i64;
        tracing::Span::current().record("duration_ms", duration);
    }
    .instrument(span)
    .await;
}

/// Get a safe action name for tracing (no sensitive data).
fn action_type_name(action: &Action) -> &'static str {
    match action {
        Action::LoadIndexes { .. } => "LoadIndexes",
        Action::LoadJobs { .. } => "LoadJobs",
        Action::LoadClusterInfo => "LoadClusterInfo",
        Action::LoadClusterPeers => "LoadClusterPeers",
        Action::SetMaintenanceMode { .. } => "SetMaintenanceMode",
        Action::RebalanceCluster => "RebalanceCluster",
        Action::DecommissionPeer { .. } => "DecommissionPeer",
        Action::RemovePeer { .. } => "RemovePeer",
        Action::LoadSavedSearches => "LoadSavedSearches",
        Action::LoadMacros => "LoadMacros",
        Action::CreateMacro { .. } => "CreateMacro",
        Action::UpdateMacro { .. } => "UpdateMacro",
        Action::DeleteMacro { .. } => "DeleteMacro",
        Action::UpdateSavedSearch { .. } => "UpdateSavedSearch",
        Action::CreateSavedSearch { .. } => "CreateSavedSearch",
        Action::DeleteSavedSearch { .. } => "DeleteSavedSearch",
        Action::ToggleSavedSearch { .. } => "ToggleSavedSearch",
        Action::LoadInternalLogs { .. } => "LoadInternalLogs",
        Action::LoadApps { .. } => "LoadApps",
        Action::LoadUsers { .. } => "LoadUsers",
        Action::LoadMoreIndexes => "LoadMoreIndexes",
        Action::LoadMoreJobs => "LoadMoreJobs",
        Action::LoadMoreApps => "LoadMoreApps",
        Action::LoadMoreUsers => "LoadMoreUsers",
        Action::LoadMoreInternalLogs => "LoadMoreInternalLogs",
        Action::LoadSearchPeers { .. } => "LoadSearchPeers",
        Action::LoadMoreSearchPeers => "LoadMoreSearchPeers",
        Action::LoadForwarders { .. } => "LoadForwarders",
        Action::LoadMoreForwarders => "LoadMoreForwarders",
        Action::LoadLookups { .. } => "LoadLookups",
        Action::LoadMoreLookups => "LoadMoreLookups",
        Action::DownloadLookup { .. } => "DownloadLookup",
        Action::DeleteLookup { .. } => "DeleteLookup",
        Action::LoadInputs { .. } => "LoadInputs",
        Action::LoadMoreInputs => "LoadMoreInputs",
        Action::LoadConfigFiles => "LoadConfigFiles",
        Action::LoadFiredAlerts { .. } => "LoadFiredAlerts",
        Action::LoadMoreFiredAlerts => "LoadMoreFiredAlerts",
        Action::LoadConfigStanzas { .. } => "LoadConfigStanzas",
        Action::EnableInput { .. } => "EnableInput",
        Action::DisableInput { .. } => "DisableInput",
        Action::SwitchToSettings => "SwitchToSettings",
        Action::RunSearch { .. } => "RunSearch",
        Action::LoadMoreSearchResults { .. } => "LoadMoreSearchResults",
        Action::ValidateSpl { .. } => "ValidateSpl",
        Action::CancelJob(_) => "CancelJob",
        Action::DeleteJob(_) => "DeleteJob",
        Action::CancelJobsBatch(_) => "CancelJobsBatch",
        Action::DeleteJobsBatch(_) => "DeleteJobsBatch",
        Action::EnableApp(_) => "EnableApp",
        Action::DisableApp(_) => "DisableApp",
        Action::InstallApp { .. } => "InstallApp",
        Action::RemoveApp { .. } => "RemoveApp",
        Action::LoadHealth => "LoadHealth",
        Action::LoadLicense => "LoadLicense",
        Action::LoadKvstore => "LoadKvstore",
        Action::LoadOverview => "LoadOverview",
        Action::LoadMultiInstanceOverview => "LoadMultiInstanceOverview",
        Action::ExportData(_, _, _) => "ExportData",
        Action::OpenProfileSwitcher => "OpenProfileSwitcher",
        Action::ProfileSelected(_) => "ProfileSelected",
        Action::CreateIndex { .. } => "CreateIndex",
        Action::ModifyIndex { .. } => "ModifyIndex",
        Action::DeleteIndex { .. } => "DeleteIndex",
        Action::CreateUser { .. } => "CreateUser",
        Action::ModifyUser { .. } => "ModifyUser",
        Action::DeleteUser { .. } => "DeleteUser",
        Action::LoadRoles { .. } => "LoadRoles",
        Action::LoadCapabilities => "LoadCapabilities",
        Action::CreateRole { .. } => "CreateRole",
        Action::ModifyRole { .. } => "ModifyRole",
        Action::DeleteRole { .. } => "DeleteRole",
        Action::InstallLicense { .. } => "InstallLicense",
        Action::CreateLicensePool { .. } => "CreateLicensePool",
        Action::ModifyLicensePool { .. } => "ModifyLicensePool",
        Action::DeleteLicensePool { .. } => "DeleteLicensePool",
        Action::ActivateLicense { .. } => "ActivateLicense",
        Action::DeactivateLicense { .. } => "DeactivateLicense",
        Action::OpenEditProfileDialog { .. } => "OpenEditProfileDialog",
        Action::SaveProfile { .. } => "SaveProfile",
        Action::DeleteProfile { .. } => "DeleteProfile",
        Action::LoadAuditEvents { .. } => "LoadAuditEvents",
        Action::LoadRecentAuditEvents { .. } => "LoadRecentAuditEvents",
        Action::LoadDashboards { .. } => "LoadDashboards",
        Action::LoadMoreDashboards => "LoadMoreDashboards",
        Action::LoadDataModels { .. } => "LoadDataModels",
        Action::LoadMoreDataModels => "LoadMoreDataModels",
        Action::RefreshIndexes => "RefreshIndexes",
        Action::RefreshJobs => "RefreshJobs",
        Action::RefreshApps => "RefreshApps",
        Action::RefreshUsers => "RefreshUsers",
        Action::RefreshInternalLogs => "RefreshInternalLogs",
        Action::RefreshDashboards => "RefreshDashboards",
        Action::RefreshDataModels => "RefreshDataModels",
        Action::RefreshInputs => "RefreshInputs",
        Action::LoadWorkloadPools { .. } => "LoadWorkloadPools",
        Action::LoadMoreWorkloadPools => "LoadMoreWorkloadPools",
        Action::LoadWorkloadRules { .. } => "LoadWorkloadRules",
        Action::LoadMoreWorkloadRules => "LoadMoreWorkloadRules",
        Action::LoadShcStatus => "LoadShcStatus",
        Action::LoadShcMembers => "LoadShcMembers",
        Action::LoadShcCaptain => "LoadShcCaptain",
        Action::LoadShcConfig => "LoadShcConfig",
        Action::AddShcMember { .. } => "AddShcMember",
        Action::RemoveShcMember { .. } => "RemoveShcMember",
        Action::RollingRestartShc { .. } => "RollingRestartShc",
        Action::SetShcCaptain { .. } => "SetShcCaptain",
        Action::Input(_) => "Input",
        Action::Mouse(_) => "Mouse",
        Action::Resize(_, _) => "Resize",
        Action::Tick => "Tick",
        Action::Quit => "Quit",
        Action::Loading(_) => "Loading",
        Action::PersistState => "PersistState",
        _ => "Other",
    }
}

async fn handle_action(
    action: Action,
    client: SharedClient,
    tx: Sender<Action>,
    config_manager: Arc<Mutex<ConfigManager>>,
    task_tracker: TaskTracker,
) {
    match action {
        Action::LoadIndexes { count, offset } => {
            indexes::handle_load_indexes(client, tx, task_tracker.clone(), count, offset).await;
        }
        Action::LoadJobs { count, offset } => {
            jobs::handle_load_jobs(client, tx, task_tracker.clone(), count, offset).await;
        }
        Action::LoadClusterInfo => {
            cluster::handle_load_cluster_info(client, tx, task_tracker.clone()).await;
        }
        Action::LoadClusterPeers => {
            cluster::handle_load_cluster_peers(client, tx, task_tracker.clone()).await;
        }
        // Cluster management actions
        Action::SetMaintenanceMode { enable } => {
            cluster::handle_set_maintenance_mode(client, tx, task_tracker.clone(), enable).await;
        }
        Action::RebalanceCluster => {
            cluster::handle_rebalance_cluster(client, tx, task_tracker.clone()).await;
        }
        Action::DecommissionPeer { peer_guid } => {
            cluster::handle_decommission_peer(client, tx, task_tracker.clone(), peer_guid).await;
        }
        Action::RemovePeer { peer_guid } => {
            cluster::handle_remove_peer(client, tx, task_tracker.clone(), peer_guid).await;
        }
        Action::LoadSavedSearches => {
            searches::handle_load_saved_searches(client, tx, task_tracker.clone()).await;
        }
        Action::LoadMacros => {
            macros::handle_load_macros(client, tx, task_tracker.clone()).await;
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
            macros::handle_create_macro(client, tx, task_tracker.clone(), params).await;
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
            macros::handle_update_macro(client, tx, task_tracker.clone(), params).await;
        }
        Action::DeleteMacro { name } => {
            macros::handle_delete_macro(client, tx, task_tracker.clone(), name).await;
        }
        Action::UpdateSavedSearch {
            name,
            search,
            description,
            disabled,
        } => {
            searches::handle_update_saved_search(
                client,
                tx,
                task_tracker.clone(),
                name,
                search,
                description,
                disabled,
            )
            .await;
        }
        Action::CreateSavedSearch {
            name,
            search,
            description,
            disabled,
        } => {
            searches::handle_create_saved_search(
                client,
                tx,
                task_tracker.clone(),
                name,
                search,
                description,
                disabled,
            )
            .await;
        }
        Action::DeleteSavedSearch { name } => {
            searches::handle_delete_saved_search(client, tx, task_tracker.clone(), name).await;
        }
        Action::ToggleSavedSearch { name, disabled } => {
            searches::handle_toggle_saved_search(client, tx, task_tracker.clone(), name, disabled)
                .await;
        }
        Action::LoadInternalLogs { count, earliest } => {
            logs::handle_load_internal_logs(client, tx, task_tracker.clone(), count, earliest)
                .await;
        }
        Action::LoadApps { count, offset } => {
            apps::handle_load_apps(client, tx, task_tracker.clone(), count, offset).await;
        }
        Action::LoadUsers { count, offset } => {
            users::handle_load_users(client, tx, task_tracker.clone(), count, offset).await;
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
            search_peers::handle_load_search_peers(client, tx, task_tracker.clone(), count, offset)
                .await;
        }
        Action::LoadMoreSearchPeers => {
            // This action is handled by the main loop which has access to state
            // It reads search_peers_pagination and sends LoadSearchPeers with updated offset
        }
        Action::LoadForwarders { count, offset } => {
            forwarders::handle_load_forwarders(client, tx, task_tracker.clone(), count, offset)
                .await;
        }
        Action::LoadMoreForwarders => {
            // This action is handled by the main loop which has access to state
            // It reads forwarders_pagination and sends LoadForwarders with updated offset
        }
        Action::LoadLookups { count, offset } => {
            lookups::handle_load_lookups(client, tx, task_tracker.clone(), count, offset).await;
        }
        Action::LoadMoreLookups => {
            // This action is handled by the main loop which has access to state
            // It reads lookups_pagination and sends LoadLookups with updated offset
        }
        Action::DownloadLookup {
            name,
            app,
            owner,
            output_path,
        } => {
            lookups::handle_download_lookup(
                client,
                tx,
                task_tracker.clone(),
                name,
                app,
                owner,
                output_path,
            )
            .await;
        }
        Action::DeleteLookup { name, app, owner } => {
            lookups::handle_delete_lookup(client, tx, task_tracker.clone(), name, app, owner).await;
        }
        Action::LoadInputs { count, offset } => {
            inputs::handle_load_inputs(client, tx, task_tracker.clone(), count, offset).await;
        }
        Action::LoadMoreInputs => {
            // This action is handled by the main loop which has access to state
            // It reads inputs_pagination and sends LoadInputs with updated offset
        }
        Action::LoadConfigFiles => {
            configs::handle_load_config_files(client, tx, task_tracker.clone()).await;
        }
        Action::LoadFiredAlerts { count, offset } => {
            alerts::handle_load_fired_alerts(client, tx, task_tracker.clone(), count, offset).await;
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
            configs::handle_load_config_stanzas(
                client,
                tx,
                task_tracker.clone(),
                config_file,
                count,
                offset,
            )
            .await;
        }
        Action::EnableInput { input_type, name } => {
            inputs::handle_enable_input(client, tx, task_tracker.clone(), input_type, name).await;
        }
        Action::DisableInput { input_type, name } => {
            inputs::handle_disable_input(client, tx, task_tracker.clone(), input_type, name).await;
        }
        Action::SwitchToSettings => {
            profiles::handle_switch_to_settings(config_manager, tx, task_tracker.clone()).await;
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
                task_tracker.clone(),
                query,
                search_defaults,
                search_mode,
                realtime_window,
            )
            .await;
        }
        Action::LoadMoreSearchResults { sid, offset, count } => {
            searches::handle_load_more_search_results(
                client,
                tx,
                task_tracker.clone(),
                sid,
                offset,
                count,
            )
            .await;
        }
        Action::ValidateSpl { search, request_id } => {
            searches::handle_validate_spl(client, tx, task_tracker.clone(), search, request_id)
                .await;
        }
        Action::CancelJob(sid) => {
            jobs::handle_cancel_job(client, tx, task_tracker.clone(), sid).await;
        }
        Action::DeleteJob(sid) => {
            jobs::handle_delete_job(client, tx, task_tracker.clone(), sid).await;
        }
        Action::CancelJobsBatch(sids) => {
            jobs::handle_cancel_jobs_batch(client, tx, task_tracker.clone(), sids).await;
        }
        Action::DeleteJobsBatch(sids) => {
            jobs::handle_delete_jobs_batch(client, tx, task_tracker.clone(), sids).await;
        }
        Action::EnableApp(name) => {
            apps::handle_enable_app(client, tx, task_tracker.clone(), name).await;
        }
        Action::DisableApp(name) => {
            apps::handle_disable_app(client, tx, task_tracker.clone(), name).await;
        }
        Action::InstallApp { file_path } => {
            apps::handle_install_app(client, tx, task_tracker.clone(), file_path).await;
        }
        Action::RemoveApp { app_name } => {
            apps::handle_remove_app(client, tx, task_tracker.clone(), app_name).await;
        }
        Action::LoadHealth => {
            health::handle_load_health(client, tx, task_tracker.clone()).await;
        }
        Action::LoadLicense => {
            license::handle_load_license(client, tx, task_tracker.clone()).await;
        }
        Action::LoadKvstore => {
            kvstore::handle_load_kvstore(client, tx, task_tracker.clone()).await;
        }
        Action::LoadOverview => {
            overview::handle_load_overview(client, tx, task_tracker.clone()).await;
        }
        Action::LoadMultiInstanceOverview => {
            multi_instance::handle_load_multi_instance_overview(
                config_manager,
                tx,
                task_tracker.clone(),
            )
            .await;
        }
        Action::ExportData(data, path, format) => {
            export::handle_export_data(data, path, format, tx, task_tracker.clone()).await;
        }
        // Profile switching actions
        Action::OpenProfileSwitcher => {
            profiles::handle_open_profile_switcher(config_manager, tx, task_tracker.clone()).await;
        }
        Action::ProfileSelected(profile_name) => {
            profiles::handle_profile_selected(
                client,
                config_manager,
                tx,
                task_tracker.clone(),
                profile_name,
            )
            .await;
        }
        // Index operations
        Action::CreateIndex { params } => {
            indexes::handle_create_index(client, tx, task_tracker.clone(), params).await;
        }
        Action::ModifyIndex { name, params } => {
            indexes::handle_modify_index(client, tx, task_tracker.clone(), name, params).await;
        }
        Action::DeleteIndex { name } => {
            indexes::handle_delete_index(client, tx, task_tracker.clone(), name).await;
        }
        // User operations
        Action::CreateUser { params } => {
            users::handle_create_user(client, tx, task_tracker.clone(), params).await;
        }
        Action::ModifyUser { name, params } => {
            users::handle_modify_user(client, tx, task_tracker.clone(), name, params).await;
        }
        Action::DeleteUser { name } => {
            users::handle_delete_user(client, tx, task_tracker.clone(), name).await;
        }
        // Role operations
        Action::LoadRoles { count, offset } => {
            roles::handle_load_roles(client, tx, task_tracker.clone(), count, offset).await;
        }
        Action::LoadCapabilities => {
            roles::handle_load_capabilities(client, tx, task_tracker.clone()).await;
        }
        Action::CreateRole { params } => {
            roles::handle_create_role(client, tx, task_tracker.clone(), params).await;
        }
        Action::ModifyRole { name, params } => {
            roles::handle_modify_role(client, tx, task_tracker.clone(), name, params).await;
        }
        Action::DeleteRole { name } => {
            roles::handle_delete_role(client, tx, task_tracker.clone(), name).await;
        }
        // License operations
        Action::InstallLicense { file_path } => {
            license::handle_install_license(client, file_path, tx, task_tracker.clone()).await;
        }
        Action::CreateLicensePool { params } => {
            license::handle_create_license_pool(client, params, tx, task_tracker.clone()).await;
        }
        Action::ModifyLicensePool { name, params } => {
            license::handle_modify_license_pool(client, name, params, tx, task_tracker.clone())
                .await;
        }
        Action::DeleteLicensePool { name } => {
            license::handle_delete_license_pool(client, name, tx, task_tracker.clone()).await;
        }
        Action::ActivateLicense { name } => {
            license::handle_activate_license(client, name, tx, task_tracker.clone()).await;
        }
        Action::DeactivateLicense { name } => {
            license::handle_deactivate_license(client, name, tx, task_tracker.clone()).await;
        }
        // Profile management actions
        Action::OpenEditProfileDialog { name } => {
            profiles::handle_open_edit_profile(
                config_manager.clone(),
                tx.clone(),
                task_tracker.clone(),
                name,
            )
            .await;
        }
        Action::SaveProfile {
            name,
            profile,
            use_keyring,
            original_name,
            from_tutorial,
        } => {
            profiles::handle_save_profile(
                config_manager.clone(),
                tx.clone(),
                task_tracker.clone(),
                name,
                profile,
                use_keyring,
                original_name,
                from_tutorial,
            )
            .await;
        }
        Action::DeleteProfile { name } => {
            profiles::handle_delete_profile(
                config_manager.clone(),
                tx.clone(),
                task_tracker.clone(),
                name,
            )
            .await;
        }
        Action::LoadAuditEvents {
            count,
            offset,
            earliest,
            latest,
        } => {
            audit::handle_load_audit_events(
                client,
                tx,
                task_tracker.clone(),
                count,
                offset,
                earliest,
                latest,
            )
            .await;
        }
        Action::LoadRecentAuditEvents { count } => {
            audit::handle_load_recent_audit_events(client, tx, task_tracker.clone(), count).await;
        }
        Action::LoadDashboards { count, offset } => {
            dashboards::handle_load_dashboards(client, tx, task_tracker.clone(), count, offset)
                .await;
        }
        Action::LoadMoreDashboards => {
            // This action is handled by the main loop which has access to state
            // It reads dashboards_pagination and sends LoadDashboards with updated offset
        }
        Action::LoadDataModels { count, offset } => {
            datamodels::handle_load_datamodels(client, tx, task_tracker.clone(), count, offset)
                .await;
        }
        Action::LoadMoreDataModels => {
            // This action is handled by the main loop which has access to state
            // It reads data_models_pagination and sends LoadDataModels with updated offset
        }
        // Refresh actions - these are translated to Load* actions with offset=0 by the main loop
        Action::RefreshIndexes
        | Action::RefreshJobs
        | Action::RefreshApps
        | Action::RefreshUsers
        | Action::RefreshInternalLogs
        | Action::RefreshDashboards
        | Action::RefreshDataModels
        | Action::RefreshInputs => {
            // These are handled by the main loop which translates them to Load* actions
        }
        Action::LoadWorkloadPools { count, offset } => {
            workload::handle_load_workload_pools(client, tx, task_tracker.clone(), count, offset)
                .await;
        }
        Action::LoadMoreWorkloadPools => {
            // This action is handled by the main loop which has access to state
            // It reads workload_pools_pagination and sends LoadWorkloadPools with updated offset
        }
        Action::LoadWorkloadRules { count, offset } => {
            workload::handle_load_workload_rules(client, tx, task_tracker.clone(), count, offset)
                .await;
        }
        Action::LoadMoreWorkloadRules => {
            // This action is handled by the main loop which has access to state
            // It reads workload_rules_pagination and sends LoadWorkloadRules with updated offset
        }
        // SHC actions
        Action::LoadShcStatus => {
            shc::handle_load_shc_status(client, tx, task_tracker.clone()).await;
        }
        Action::LoadShcMembers => {
            shc::handle_load_shc_members(client, tx, task_tracker.clone()).await;
        }
        Action::LoadShcCaptain => {
            shc::handle_load_shc_captain(client, tx, task_tracker.clone()).await;
        }
        Action::LoadShcConfig => {
            shc::handle_load_shc_config(client, tx, task_tracker.clone()).await;
        }
        Action::AddShcMember { target_uri } => {
            shc::handle_add_shc_member(client, tx, task_tracker.clone(), target_uri).await;
        }
        Action::RemoveShcMember { member_guid } => {
            shc::handle_remove_shc_member(client, tx, task_tracker.clone(), member_guid).await;
        }
        Action::RollingRestartShc { force } => {
            shc::handle_rolling_restart_shc(client, tx, task_tracker.clone(), force).await;
        }
        Action::SetShcCaptain { member_guid } => {
            shc::handle_set_shc_captain(client, tx, task_tracker.clone(), member_guid).await;
        }
        _ => {}
    }
}
