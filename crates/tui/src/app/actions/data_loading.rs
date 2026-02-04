//! Data loading action handlers for the TUI app.
//!
//! Responsibilities:
//! - Handle *Loaded and More*Loaded actions for all data types
//! - Update pagination state when data is loaded
//! - Handle error cases for data loading failures
//! - Rebuild filtered indices and restore selection when needed

use crate::action::{Action, LicenseData};
use crate::app::App;
use crate::app::state::HealthState;
use crate::ui::Toast;
use splunk_client::models::DataModel;

impl App {
    /// Handle data loading result actions.
    pub fn handle_data_loading_action(&mut self, action: Action) {
        match action {
            // Indexes
            Action::IndexesLoaded(Ok(indexes)) => {
                self.handle_indexes_loaded(indexes);
            }
            Action::IndexesLoaded(Err(e)) => {
                self.handle_data_load_error("indexes", e);
            }
            Action::MoreIndexesLoaded(Ok(indexes)) => {
                self.handle_more_indexes_loaded(indexes);
            }
            Action::MoreIndexesLoaded(Err(e)) => {
                self.handle_data_load_error("more indexes", e);
            }

            // Jobs
            Action::JobsLoaded(Ok(jobs)) => {
                self.handle_jobs_loaded(jobs);
            }
            Action::JobsLoaded(Err(e)) => {
                self.handle_data_load_error("jobs", e);
            }
            Action::MoreJobsLoaded(Ok(jobs)) => {
                self.handle_more_jobs_loaded(jobs);
            }
            Action::MoreJobsLoaded(Err(e)) => {
                self.handle_data_load_error("more jobs", e);
            }

            // Saved Searches
            Action::SavedSearchesLoaded(Ok(searches)) => {
                self.saved_searches = Some(searches);
                self.loading = false;
            }
            Action::SavedSearchesLoaded(Err(e)) => {
                self.handle_data_load_error("saved searches", e);
            }

            // Macros
            Action::MacrosLoaded(Ok(macros)) => {
                self.macros = Some(macros);
                self.loading = false;
            }
            Action::MacrosLoaded(Err(e)) => {
                self.handle_data_load_error("macros", e);
            }
            Action::MacroCreated(Ok(())) => {
                self.toasts
                    .push(Toast::success("Macro created successfully"));
                self.loading = false;
            }
            Action::MacroCreated(Err(e)) => {
                self.handle_data_load_error("create macro", e);
            }
            Action::MacroUpdated(Ok(())) => {
                self.toasts
                    .push(Toast::success("Macro updated successfully"));
                self.loading = false;
            }
            Action::MacroUpdated(Err(e)) => {
                self.handle_data_load_error("update macro", e);
            }
            Action::MacroDeleted(Ok(name)) => {
                self.toasts
                    .push(Toast::success(format!("Macro '{}' deleted", name)));
                self.loading = false;
                // Remove from list
                if let Some(ref mut macros) = self.macros {
                    macros.retain(|m| m.name != name);
                }
            }
            Action::MacroDeleted(Err(e)) => {
                self.handle_data_load_error("delete macro", e);
            }

            // Internal Logs
            Action::InternalLogsLoaded(Ok(logs)) => {
                self.handle_internal_logs_loaded(logs);
            }
            Action::InternalLogsLoaded(Err(e)) => {
                self.handle_data_load_error("internal logs", e);
            }

            // Cluster
            Action::ClusterInfoLoaded(Ok(info)) => {
                self.cluster_info = Some(info);
                self.loading = false;
            }
            Action::ClusterInfoLoaded(Err(e)) => {
                self.handle_data_load_error("cluster info", e);
            }
            Action::ClusterPeersLoaded(Ok(peers)) => {
                self.cluster_peers = Some(peers);
                self.loading = false;
            }
            Action::ClusterPeersLoaded(Err(e)) => {
                self.handle_data_load_error("cluster peers", e);
            }

            // Health
            Action::HealthLoaded(boxed_result) => {
                self.handle_health_loaded(boxed_result);
            }
            Action::HealthStatusLoaded(result) => {
                self.handle_health_status_loaded(result);
            }

            // License
            Action::LicenseLoaded(boxed_result) => {
                self.handle_license_loaded(*boxed_result);
            }

            // KVStore
            Action::KvstoreLoaded(Ok(status)) => {
                self.kvstore_status = Some(status);
                self.loading = false;
            }
            Action::KvstoreLoaded(Err(e)) => {
                self.handle_data_load_error("KVStore status", e);
            }

            // Apps
            Action::AppsLoaded(Ok(apps)) => {
                self.handle_apps_loaded(apps);
            }
            Action::AppsLoaded(Err(e)) => {
                self.handle_data_load_error("apps", e);
            }
            Action::MoreAppsLoaded(Ok(apps)) => {
                self.handle_more_apps_loaded(apps);
            }
            Action::MoreAppsLoaded(Err(e)) => {
                self.handle_data_load_error("more apps", e);
            }

            // Users
            Action::UsersLoaded(Ok(users)) => {
                self.handle_users_loaded(users);
            }
            Action::UsersLoaded(Err(e)) => {
                self.handle_data_load_error("users", e);
            }
            Action::MoreUsersLoaded(Ok(users)) => {
                self.handle_more_users_loaded(users);
            }
            Action::MoreUsersLoaded(Err(e)) => {
                self.handle_data_load_error("more users", e);
            }

            // Roles
            Action::RolesLoaded(Ok(roles)) => {
                self.roles = Some(roles);
                self.loading = false;
            }
            Action::RolesLoaded(Err(e)) => {
                self.handle_data_load_error("roles", e);
            }
            Action::RoleCreated(Ok(role)) => {
                self.toasts
                    .push(Toast::success(format!("Role '{}' created", role.name)));
                self.loading = false;
            }
            Action::RoleCreated(Err(e)) => {
                self.handle_data_load_error("create role", e);
            }
            Action::RoleModified(Ok(role)) => {
                self.toasts
                    .push(Toast::success(format!("Role '{}' modified", role.name)));
                self.loading = false;
            }
            Action::RoleModified(Err(e)) => {
                self.handle_data_load_error("modify role", e);
            }
            Action::RoleDeleted(Ok(name)) => {
                self.toasts
                    .push(Toast::success(format!("Role '{}' deleted", name)));
                self.loading = false;
            }
            Action::RoleDeleted(Err(e)) => {
                self.handle_data_load_error("delete role", e);
            }
            Action::CapabilitiesLoaded(Ok(capabilities)) => {
                self.capabilities = Some(capabilities);
                self.loading = false;
            }
            Action::CapabilitiesLoaded(Err(e)) => {
                self.handle_data_load_error("capabilities", e);
            }

            // Search Peers
            Action::SearchPeersLoaded(Ok(peers)) => {
                self.handle_search_peers_loaded(peers);
            }
            Action::SearchPeersLoaded(Err(e)) => {
                self.handle_data_load_error("search peers", e);
            }
            Action::MoreSearchPeersLoaded(Ok(peers)) => {
                self.handle_more_search_peers_loaded(peers);
            }
            Action::MoreSearchPeersLoaded(Err(e)) => {
                self.handle_data_load_error("more search peers", e);
            }

            // Forwarders
            Action::ForwardersLoaded(Ok(forwarders)) => {
                self.handle_forwarders_loaded(forwarders);
            }
            Action::ForwardersLoaded(Err(e)) => {
                self.handle_data_load_error("forwarders", e);
            }
            Action::MoreForwardersLoaded(Ok(forwarders)) => {
                self.handle_more_forwarders_loaded(forwarders);
            }
            Action::MoreForwardersLoaded(Err(e)) => {
                self.handle_data_load_error("more forwarders", e);
            }

            // Lookups
            Action::LookupsLoaded(Ok(lookups)) => {
                self.handle_lookups_loaded(lookups);
            }
            Action::LookupsLoaded(Err(e)) => {
                self.handle_data_load_error("lookups", e);
            }
            Action::MoreLookupsLoaded(Ok(lookups)) => {
                self.handle_more_lookups_loaded(lookups);
            }
            Action::MoreLookupsLoaded(Err(e)) => {
                self.handle_data_load_error("more lookups", e);
            }

            // Inputs
            Action::InputsLoaded(Ok(inputs)) => {
                self.handle_inputs_loaded(inputs);
            }
            Action::InputsLoaded(Err(e)) => {
                self.handle_data_load_error("inputs", e);
            }
            Action::MoreInputsLoaded(Ok(inputs)) => {
                self.handle_more_inputs_loaded(inputs);
            }
            Action::MoreInputsLoaded(Err(e)) => {
                self.handle_data_load_error("more inputs", e);
            }

            // Fired Alerts
            Action::FiredAlertsLoaded(Ok(alerts)) => {
                self.handle_fired_alerts_loaded(alerts);
            }
            Action::FiredAlertsLoaded(Err(e)) => {
                self.handle_data_load_error("fired alerts", e);
            }
            Action::MoreFiredAlertsLoaded(Ok(alerts)) => {
                self.handle_more_fired_alerts_loaded(alerts);
            }
            Action::MoreFiredAlertsLoaded(Err(e)) => {
                self.handle_data_load_error("more fired alerts", e);
            }

            // Audit Events
            Action::AuditEventsLoaded(Ok(events)) => {
                let sel = self.audit_state.selected();
                self.audit_events = Some(events);
                self.loading = false;
                if let Some(events) = &self.audit_events {
                    self.audit_state.select(
                        sel.map(|i| i.min(events.len().saturating_sub(1)))
                            .or(Some(0)),
                    );
                }
            }
            Action::AuditEventsLoaded(Err(e)) => {
                self.handle_data_load_error("audit events", e);
            }

            // Dashboards
            Action::DashboardsLoaded(Ok(dashboards)) => {
                self.handle_dashboards_loaded(dashboards);
            }
            Action::DashboardsLoaded(Err(e)) => {
                self.handle_data_load_error("dashboards", e);
            }
            Action::MoreDashboardsLoaded(Ok(dashboards)) => {
                self.handle_more_dashboards_loaded(dashboards);
            }
            Action::MoreDashboardsLoaded(Err(e)) => {
                self.handle_data_load_error("more dashboards", e);
            }

            // Data Models
            Action::DataModelsLoaded(Ok(datamodels)) => {
                self.handle_datamodels_loaded(datamodels);
            }
            Action::DataModelsLoaded(Err(e)) => {
                self.handle_data_load_error("data models", e);
            }
            Action::MoreDataModelsLoaded(Ok(datamodels)) => {
                self.handle_more_datamodels_loaded(datamodels);
            }
            Action::MoreDataModelsLoaded(Err(e)) => {
                self.handle_data_load_error("more data models", e);
            }

            // Workload Management
            Action::WorkloadPoolsLoaded(Ok(pools)) => {
                self.handle_workload_pools_loaded(pools);
            }
            Action::WorkloadPoolsLoaded(Err(e)) => {
                self.handle_data_load_error("workload pools", e);
            }
            Action::MoreWorkloadPoolsLoaded(Ok(pools)) => {
                self.handle_more_workload_pools_loaded(pools);
            }
            Action::MoreWorkloadPoolsLoaded(Err(e)) => {
                self.handle_data_load_error("more workload pools", e);
            }
            Action::WorkloadRulesLoaded(Ok(rules)) => {
                self.handle_workload_rules_loaded(rules);
            }
            Action::WorkloadRulesLoaded(Err(e)) => {
                self.handle_data_load_error("workload rules", e);
            }
            Action::MoreWorkloadRulesLoaded(Ok(rules)) => {
                self.handle_more_workload_rules_loaded(rules);
            }
            Action::MoreWorkloadRulesLoaded(Err(e)) => {
                self.handle_data_load_error("more workload rules", e);
            }

            // Config Files
            Action::ConfigFilesLoaded(Ok(files)) => {
                self.config_files = Some(files);
                self.loading = false;
            }
            Action::ConfigFilesLoaded(Err(e)) => {
                self.handle_data_load_error("config files", e);
            }

            // Config Stanzas
            Action::ConfigStanzasLoaded(Ok(stanzas)) => {
                self.config_stanzas = Some(stanzas);
                self.loading = false;
                // Rebuild filtered indices since data changed
                self.rebuild_filtered_stanza_indices();
            }
            Action::ConfigStanzasLoaded(Err(e)) => {
                self.handle_data_load_error("config stanzas", e);
            }

            // Settings and Overview
            Action::SettingsLoaded(state) => {
                self.apply_loaded_settings(state);
            }
            Action::OverviewLoaded(data) => {
                self.overview_data = Some(data);
                self.loading = false;
            }
            Action::MultiInstanceOverviewLoaded(data) => {
                self.multi_instance_data = Some(data);
                self.loading = false;
            }

            _ => {}
        }
    }

    // Indexes handlers
    fn handle_indexes_loaded(&mut self, indexes: Vec<splunk_client::models::Index>) {
        let count = indexes.len();
        self.indexes = Some(indexes);
        self.indexes_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_more_indexes_loaded(&mut self, indexes: Vec<splunk_client::models::Index>) {
        let count = indexes.len();
        if let Some(ref mut existing) = self.indexes {
            existing.extend(indexes);
        } else {
            self.indexes = Some(indexes);
        }
        self.indexes_pagination.update_loaded(count);
        self.loading = false;
    }

    // Jobs handlers
    fn handle_jobs_loaded(&mut self, jobs: Vec<splunk_client::SearchJobStatus>) {
        let sel = self.jobs_state.selected();
        let count = jobs.len();
        self.jobs = Some(jobs);
        self.jobs_pagination.update_loaded(count);
        self.loading = false;
        // Rebuild filtered indices and restore selection clamped to new bounds
        self.rebuild_filtered_indices();
        let filtered_len = self.filtered_jobs_len();
        self.jobs_state.select(
            sel.map(|i| i.min(filtered_len.saturating_sub(1)))
                .or(Some(0)),
        );
    }

    fn handle_more_jobs_loaded(&mut self, jobs: Vec<splunk_client::SearchJobStatus>) {
        let sel = self.jobs_state.selected();
        let count = jobs.len();
        if let Some(ref mut existing) = self.jobs {
            existing.extend(jobs);
        } else {
            self.jobs = Some(jobs);
        }
        self.jobs_pagination.update_loaded(count);
        self.loading = false;
        // Rebuild filtered indices to include new items
        self.rebuild_filtered_indices();
        let filtered_len = self.filtered_jobs_len();
        self.jobs_state.select(
            sel.map(|i| i.min(filtered_len.saturating_sub(1)))
                .or(Some(0)),
        );
    }

    // Internal logs handler
    fn handle_internal_logs_loaded(&mut self, logs: Vec<splunk_client::models::LogEntry>) {
        let sel = self.internal_logs_state.selected();
        self.internal_logs = Some(logs);
        self.loading = false;
        if let Some(logs) = &self.internal_logs {
            self.internal_logs_state
                .select(sel.map(|i| i.min(logs.len().saturating_sub(1))).or(Some(0)));
        }
    }

    // Health handlers
    fn handle_health_loaded(
        &mut self,
        boxed_result: Box<
            Result<
                splunk_client::models::HealthCheckOutput,
                std::sync::Arc<splunk_client::ClientError>,
            >,
        >,
    ) {
        match *boxed_result {
            Ok(ref info) => {
                self.health_info = Some(info.clone());
                // Update health state from splunkd_health if available
                if let Some(ref health) = info.splunkd_health {
                    let new_state = HealthState::from_health_str(&health.health);
                    self.set_health_state(new_state);
                }
                // Store server info for header display
                if let Some(ref server_info) = info.server_info {
                    self.set_server_info(server_info);
                }
                self.loading = false;
            }
            Err(e) => {
                self.handle_data_load_error("health info", e);
            }
        }
    }

    fn handle_health_status_loaded(
        &mut self,
        result: Result<
            splunk_client::models::SplunkHealth,
            std::sync::Arc<splunk_client::ClientError>,
        >,
    ) {
        match result {
            Ok(health) => {
                let new_state = HealthState::from_health_str(&health.health);
                self.set_health_state(new_state);
            }
            Err(_) => {
                // Error getting health - mark as unhealthy
                self.set_health_state(HealthState::Unhealthy);
            }
        }
    }

    // License handler
    fn handle_license_loaded(
        &mut self,
        result: Result<LicenseData, std::sync::Arc<splunk_client::ClientError>>,
    ) {
        match result {
            Ok(data) => {
                self.license_info = Some(data);
                self.loading = false;
            }
            Err(e) => {
                self.handle_data_load_error("license info", e);
            }
        }
    }

    // Apps handlers
    fn handle_apps_loaded(&mut self, apps: Vec<splunk_client::models::App>) {
        let count = apps.len();
        self.apps = Some(apps);
        self.apps_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_more_apps_loaded(&mut self, apps: Vec<splunk_client::models::App>) {
        let count = apps.len();
        if let Some(ref mut existing) = self.apps {
            existing.extend(apps);
        } else {
            self.apps = Some(apps);
        }
        self.apps_pagination.update_loaded(count);
        self.loading = false;
    }

    // Users handlers
    fn handle_users_loaded(&mut self, users: Vec<splunk_client::models::User>) {
        let count = users.len();
        self.users = Some(users);
        self.users_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_more_users_loaded(&mut self, users: Vec<splunk_client::models::User>) {
        let count = users.len();
        if let Some(ref mut existing) = self.users {
            existing.extend(users);
        } else {
            self.users = Some(users);
        }
        self.users_pagination.update_loaded(count);
        self.loading = false;
    }

    // Search peers handlers
    fn handle_search_peers_loaded(&mut self, peers: Vec<splunk_client::models::SearchPeer>) {
        let count = peers.len();
        self.search_peers = Some(peers);
        self.search_peers_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_more_search_peers_loaded(&mut self, peers: Vec<splunk_client::models::SearchPeer>) {
        let count = peers.len();
        if let Some(ref mut existing) = self.search_peers {
            existing.extend(peers);
        } else {
            self.search_peers = Some(peers);
        }
        self.search_peers_pagination.update_loaded(count);
        self.loading = false;
    }

    // Forwarders handlers
    fn handle_forwarders_loaded(&mut self, forwarders: Vec<splunk_client::models::Forwarder>) {
        let count = forwarders.len();
        self.forwarders = Some(forwarders);
        self.forwarders_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_more_forwarders_loaded(&mut self, forwarders: Vec<splunk_client::models::Forwarder>) {
        let count = forwarders.len();
        if let Some(ref mut existing) = self.forwarders {
            existing.extend(forwarders);
        } else {
            self.forwarders = Some(forwarders);
        }
        self.forwarders_pagination.update_loaded(count);
        self.loading = false;
    }

    // Lookups handlers
    fn handle_lookups_loaded(&mut self, lookups: Vec<splunk_client::models::LookupTable>) {
        let count = lookups.len();
        self.lookups = Some(lookups);
        self.lookups_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_more_lookups_loaded(&mut self, lookups: Vec<splunk_client::models::LookupTable>) {
        let count = lookups.len();
        if let Some(ref mut existing) = self.lookups {
            existing.extend(lookups);
        } else {
            self.lookups = Some(lookups);
        }
        self.lookups_pagination.update_loaded(count);
        self.loading = false;
    }

    // Inputs handlers
    fn handle_inputs_loaded(&mut self, inputs: Vec<splunk_client::models::Input>) {
        let count = inputs.len();
        self.inputs = Some(inputs);
        self.inputs_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_more_inputs_loaded(&mut self, inputs: Vec<splunk_client::models::Input>) {
        let count = inputs.len();
        if let Some(ref mut existing) = self.inputs {
            existing.extend(inputs);
        } else {
            self.inputs = Some(inputs);
        }
        self.inputs_pagination.update_loaded(count);
        self.loading = false;
    }

    // Fired alerts handlers
    fn handle_fired_alerts_loaded(&mut self, alerts: Vec<splunk_client::models::FiredAlert>) {
        let count = alerts.len();
        self.fired_alerts = Some(alerts);
        self.fired_alerts_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_more_fired_alerts_loaded(&mut self, alerts: Vec<splunk_client::models::FiredAlert>) {
        let count = alerts.len();
        if let Some(ref mut existing) = self.fired_alerts {
            existing.extend(alerts);
        } else {
            self.fired_alerts = Some(alerts);
        }
        self.fired_alerts_pagination.update_loaded(count);
        self.loading = false;
    }

    // Dashboards handlers
    fn handle_dashboards_loaded(&mut self, dashboards: Vec<splunk_client::models::Dashboard>) {
        let count = dashboards.len();
        self.dashboards = Some(dashboards);
        self.dashboards_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_more_dashboards_loaded(&mut self, dashboards: Vec<splunk_client::models::Dashboard>) {
        let count = dashboards.len();
        if let Some(ref mut existing) = self.dashboards {
            existing.extend(dashboards);
        } else {
            self.dashboards = Some(dashboards);
        }
        self.dashboards_pagination.update_loaded(count);
        self.loading = false;
    }

    // Data models handlers
    fn handle_datamodels_loaded(&mut self, datamodels: Vec<DataModel>) {
        let count = datamodels.len();
        self.data_models = Some(datamodels);
        self.data_models_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_more_datamodels_loaded(&mut self, datamodels: Vec<DataModel>) {
        let count = datamodels.len();
        if let Some(ref mut existing) = self.data_models {
            existing.extend(datamodels);
        } else {
            self.data_models = Some(datamodels);
        }
        self.data_models_pagination.update_loaded(count);
        self.loading = false;
    }

    // Workload management handlers
    fn handle_workload_pools_loaded(&mut self, pools: Vec<splunk_client::models::WorkloadPool>) {
        let count = pools.len();
        self.workload_pools = Some(pools);
        self.workload_pools_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_more_workload_pools_loaded(
        &mut self,
        pools: Vec<splunk_client::models::WorkloadPool>,
    ) {
        let count = pools.len();
        if let Some(ref mut existing) = self.workload_pools {
            existing.extend(pools);
        } else {
            self.workload_pools = Some(pools);
        }
        self.workload_pools_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_workload_rules_loaded(&mut self, rules: Vec<splunk_client::models::WorkloadRule>) {
        let count = rules.len();
        self.workload_rules = Some(rules);
        self.workload_rules_pagination.update_loaded(count);
        self.loading = false;
    }

    fn handle_more_workload_rules_loaded(
        &mut self,
        rules: Vec<splunk_client::models::WorkloadRule>,
    ) {
        let count = rules.len();
        if let Some(ref mut existing) = self.workload_rules {
            existing.extend(rules);
        } else {
            self.workload_rules = Some(rules);
        }
        self.workload_rules_pagination.update_loaded(count);
        self.loading = false;
    }

    // Settings handler
    fn apply_loaded_settings(&mut self, state: splunk_config::PersistedState) {
        use crate::app::state::{parse_sort_column, parse_sort_direction};
        self.auto_refresh = state.auto_refresh;
        self.sort_state.column = parse_sort_column(&state.sort_column);
        self.sort_state.direction = parse_sort_direction(&state.sort_direction);
        self.search_history = state.search_history;
        if let Some(query) = state.last_search_query {
            self.search_input = query;
        }
        self.toasts.push(Toast::info("Settings loaded from file"));
        self.loading = false;
    }

    // Generic error handler for data loading failures
    fn handle_data_load_error(
        &mut self,
        resource_name: &str,
        error: std::sync::Arc<splunk_client::ClientError>,
    ) {
        let error_msg = format!("Failed to load {}: {}", resource_name, error);
        self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
            error.as_ref(),
        ));
        self.toasts.push(Toast::error(error_msg));
        self.loading = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use crate::app::state::HealthState;
    use splunk_client::models::{HealthCheckOutput, SplunkHealth};
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn test_indexes_loaded_updates_state() {
        let mut app = App::new(None, ConnectionContext::default());

        let indexes = vec![splunk_client::models::Index {
            name: "test_index".to_string(),
            max_total_data_size_mb: None,
            current_db_size_mb: 0,
            total_event_count: 0,
            max_warm_db_count: None,
            max_hot_buckets: None,
            frozen_time_period_in_secs: None,
            cold_db_path: None,
            home_path: None,
            thawed_path: None,
            cold_to_frozen_dir: None,
            primary_index: None,
        }];

        app.handle_data_loading_action(Action::IndexesLoaded(Ok(indexes)));

        assert!(app.indexes.is_some());
        assert_eq!(app.indexes.as_ref().unwrap().len(), 1);
        assert!(!app.loading);
    }

    #[test]
    fn test_health_status_loaded_ok() {
        let mut app = App::new(None, ConnectionContext::default());

        let health = SplunkHealth {
            health: "green".to_string(),
            features: HashMap::new(),
        };

        app.handle_data_loading_action(Action::HealthStatusLoaded(Ok(health)));

        assert_eq!(app.health_state, HealthState::Healthy);
    }

    #[test]
    fn test_health_status_loaded_err() {
        let mut app = App::new(None, ConnectionContext::default());
        app.health_state = HealthState::Healthy;

        let error = splunk_client::ClientError::ConnectionRefused("test".to_string());
        app.handle_data_loading_action(Action::HealthStatusLoaded(Err(Arc::new(error))));

        assert_eq!(app.health_state, HealthState::Unhealthy);
        assert_eq!(app.toasts.len(), 1);
    }

    #[test]
    fn test_health_loaded_with_splunkd_health() {
        let mut app = App::new(None, ConnectionContext::default());

        let health_output = HealthCheckOutput {
            server_info: None,
            splunkd_health: Some(SplunkHealth {
                health: "red".to_string(),
                features: HashMap::new(),
            }),
            license_usage: None,
            kvstore_status: None,
            log_parsing_health: None,
        };

        app.handle_data_loading_action(Action::HealthLoaded(Box::new(Ok(health_output))));

        assert_eq!(app.health_state, HealthState::Unhealthy);
    }

    #[test]
    fn test_jobs_loaded_preserves_selection() {
        let mut app = App::new(None, ConnectionContext::default());
        app.jobs_state.select(Some(5));

        let jobs = vec![
            splunk_client::SearchJobStatus {
                sid: "job1".to_string(),
                is_done: false,
                is_finalized: false,
                done_progress: 0.5,
                run_duration: 1.0,
                cursor_time: None,
                scan_count: 100,
                event_count: 50,
                result_count: 25,
                disk_usage: 1024,
                priority: None,
                label: None,
            },
            splunk_client::SearchJobStatus {
                sid: "job2".to_string(),
                is_done: true,
                is_finalized: false,
                done_progress: 1.0,
                run_duration: 2.0,
                cursor_time: None,
                scan_count: 200,
                event_count: 100,
                result_count: 50,
                disk_usage: 2048,
                priority: None,
                label: None,
            },
        ];

        app.handle_data_loading_action(Action::JobsLoaded(Ok(jobs)));

        assert!(app.jobs.is_some());
        // Selection should be clamped to new bounds (2 jobs, so max index is 1)
        assert_eq!(app.jobs_state.selected(), Some(1));
    }

    #[test]
    fn test_data_load_error_shows_toast() {
        let mut app = App::new(None, ConnectionContext::default());

        let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
        app.handle_data_loading_action(Action::IndexesLoaded(Err(Arc::new(error))));

        assert!(app.current_error.is_some());
        assert_eq!(app.toasts.len(), 1);
        assert!(!app.loading);
    }

    #[test]
    fn test_config_files_loaded_updates_state() {
        let mut app = App::new(None, ConnectionContext::default());
        app.loading = true;

        let files = vec![
            splunk_client::models::ConfigFile {
                name: "props".to_string(),
                title: "props.conf".to_string(),
                description: Some("Properties configuration".to_string()),
            },
            splunk_client::models::ConfigFile {
                name: "transforms".to_string(),
                title: "transforms.conf".to_string(),
                description: Some("Transformations".to_string()),
            },
        ];

        app.handle_data_loading_action(Action::ConfigFilesLoaded(Ok(files)));

        assert!(app.config_files.is_some());
        assert_eq!(app.config_files.as_ref().unwrap().len(), 2);
        assert!(!app.loading);
    }

    #[test]
    fn test_config_files_loaded_error_shows_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        app.loading = true;

        let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
        app.handle_data_loading_action(Action::ConfigFilesLoaded(Err(Arc::new(error))));

        assert!(app.current_error.is_some());
        assert_eq!(app.toasts.len(), 1);
        assert!(!app.loading);
        assert!(app.toasts[0].message.contains("config files"));
    }

    #[test]
    fn test_config_stanzas_loaded_updates_state() {
        let mut app = App::new(None, ConnectionContext::default());
        app.loading = true;

        let stanzas = vec![
            splunk_client::models::ConfigStanza {
                name: "default".to_string(),
                config_file: "props".to_string(),
                settings: std::collections::HashMap::new(),
            },
            splunk_client::models::ConfigStanza {
                name: "access_combined".to_string(),
                config_file: "props".to_string(),
                settings: std::collections::HashMap::new(),
            },
        ];

        app.handle_data_loading_action(Action::ConfigStanzasLoaded(Ok(stanzas)));

        assert!(app.config_stanzas.is_some());
        assert_eq!(app.config_stanzas.as_ref().unwrap().len(), 2);
        assert!(!app.loading);
        // filtered_stanza_indices should be rebuilt
        assert_eq!(app.filtered_stanza_indices.len(), 2);
    }

    #[test]
    fn test_config_stanzas_loaded_error_shows_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        app.loading = true;

        let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
        app.handle_data_loading_action(Action::ConfigStanzasLoaded(Err(Arc::new(error))));

        assert!(app.current_error.is_some());
        assert_eq!(app.toasts.len(), 1);
        assert!(!app.loading);
        assert!(app.toasts[0].message.contains("config stanzas"));
    }

    // Macro action handler tests
    #[test]
    fn test_macros_loaded_updates_state() {
        let mut app = App::new(None, ConnectionContext::default());
        app.loading = true;

        let macros = vec![
            splunk_client::models::Macro {
                name: "test_macro".to_string(),
                definition: "index=main | head 10".to_string(),
                args: None,
                description: Some("Test macro".to_string()),
                disabled: false,
                iseval: false,
                validation: None,
                errormsg: None,
            },
            splunk_client::models::Macro {
                name: "param_macro(2)".to_string(),
                definition: "index=$arg1$ | head $arg2$".to_string(),
                args: Some("arg1,arg2".to_string()),
                description: None,
                disabled: false,
                iseval: false,
                validation: None,
                errormsg: None,
            },
        ];

        app.handle_data_loading_action(Action::MacrosLoaded(Ok(macros)));

        assert!(app.macros.is_some());
        assert_eq!(app.macros.as_ref().unwrap().len(), 2);
        assert!(!app.loading);
    }

    #[test]
    fn test_macros_loaded_error_shows_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        app.loading = true;

        let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
        app.handle_data_loading_action(Action::MacrosLoaded(Err(Arc::new(error))));

        assert!(app.current_error.is_some());
        assert_eq!(app.toasts.len(), 1);
        assert!(!app.loading);
        assert!(app.toasts[0].message.contains("macros"));
    }

    #[test]
    fn test_macro_created_success_shows_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        app.loading = true;

        app.handle_data_loading_action(Action::MacroCreated(Ok(())));

        assert!(!app.loading);
        assert_eq!(app.toasts.len(), 1);
        assert!(app.toasts[0].message.contains("created"));
    }

    #[test]
    fn test_macro_created_error_shows_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        app.loading = true;

        let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
        app.handle_data_loading_action(Action::MacroCreated(Err(Arc::new(error))));

        assert!(app.current_error.is_some());
        assert_eq!(app.toasts.len(), 1);
        assert!(!app.loading);
        assert!(app.toasts[0].message.contains("create macro"));
    }

    #[test]
    fn test_macro_updated_success_shows_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        app.loading = true;

        app.handle_data_loading_action(Action::MacroUpdated(Ok(())));

        assert!(!app.loading);
        assert_eq!(app.toasts.len(), 1);
        assert!(app.toasts[0].message.contains("updated"));
    }

    #[test]
    fn test_macro_updated_error_shows_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        app.loading = true;

        let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
        app.handle_data_loading_action(Action::MacroUpdated(Err(Arc::new(error))));

        assert!(app.current_error.is_some());
        assert_eq!(app.toasts.len(), 1);
        assert!(!app.loading);
        assert!(app.toasts[0].message.contains("update macro"));
    }

    #[test]
    fn test_macro_deleted_success_removes_from_list() {
        let mut app = App::new(None, ConnectionContext::default());
        app.loading = true;
        app.macros = Some(vec![
            splunk_client::models::Macro {
                name: "macro_to_delete".to_string(),
                definition: "index=main".to_string(),
                args: None,
                description: None,
                disabled: false,
                iseval: false,
                validation: None,
                errormsg: None,
            },
            splunk_client::models::Macro {
                name: "keep_this_macro".to_string(),
                definition: "index=internal".to_string(),
                args: None,
                description: None,
                disabled: false,
                iseval: false,
                validation: None,
                errormsg: None,
            },
        ]);

        app.handle_data_loading_action(Action::MacroDeleted(Ok("macro_to_delete".to_string())));

        assert!(!app.loading);
        assert_eq!(app.toasts.len(), 1);
        assert!(app.toasts[0].message.contains("deleted"));
        // Verify the macro was removed from the list
        assert_eq!(app.macros.as_ref().unwrap().len(), 1);
        assert_eq!(app.macros.as_ref().unwrap()[0].name, "keep_this_macro");
    }

    #[test]
    fn test_macro_deleted_error_shows_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        app.loading = true;

        let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
        app.handle_data_loading_action(Action::MacroDeleted(Err(Arc::new(error))));

        assert!(app.current_error.is_some());
        assert_eq!(app.toasts.len(), 1);
        assert!(!app.loading);
        assert!(app.toasts[0].message.contains("delete macro"));
    }
}
