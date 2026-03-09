//! Purpose: Helper handlers for TUI data-loading outcomes and pagination state updates.
//! Responsibilities: Apply typed load results to `App` state, update pagination metadata, and map load failures to UI errors.
//! Non-scope: Does not dispatch actions or perform network I/O.
//! Invariants/Assumptions: Each handler leaves loading flags and selection state internally consistent.

use crate::action::LicenseData;
use crate::app::App;
use crate::app::state::HealthState;
use crate::onboarding::OnboardingMilestone;
use crate::ui::Toast;
use splunk_client::models::DataModel;

impl App {
    fn apply_paginated_items<T>(target: &mut Option<Vec<T>>, items: Vec<T>, append: bool) -> usize {
        let count = items.len();

        if append {
            if let Some(existing) = target.as_mut() {
                existing.extend(items);
            } else {
                *target = Some(items);
            }
        } else {
            *target = Some(items);
        }

        count
    }

    pub(crate) fn handle_multi_instance_overview_loaded(
        &mut self,
        data: crate::action::MultiInstanceOverviewData,
    ) {
        if let Some(ref mut existing) = self.multi_instance_data {
            existing.timestamp = data.timestamp;
            // If the incoming data actually has instances (e.g. from a legacy caller), use them
            if !data.instances.is_empty() {
                existing.instances = data.instances;
            }
        } else {
            self.multi_instance_data = Some(data);
        }
        self.loading = false;
    }

    pub(crate) fn handle_multi_instance_instance_loaded(
        &mut self,
        new_instance: crate::action::InstanceOverview,
    ) {
        use crate::action::InstanceStatus;

        if self.multi_instance_data.is_none() {
            self.multi_instance_data = Some(crate::action::MultiInstanceOverviewData {
                timestamp: chrono::Utc::now().to_rfc3339(),
                instances: Vec::new(),
            });
        }

        if let Some(ref mut data) = self.multi_instance_data {
            if let Some(existing) = data
                .instances
                .iter_mut()
                .find(|i| i.profile_name == new_instance.profile_name)
            {
                // Graceful degradation logic:
                // If the new fetch failed but we have healthy cached data, transition to Cached
                if new_instance.error.is_some() && existing.status == InstanceStatus::Healthy {
                    existing.status = InstanceStatus::Cached;
                    existing.error = new_instance.error;
                    // Keep old resources and job_count
                } else {
                    // Update with new data (Success or hard Failure)
                    let mut updated = new_instance;
                    if updated.error.is_none() {
                        updated.status = InstanceStatus::Healthy;
                        updated.last_success_at = Some(chrono::Utc::now().to_rfc3339());
                    } else {
                        updated.status = InstanceStatus::Failed;
                    }
                    *existing = updated;
                }
            } else {
                // New instance discovered or first load
                let mut updated = new_instance;
                if updated.error.is_none() {
                    updated.status = InstanceStatus::Healthy;
                    updated.last_success_at = Some(chrono::Utc::now().to_rfc3339());
                } else {
                    updated.status = InstanceStatus::Failed;
                }
                data.instances.push(updated);
            }
        }
    }

    // Indexes handlers
    pub(crate) fn handle_indexes_loaded(&mut self, indexes: Vec<splunk_client::models::Index>) {
        let count = Self::apply_paginated_items(&mut self.indexes, indexes, false);
        self.indexes_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_indexes_loaded(
        &mut self,
        indexes: Vec<splunk_client::models::Index>,
    ) {
        let count = Self::apply_paginated_items(&mut self.indexes, indexes, true);
        self.indexes_pagination.update_loaded(count);
        self.loading = false;
    }

    // Jobs handlers
    pub(crate) fn handle_jobs_loaded(&mut self, jobs: Vec<splunk_client::SearchJobStatus>) {
        let sel = self.jobs_state.selected();
        let count = Self::apply_paginated_items(&mut self.jobs, jobs, false);
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

    pub(crate) fn handle_more_jobs_loaded(&mut self, jobs: Vec<splunk_client::SearchJobStatus>) {
        let sel = self.jobs_state.selected();
        let count = Self::apply_paginated_items(&mut self.jobs, jobs, true);
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
    pub(crate) fn handle_internal_logs_loaded(
        &mut self,
        logs: Vec<splunk_client::models::LogEntry>,
    ) {
        let sel = self.internal_logs_state.selected();
        self.internal_logs = Some(logs);
        self.loading = false;
        if let Some(logs) = &self.internal_logs {
            self.internal_logs_state
                .select(sel.map(|i| i.min(logs.len().saturating_sub(1))).or(Some(0)));
        }
    }

    // Health handlers
    pub(crate) fn handle_health_loaded(
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
                    let new_state = HealthState::from_health_str(&health.health.to_string());
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

    pub(crate) fn handle_health_status_loaded(
        &mut self,
        result: Result<
            splunk_client::models::SplunkHealth,
            std::sync::Arc<splunk_client::ClientError>,
        >,
    ) {
        match result {
            Ok(health) => {
                let new_state = HealthState::from_health_str(&health.health.to_string());
                self.set_health_state(new_state);
                self.mark_onboarding_milestone(OnboardingMilestone::ConnectionVerified);
            }
            Err(_) => {
                // Error getting health - mark as unhealthy
                self.set_health_state(HealthState::Unhealthy);
            }
        }
    }

    // License handler
    pub(crate) fn handle_license_loaded(
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
    pub(crate) fn handle_apps_loaded(&mut self, apps: Vec<splunk_client::models::App>) {
        let count = Self::apply_paginated_items(&mut self.apps, apps, false);
        self.apps_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_apps_loaded(&mut self, apps: Vec<splunk_client::models::App>) {
        let count = Self::apply_paginated_items(&mut self.apps, apps, true);
        self.apps_pagination.update_loaded(count);
        self.loading = false;
    }

    // Users handlers
    pub(crate) fn handle_users_loaded(&mut self, users: Vec<splunk_client::models::User>) {
        let count = Self::apply_paginated_items(&mut self.users, users, false);
        self.users_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_users_loaded(&mut self, users: Vec<splunk_client::models::User>) {
        let count = Self::apply_paginated_items(&mut self.users, users, true);
        self.users_pagination.update_loaded(count);
        self.loading = false;
    }

    // Roles handlers
    pub(crate) fn handle_roles_loaded(&mut self, roles: Vec<splunk_client::models::Role>) {
        let count = Self::apply_paginated_items(&mut self.roles, roles, false);
        self.roles_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_roles_loaded(&mut self, roles: Vec<splunk_client::models::Role>) {
        let count = Self::apply_paginated_items(&mut self.roles, roles, true);
        self.roles_pagination.update_loaded(count);
        self.loading = false;
    }

    // Search peers handlers
    pub(crate) fn handle_search_peers_loaded(
        &mut self,
        peers: Vec<splunk_client::models::SearchPeer>,
    ) {
        let count = Self::apply_paginated_items(&mut self.search_peers, peers, false);
        self.search_peers_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_search_peers_loaded(
        &mut self,
        peers: Vec<splunk_client::models::SearchPeer>,
    ) {
        let count = Self::apply_paginated_items(&mut self.search_peers, peers, true);
        self.search_peers_pagination.update_loaded(count);
        self.loading = false;
    }

    // Forwarders handlers
    pub(crate) fn handle_forwarders_loaded(
        &mut self,
        forwarders: Vec<splunk_client::models::Forwarder>,
    ) {
        let count = Self::apply_paginated_items(&mut self.forwarders, forwarders, false);
        self.forwarders_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_forwarders_loaded(
        &mut self,
        forwarders: Vec<splunk_client::models::Forwarder>,
    ) {
        let count = Self::apply_paginated_items(&mut self.forwarders, forwarders, true);
        self.forwarders_pagination.update_loaded(count);
        self.loading = false;
    }

    // Lookups handlers
    pub(crate) fn handle_lookups_loaded(
        &mut self,
        lookups: Vec<splunk_client::models::LookupTable>,
    ) {
        let count = Self::apply_paginated_items(&mut self.lookups, lookups, false);
        self.lookups_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_lookups_loaded(
        &mut self,
        lookups: Vec<splunk_client::models::LookupTable>,
    ) {
        let count = Self::apply_paginated_items(&mut self.lookups, lookups, true);
        self.lookups_pagination.update_loaded(count);
        self.loading = false;
    }

    // Inputs handlers
    pub(crate) fn handle_inputs_loaded(&mut self, inputs: Vec<splunk_client::models::Input>) {
        let count = Self::apply_paginated_items(&mut self.inputs, inputs, false);
        self.inputs_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_inputs_loaded(&mut self, inputs: Vec<splunk_client::models::Input>) {
        let count = Self::apply_paginated_items(&mut self.inputs, inputs, true);
        self.inputs_pagination.update_loaded(count);
        self.loading = false;
    }

    // Fired alerts handlers
    pub(crate) fn handle_fired_alerts_loaded(
        &mut self,
        alerts: Vec<splunk_client::models::FiredAlert>,
    ) {
        let count = Self::apply_paginated_items(&mut self.fired_alerts, alerts, false);
        self.fired_alerts_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_fired_alerts_loaded(
        &mut self,
        alerts: Vec<splunk_client::models::FiredAlert>,
    ) {
        let count = Self::apply_paginated_items(&mut self.fired_alerts, alerts, true);
        self.fired_alerts_pagination.update_loaded(count);
        self.loading = false;
    }

    // Dashboards handlers
    pub(crate) fn handle_dashboards_loaded(
        &mut self,
        dashboards: Vec<splunk_client::models::Dashboard>,
    ) {
        let count = Self::apply_paginated_items(&mut self.dashboards, dashboards, false);
        self.dashboards_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_dashboards_loaded(
        &mut self,
        dashboards: Vec<splunk_client::models::Dashboard>,
    ) {
        let count = Self::apply_paginated_items(&mut self.dashboards, dashboards, true);
        self.dashboards_pagination.update_loaded(count);
        self.loading = false;
    }

    // Data models handlers
    pub(crate) fn handle_datamodels_loaded(&mut self, datamodels: Vec<DataModel>) {
        let count = Self::apply_paginated_items(&mut self.data_models, datamodels, false);
        self.data_models_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_datamodels_loaded(&mut self, datamodels: Vec<DataModel>) {
        let count = Self::apply_paginated_items(&mut self.data_models, datamodels, true);
        self.data_models_pagination.update_loaded(count);
        self.loading = false;
    }

    // Workload management handlers
    pub(crate) fn handle_workload_pools_loaded(
        &mut self,
        pools: Vec<splunk_client::models::WorkloadPool>,
    ) {
        let count = Self::apply_paginated_items(&mut self.workload_pools, pools, false);
        self.workload_pools_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_workload_pools_loaded(
        &mut self,
        pools: Vec<splunk_client::models::WorkloadPool>,
    ) {
        let count = Self::apply_paginated_items(&mut self.workload_pools, pools, true);
        self.workload_pools_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_workload_rules_loaded(
        &mut self,
        rules: Vec<splunk_client::models::WorkloadRule>,
    ) {
        let count = Self::apply_paginated_items(&mut self.workload_rules, rules, false);
        self.workload_rules_pagination.update_loaded(count);
        self.loading = false;
    }

    pub(crate) fn handle_more_workload_rules_loaded(
        &mut self,
        rules: Vec<splunk_client::models::WorkloadRule>,
    ) {
        let count = Self::apply_paginated_items(&mut self.workload_rules, rules, true);
        self.workload_rules_pagination.update_loaded(count);
        self.loading = false;
    }

    // Settings handler
    pub(crate) fn apply_loaded_settings(&mut self, state: splunk_config::PersistedState) {
        use crate::app::state::{parse_sort_column, parse_sort_direction};
        self.auto_refresh = state.auto_refresh;
        self.sort_state.column = parse_sort_column(&state.sort_column);
        self.sort_state.direction = parse_sort_direction(&state.sort_direction);
        self.search_history = state.search_history;
        if let Some(query) = state.last_search_query {
            self.search_input.set_value(query);
        }
        // Update search_defaults and sync search_results_page_size to stay consistent
        // with the persisted max_results value (RQ-0331)
        self.search_defaults = state.search_defaults;
        self.search_results_page_size = if self.search_defaults.max_results == 0 {
            splunk_config::SearchDefaults::default().max_results
        } else {
            self.search_defaults.max_results
        };
        self.toasts.push(Toast::info("Settings loaded from file"));
        self.loading = false;
    }

    // Generic error handler for data loading failures
    pub(crate) fn handle_data_load_error(
        &mut self,
        resource_name: &str,
        error: std::sync::Arc<splunk_client::ClientError>,
    ) {
        use crate::ui::popup::{Popup, PopupType};

        if Self::is_expected_unclustered_error(resource_name, error.as_ref()) {
            match resource_name {
                "cluster info" => {
                    self.cluster_info = None;
                }
                "cluster peers" => {
                    self.cluster_peers = None;
                }
                "shc status" => {
                    self.shc_status = None;
                    self.shc_unavailable = true;
                }
                "shc members" => {
                    self.shc_members = None;
                    self.shc_unavailable = true;
                }
                "shc captain" => {
                    self.shc_captain = None;
                    self.shc_unavailable = true;
                }
                "shc config" => {
                    self.shc_config = None;
                    self.shc_unavailable = true;
                }
                _ => {}
            }
            self.loading = false;
            self.loading_since = None;
            return;
        }

        // Use shared classifier for consistent error messaging
        let error_details = crate::error_details::ErrorDetails::from_client_error(error.as_ref());
        let error_msg = format!(
            "Failed to load {}: {}",
            resource_name, error_details.summary
        );

        // Check if this is an auth error and open recovery popup
        if let Some(ref auth_recovery) = error_details.auth_recovery {
            // Emit auth recovery shown metric
            if let Some(ref collector) = self.ux_telemetry {
                collector.record_auth_recovery_shown(auth_recovery.kind);
            }
            self.popup = Some(
                Popup::builder(PopupType::AuthRecovery {
                    kind: auth_recovery.kind,
                })
                .build(),
            );
        }

        self.current_error = Some(error_details);
        self.toasts.push(Toast::error(error_msg));
        self.loading = false;
        self.loading_since = None;
    }

    fn is_expected_unclustered_error(
        resource_name: &str,
        error: &splunk_client::ClientError,
    ) -> bool {
        let is_cluster_resource = matches!(
            resource_name,
            "cluster info"
                | "cluster peers"
                | "shc status"
                | "shc members"
                | "shc captain"
                | "shc config"
        );
        if !is_cluster_resource {
            return false;
        }

        if matches!(
            error,
            splunk_client::ClientError::NotFound(_)
                | splunk_client::ClientError::ApiError { status: 404, .. }
        ) {
            return true;
        }

        // Standalone Splunk can return 503 from SHC endpoints even when the
        // instance is healthy and simply not clustered.
        let is_shc_resource = matches!(
            resource_name,
            "shc status" | "shc members" | "shc captain" | "shc config"
        );
        if !is_shc_resource {
            return false;
        }

        match error {
            splunk_client::ClientError::ApiError {
                status: 503,
                url,
                message,
                ..
            } => {
                let url = url.to_ascii_lowercase();
                let message = message.to_ascii_lowercase();
                url.contains("/services/shcluster/")
                    && (message.contains("service temporarily unavailable")
                        || message.contains("search head cluster")
                        || message.contains("shcluster"))
            }
            _ => false,
        }
    }
}
