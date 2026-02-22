//! Data loading action handlers for the TUI app.
//!
//! Purpose:
//! - Dispatch `*Loaded` action results to typed state-update handlers.
//!
//! Responsibilities:
//! - Handle *Loaded and More*Loaded actions for all data types
//! - Update pagination state when data is loaded
//! - Handle error cases for data loading failures
//! - Rebuild filtered indices and restore selection when needed
//!
//! Non-scope:
//! - Does not perform network requests or background polling.
//!
//! Invariants/Assumptions:
//! - Dispatch remains side-effect free beyond deterministic in-memory state mutation.

use crate::action::Action;
use crate::app::App;
use crate::ui::Toast;

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
            Action::IndexCreated(Ok(index)) => {
                self.toasts
                    .push(Toast::success(format!("Index '{}' created", index.name)));
                self.loading = false;
                // Add to list if present
                if let Some(ref mut indexes) = self.indexes {
                    indexes.push(index);
                }
            }
            Action::IndexCreated(Err(e)) => {
                self.handle_data_load_error("create index", e);
            }
            Action::IndexModified(Ok(index)) => {
                self.toasts
                    .push(Toast::success(format!("Index '{}' modified", index.name)));
                self.loading = false;
                // Update in list if present
                if let Some(ref mut indexes) = self.indexes {
                    if let Some(idx) = indexes.iter().position(|i| i.name == index.name) {
                        indexes[idx] = index;
                    }
                }
            }
            Action::IndexModified(Err(e)) => {
                self.handle_data_load_error("modify index", e);
            }
            Action::IndexDeleted(Ok(name)) => {
                self.toasts
                    .push(Toast::success(format!("Index '{}' deleted", name)));
                self.loading = false;
                // Remove from local list if present
                if let Some(ref mut indexes) = self.indexes {
                    indexes.retain(|i| i.name != name);
                }
            }
            Action::IndexDeleted(Err(e)) => {
                self.handle_data_load_error("delete index", e);
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
            Action::UserCreated(Ok(user)) => {
                self.toasts
                    .push(Toast::success(format!("User '{}' created", user.name)));
                self.loading = false;
                // Add to list if present
                if let Some(ref mut users) = self.users {
                    users.push(user);
                }
            }
            Action::UserCreated(Err(e)) => {
                self.handle_data_load_error("create user", e);
            }
            Action::UserModified(Ok(user)) => {
                self.toasts
                    .push(Toast::success(format!("User '{}' modified", user.name)));
                self.loading = false;
                // Update in list if present
                if let Some(ref mut users) = self.users {
                    if let Some(idx) = users.iter().position(|u| u.name == user.name) {
                        users[idx] = user;
                    }
                }
            }
            Action::UserModified(Err(e)) => {
                self.handle_data_load_error("modify user", e);
            }
            Action::UserDeleted(Ok(name)) => {
                self.toasts
                    .push(Toast::success(format!("User '{}' deleted", name)));
                self.loading = false;
                // Remove from local list if present
                if let Some(ref mut users) = self.users {
                    users.retain(|u| u.name != name);
                }
            }
            Action::UserDeleted(Err(e)) => {
                self.handle_data_load_error("delete user", e);
            }

            // Roles
            Action::RolesLoaded(Ok(roles)) => {
                self.handle_roles_loaded(roles);
            }
            Action::RolesLoaded(Err(e)) => {
                self.handle_data_load_error("roles", e);
            }
            Action::MoreRolesLoaded(Ok(roles)) => {
                self.handle_more_roles_loaded(roles);
            }
            Action::MoreRolesLoaded(Err(e)) => {
                self.handle_data_load_error("more roles", e);
            }
            Action::RoleCreated(Ok(role)) => {
                self.toasts
                    .push(Toast::success(format!("Role '{}' created", role.name)));
                self.loading = false;
                if let Some(ref mut roles) = self.roles {
                    roles.push(role);
                }
            }
            Action::RoleCreated(Err(e)) => {
                self.handle_data_load_error("create role", e);
            }
            Action::RoleModified(Ok(role)) => {
                self.toasts
                    .push(Toast::success(format!("Role '{}' modified", role.name)));
                self.loading = false;
                if let Some(ref mut roles) = self.roles {
                    if let Some(idx) = roles.iter().position(|r| r.name == role.name) {
                        roles[idx] = role;
                    }
                }
            }
            Action::RoleModified(Err(e)) => {
                self.handle_data_load_error("modify role", e);
            }
            Action::RoleDeleted(Ok(name)) => {
                self.toasts
                    .push(Toast::success(format!("Role '{}' deleted", name)));
                self.loading = false;
                if let Some(ref mut roles) = self.roles {
                    roles.retain(|r| r.name != name);
                }
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

            // SHC
            Action::ShcStatusLoaded(Ok(status)) => {
                self.shc_status = Some(status);
                self.shc_unavailable = false;
                self.loading = false;
            }
            Action::ShcStatusLoaded(Err(e)) => {
                self.handle_data_load_error("shc status", e);
            }
            Action::ShcMembersLoaded(Ok(members)) => {
                self.shc_members = Some(members);
                self.shc_unavailable = false;
                self.loading = false;
                if let Some(members) = &self.shc_members {
                    self.shc_members_state.select(Some(
                        self.shc_members_state
                            .selected()
                            .unwrap_or(0)
                            .min(members.len().saturating_sub(1)),
                    ));
                }
            }
            Action::ShcMembersLoaded(Err(e)) => {
                self.handle_data_load_error("shc members", e);
            }
            Action::ShcCaptainLoaded(Ok(captain)) => {
                self.shc_captain = Some(captain);
                self.shc_unavailable = false;
                self.loading = false;
            }
            Action::ShcCaptainLoaded(Err(e)) => {
                self.handle_data_load_error("shc captain", e);
            }
            Action::ShcConfigLoaded(Ok(config)) => {
                self.shc_config = Some(config);
                self.shc_unavailable = false;
                self.loading = false;
            }
            Action::ShcConfigLoaded(Err(e)) => {
                self.handle_data_load_error("shc config", e);
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
                self.handle_multi_instance_overview_loaded(data);
            }
            Action::MultiInstanceInstanceLoaded(instance) => {
                self.handle_multi_instance_instance_loaded(instance);
            }

            _ => {}
        }
    }
}

#[cfg(test)]
#[path = "data_loading_tests.rs"]
mod tests;
