//! Action handling for the TUI app.
//!
//! Responsibilities:
//! - Process Actions and mutate App state accordingly
//! - Handle API result actions
//! - Handle navigation actions
//!
//! Non-responsibilities:
//! - Does NOT create Actions (handled by input handlers)
//! - Does NOT perform async operations

use crate::action::Action;
use crate::app::App;
use crate::app::clipboard;
use crate::app::state::{ClusterViewMode, CurrentScreen, HealthState};
use crate::ui::Toast;
use crate::ui::popup::{Popup, PopupType};

impl App {
    /// Pure state mutation based on Action.
    pub fn update(&mut self, action: Action) {
        match action {
            Action::OpenHelpPopup => {
                self.popup = Some(Popup::builder(PopupType::Help).build());
            }
            Action::SwitchToSearch => {
                self.current_screen = CurrentScreen::Search;
            }
            Action::SwitchToSettingsScreen => {
                self.current_screen = CurrentScreen::Settings;
            }
            Action::NextScreen => {
                self.current_screen = self.current_screen.next();
            }
            Action::PreviousScreen => {
                self.current_screen = self.current_screen.previous();
            }
            Action::LoadIndexes {
                count: _,
                offset: _,
            } => {
                self.current_screen = CurrentScreen::Indexes;
                // Reset pagination state for fresh load
                self.indexes_pagination.reset();
            }
            Action::LoadClusterInfo => {
                self.current_screen = CurrentScreen::Cluster;
            }
            Action::ToggleClusterViewMode => {
                self.cluster_view_mode = self.cluster_view_mode.toggle();
                // When switching to peers view, trigger peers load if not already loaded
                if self.cluster_view_mode == ClusterViewMode::Peers && self.cluster_peers.is_none()
                {
                    // The side effect handler will trigger the actual load
                }
            }
            Action::LoadJobs {
                count: _,
                offset: _,
            } => {
                self.current_screen = CurrentScreen::Jobs;
                // Reset pagination state for fresh load
                self.jobs_pagination.reset();
            }
            Action::LoadHealth => {
                self.current_screen = CurrentScreen::Health;
            }
            Action::LoadLicense => {
                self.current_screen = CurrentScreen::License;
            }
            Action::LoadKvstore => {
                self.current_screen = CurrentScreen::Kvstore;
            }
            Action::LoadSavedSearches => {
                self.current_screen = CurrentScreen::SavedSearches;
            }
            Action::LoadInternalLogs {
                count: _,
                earliest: _,
            } => {
                self.current_screen = CurrentScreen::InternalLogs;
            }
            Action::LoadApps {
                count: _,
                offset: _,
            } => {
                self.current_screen = CurrentScreen::Apps;
                // Reset pagination state for fresh load
                self.apps_pagination.reset();
            }
            Action::LoadUsers {
                count: _,
                offset: _,
            } => {
                self.current_screen = CurrentScreen::Users;
                // Reset pagination state for fresh load
                self.users_pagination.reset();
            }
            Action::LoadSearchPeers {
                count: _,
                offset: _,
            } => {
                self.current_screen = CurrentScreen::SearchPeers;
                // Reset pagination state for fresh load
                self.search_peers_pagination.reset();
            }
            Action::LoadInputs {
                count: _,
                offset: _,
            } => {
                self.current_screen = CurrentScreen::Inputs;
                // Reset pagination state for fresh load
                self.inputs_pagination.reset();
            }
            Action::LoadFiredAlerts => {
                self.current_screen = CurrentScreen::FiredAlerts;
                // Reset pagination state for fresh load
                self.fired_alerts_pagination.reset();
            }
            // LoadMore actions - handled by main loop which has access to state
            Action::LoadMoreIndexes
            | Action::LoadMoreJobs
            | Action::LoadMoreApps
            | Action::LoadMoreUsers
            | Action::LoadMoreSearchPeers
            | Action::LoadMoreInputs
            | Action::LoadMoreFiredAlerts => {
                // These are handled in the main loop which has access to pagination state
            }
            Action::NavigateDown => self.next_item(),
            Action::NavigateUp => self.previous_item(),
            Action::PageDown => self.next_page(),
            Action::PageUp => self.previous_page(),
            Action::GoToTop => self.go_to_top(),
            Action::GoToBottom => self.go_to_bottom(),
            Action::EnterSearchMode => {
                self.is_filtering = true;
                // Save current filter for potential cancel
                self.filter_before_edit = self.search_filter.clone();
                // Pre-populate filter_input with existing filter for editing
                self.filter_input = self.search_filter.clone().unwrap_or_default();
            }
            Action::SearchInput(c) => {
                self.filter_input.push(c);
            }
            Action::ClearSearch => {
                self.search_filter = None;
                self.rebuild_filtered_indices();
            }
            Action::CycleSortColumn => {
                self.sort_state.cycle();
                self.rebuild_filtered_indices();
            }
            Action::ToggleSortDirection => {
                self.sort_state.toggle_direction();
                self.rebuild_filtered_indices();
            }
            Action::CycleTheme => {
                self.color_theme = self.color_theme.cycle_next();
                self.theme = splunk_config::Theme::from(self.color_theme);
                self.toasts
                    .push(Toast::info(format!("Theme: {}", self.color_theme)));
            }
            Action::Loading(is_loading) => {
                self.loading = is_loading;
                if is_loading {
                    self.progress = 0.0;
                }
            }
            Action::Progress(p) => {
                self.progress = p;
            }
            Action::Notify(level, message) => {
                self.toasts.push(Toast::new(message, level));
            }
            Action::CopyToClipboard(content) => match clipboard::copy_to_clipboard(content.clone())
            {
                Ok(()) => {
                    let preview = Self::clipboard_preview(&content);
                    self.toasts.push(Toast::info(format!("Copied: {preview}")));
                }
                Err(e) => {
                    self.toasts
                        .push(Toast::error(format!("Clipboard error: {e}")));
                }
            },
            Action::Tick => {
                // Prune expired toasts
                self.toasts.retain(|t| !t.is_expired());
            }
            Action::IndexesLoaded(Ok(indexes)) => {
                let count = indexes.len();
                self.indexes = Some(indexes);
                self.indexes_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreIndexesLoaded(Ok(indexes)) => {
                let count = indexes.len();
                if let Some(ref mut existing) = self.indexes {
                    existing.extend(indexes);
                } else {
                    self.indexes = Some(indexes);
                }
                self.indexes_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreIndexesLoaded(Err(e)) => {
                let error_msg = format!("Failed to load more indexes: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::JobsLoaded(Ok(jobs)) => {
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
            Action::MoreJobsLoaded(Ok(jobs)) => {
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
            Action::MoreJobsLoaded(Err(e)) => {
                let error_msg = format!("Failed to load more jobs: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::SavedSearchesLoaded(Ok(searches)) => {
                self.saved_searches = Some(searches);
                self.loading = false;
            }
            Action::InternalLogsLoaded(Ok(logs)) => {
                let sel = self.internal_logs_state.selected();
                self.internal_logs = Some(logs);
                self.loading = false;
                if let Some(logs) = &self.internal_logs {
                    self.internal_logs_state
                        .select(sel.map(|i| i.min(logs.len().saturating_sub(1))).or(Some(0)));
                }
            }
            Action::ClusterInfoLoaded(Ok(info)) => {
                self.cluster_info = Some(info);
                self.loading = false;
            }
            Action::ClusterPeersLoaded(Ok(peers)) => {
                self.cluster_peers = Some(peers);
                self.loading = false;
            }
            Action::ClusterPeersLoaded(Err(e)) => {
                let error_msg = format!("Failed to load cluster peers: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::HealthLoaded(boxed_result) => match *boxed_result {
                Ok(ref info) => {
                    self.health_info = Some(info.clone());
                    // Update health state from splunkd_health if available
                    if let Some(ref health) = info.splunkd_health {
                        let new_state = HealthState::from_health_str(&health.health);
                        self.set_health_state(new_state);
                    }
                    // Store server info for header display (RQ-0134)
                    if let Some(ref server_info) = info.server_info {
                        self.set_server_info(server_info);
                    }
                    self.loading = false;
                }
                Err(e) => {
                    let error_msg = format!("Failed to load health info: {}", e);
                    self.current_error = Some(
                        crate::error_details::ErrorDetails::from_client_error(e.as_ref()),
                    );
                    self.toasts.push(Toast::error(error_msg));
                    self.loading = false;
                }
            },
            Action::LicenseLoaded(boxed_result) => match *boxed_result {
                Ok(ref data) => {
                    self.license_info = Some(data.clone());
                    self.loading = false;
                }
                Err(e) => {
                    let error_msg = format!("Failed to load license info: {}", e);
                    self.current_error = Some(
                        crate::error_details::ErrorDetails::from_client_error(e.as_ref()),
                    );
                    self.toasts.push(Toast::error(error_msg));
                    self.loading = false;
                }
            },
            Action::KvstoreLoaded(Ok(status)) => {
                self.kvstore_status = Some(status);
                self.loading = false;
            }
            Action::KvstoreLoaded(Err(e)) => {
                let error_msg = format!("Failed to load KVStore status: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::HealthStatusLoaded(result) => match result {
                Ok(health) => {
                    let new_state = HealthState::from_health_str(&health.health);
                    self.set_health_state(new_state);
                }
                Err(_) => {
                    // Error getting health - mark as unhealthy
                    self.set_health_state(HealthState::Unhealthy);
                }
            },
            Action::SearchStarted(query) => {
                self.running_query = Some(query);
            }
            Action::SearchComplete(Ok((results, sid, total))) => {
                let results_count = results.len() as u64;
                self.set_search_results(results);
                self.search_sid = Some(sid);
                // Set pagination state from initial search results
                self.search_results_total_count = total;
                self.search_has_more_results = if let Some(t) = total {
                    results_count < t
                } else {
                    // When total is None, infer from page fullness
                    // Note: initial fetch in main.rs uses 1000, but we use app's page_size for consistency
                    results_count >= self.search_results_page_size
                };
                // Use running_query for status message, falling back to search_input if not set
                let query_for_status = self
                    .running_query
                    .take()
                    .unwrap_or_else(|| self.search_input.clone());
                self.search_status = format!("Search complete: {}", query_for_status);
                self.loading = false;
            }
            Action::MoreSearchResultsLoaded(Ok((results, _offset, total))) => {
                self.append_search_results(results, total);
                self.loading = false;
            }
            Action::MoreSearchResultsLoaded(Err(e)) => {
                let error_msg = format!("Failed to load more results: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::JobOperationComplete(msg) => {
                self.selected_jobs.clear();
                self.search_status = msg;
                self.loading = false;
            }
            Action::IndexesLoaded(Err(e)) => {
                let error_msg = format!("Failed to load indexes: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::JobsLoaded(Err(e)) => {
                let error_msg = format!("Failed to load jobs: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::SavedSearchesLoaded(Err(e)) => {
                let error_msg = format!("Failed to load saved searches: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::InternalLogsLoaded(Err(e)) => {
                let error_msg = format!("Failed to load internal logs: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::AppsLoaded(Ok(apps)) => {
                let count = apps.len();
                self.apps = Some(apps);
                self.apps_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreAppsLoaded(Ok(apps)) => {
                let count = apps.len();
                if let Some(ref mut existing) = self.apps {
                    existing.extend(apps);
                } else {
                    self.apps = Some(apps);
                }
                self.apps_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreAppsLoaded(Err(e)) => {
                let error_msg = format!("Failed to load more apps: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::AppsLoaded(Err(e)) => {
                let error_msg = format!("Failed to load apps: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::UsersLoaded(Ok(users)) => {
                let count = users.len();
                self.users = Some(users);
                self.users_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreUsersLoaded(Ok(users)) => {
                let count = users.len();
                if let Some(ref mut existing) = self.users {
                    existing.extend(users);
                } else {
                    self.users = Some(users);
                }
                self.users_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreUsersLoaded(Err(e)) => {
                let error_msg = format!("Failed to load more users: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::UsersLoaded(Err(e)) => {
                let error_msg = format!("Failed to load users: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::SearchPeersLoaded(Ok(peers)) => {
                let count = peers.len();
                self.search_peers = Some(peers);
                self.search_peers_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreSearchPeersLoaded(Ok(peers)) => {
                let count = peers.len();
                if let Some(ref mut existing) = self.search_peers {
                    existing.extend(peers);
                } else {
                    self.search_peers = Some(peers);
                }
                self.search_peers_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreSearchPeersLoaded(Err(e)) => {
                let error_msg = format!("Failed to load more search peers: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::SearchPeersLoaded(Err(e)) => {
                let error_msg = format!("Failed to load search peers: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::ForwardersLoaded(Ok(forwarders)) => {
                let count = forwarders.len();
                self.forwarders = Some(forwarders);
                self.forwarders_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreForwardersLoaded(Ok(forwarders)) => {
                let count = forwarders.len();
                if let Some(ref mut existing) = self.forwarders {
                    existing.extend(forwarders);
                } else {
                    self.forwarders = Some(forwarders);
                }
                self.forwarders_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreForwardersLoaded(Err(e)) => {
                let error_msg = format!("Failed to load more forwarders: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::ForwardersLoaded(Err(e)) => {
                let error_msg = format!("Failed to load forwarders: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::InputsLoaded(Ok(inputs)) => {
                let count = inputs.len();
                self.inputs = Some(inputs);
                self.inputs_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreInputsLoaded(Ok(inputs)) => {
                let count = inputs.len();
                if let Some(ref mut existing) = self.inputs {
                    existing.extend(inputs);
                } else {
                    self.inputs = Some(inputs);
                }
                self.inputs_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreInputsLoaded(Err(e)) => {
                let error_msg = format!("Failed to load more inputs: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::InputsLoaded(Err(e)) => {
                let error_msg = format!("Failed to load inputs: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::FiredAlertsLoaded(Ok(alerts)) => {
                let count = alerts.len();
                self.fired_alerts = Some(alerts);
                self.fired_alerts_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreFiredAlertsLoaded(Ok(alerts)) => {
                let count = alerts.len();
                if let Some(ref mut existing) = self.fired_alerts {
                    existing.extend(alerts);
                } else {
                    self.fired_alerts = Some(alerts);
                }
                self.fired_alerts_pagination.update_loaded(count);
                self.loading = false;
            }
            Action::MoreFiredAlertsLoaded(Err(e)) => {
                let error_msg = format!("Failed to load more fired alerts: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::FiredAlertsLoaded(Err(e)) => {
                let error_msg = format!("Failed to load fired alerts: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::SettingsLoaded(state) => {
                self.auto_refresh = state.auto_refresh;
                self.sort_state.column = crate::app::state::parse_sort_column(&state.sort_column);
                self.sort_state.direction =
                    crate::app::state::parse_sort_direction(&state.sort_direction);
                self.search_history = state.search_history;
                if let Some(query) = state.last_search_query {
                    self.search_input = query;
                }
                self.toasts.push(Toast::info("Settings loaded from file"));
                self.loading = false;
            }
            Action::OverviewLoaded(data) => {
                self.overview_data = Some(data);
                self.loading = false;
            }
            Action::MultiInstanceOverviewLoaded(data) => {
                self.multi_instance_data = Some(data);
                self.loading = false;
            }
            Action::ClusterInfoLoaded(Err(e)) => {
                let error_msg = format!("Failed to load cluster info: {}", e);
                self.current_error = Some(crate::error_details::ErrorDetails::from_client_error(
                    e.as_ref(),
                ));
                self.toasts.push(Toast::error(error_msg));
                self.loading = false;
            }
            Action::SearchComplete(Err((error_msg, details))) => {
                self.current_error = Some(details);
                self.toasts.push(Toast::error(error_msg));
                self.running_query = None; // Clear the running query on error
                self.loading = false;
            }
            Action::SplValidationResult {
                valid,
                errors,
                warnings,
            } => {
                self.spl_validation_state = crate::app::SplValidationState {
                    valid: Some(valid),
                    errors,
                    warnings,
                    last_validated: Some(std::time::Instant::now()),
                };
                self.spl_validation_pending = false;
            }
            Action::ShowErrorDetails(details) => {
                self.current_error = Some(details);
                self.popup = Some(Popup::builder(PopupType::ErrorDetails).build());
            }
            Action::ShowErrorDetailsFromCurrent => {
                if self.current_error.is_some() {
                    self.popup = Some(Popup::builder(PopupType::ErrorDetails).build());
                }
            }
            Action::ClearErrorDetails => {
                self.current_error = None;
                self.popup = None;
            }
            Action::InspectJob => {
                // Transition to job inspect screen if we have jobs and a selection
                if self.jobs.as_ref().map(|j| !j.is_empty()).unwrap_or(false)
                    && self.jobs_state.selected().is_some()
                {
                    self.current_screen = CurrentScreen::JobInspect;
                }
            }
            Action::ExitInspectMode => {
                // Return to jobs screen
                self.current_screen = CurrentScreen::Jobs;
            }
            // Profile switching actions
            Action::OpenProfileSwitcher => {
                // This action is handled in main.rs side effects which will
                // send the profile list and trigger the popup opening
            }
            Action::OpenProfileSelectorWithList(profiles) => {
                if !profiles.is_empty() {
                    self.popup = Some(
                        Popup::builder(PopupType::ProfileSelector {
                            profiles,
                            selected_index: 0,
                        })
                        .build(),
                    );
                }
            }
            Action::ProfileSelected(_) => {
                // This action is handled in main.rs side effects
                // It triggers the actual profile switch with new client creation
            }
            Action::ProfileSwitchResult(Ok(ctx)) => {
                // Update connection context with new profile info
                self.profile_name = ctx.profile_name;
                self.base_url = Some(ctx.base_url);
                self.auth_mode = Some(ctx.auth_mode);
                // Clear server info until new health check loads
                self.server_version = None;
                self.server_build = None;
                self.toasts.push(Toast::info(format!(
                    "Switched to profile: {}",
                    self.profile_name.as_deref().unwrap_or("default")
                )));
            }
            Action::ProfileSwitchResult(Err(e)) => {
                self.toasts
                    .push(Toast::error(format!("Failed to switch profile: {}", e)));
            }
            Action::ClearAllData => {
                // Clear all cached data after profile switch
                self.indexes = None;
                self.jobs = None;
                self.saved_searches = None;
                self.internal_logs = None;
                self.cluster_info = None;
                self.cluster_peers = None;
                self.health_info = None;
                self.license_info = None;
                self.kvstore_status = None;
                self.apps = None;
                self.users = None;
                self.search_peers = None;
                self.fired_alerts = None;
                self.search_results.clear();
                self.search_sid = None;
                self.search_results_total_count = None;
                self.search_has_more_results = false;
                // Reset list states
                self.indexes_state.select(Some(0));
                self.jobs_state.select(Some(0));
                self.saved_searches_state.select(Some(0));
                self.internal_logs_state.select(Some(0));
                self.cluster_peers_state.select(Some(0));
                self.apps_state.select(Some(0));
                self.users_state.select(Some(0));
                self.search_peers_state.select(Some(0));
                self.fired_alerts_state.select(Some(0));
                // Trigger reload for current screen
                // The load action will be sent by main.rs after this
            }
            Action::Resize(width, height) => {
                // Update last_area to reflect new terminal dimensions
                self.last_area = ratatui::layout::Rect::new(0, 0, width, height);
            }
            Action::OpenCreateIndexDialog => {
                self.popup = Some(
                    Popup::builder(PopupType::CreateIndex {
                        name_input: String::new(),
                        max_data_size_mb: None,
                        max_hot_buckets: None,
                        max_warm_db_count: None,
                        frozen_time_period_secs: None,
                        home_path: None,
                        cold_db_path: None,
                        thawed_path: None,
                        cold_to_frozen_dir: None,
                    })
                    .build(),
                );
            }
            // Profile management actions
            Action::OpenCreateProfileDialog => {
                self.popup = Some(
                    Popup::builder(PopupType::CreateProfile {
                        name_input: String::new(),
                        base_url_input: String::new(),
                        username_input: String::new(),
                        password_input: String::new(),
                        api_token_input: String::new(),
                        skip_verify: false,
                        timeout_seconds: 30,
                        max_retries: 3,
                        use_keyring: false,
                        selected_field: crate::ui::popup::ProfileField::Name,
                    })
                    .build(),
                );
            }
            Action::OpenEditProfileDialogWithData {
                original_name,
                name_input,
                base_url_input,
                username_input,
                skip_verify,
                timeout_seconds,
                max_retries,
            } => {
                self.popup = Some(
                    Popup::builder(PopupType::EditProfile {
                        original_name,
                        name_input,
                        base_url_input,
                        username_input,
                        password_input: String::new(), // Empty means "keep existing"
                        api_token_input: String::new(), // Empty means "keep existing"
                        skip_verify,
                        timeout_seconds,
                        max_retries: max_retries as u64,
                        use_keyring: false,
                        selected_field: crate::ui::popup::ProfileField::Name,
                    })
                    .build(),
                );
            }
            Action::OpenDeleteProfileConfirm { name } => {
                self.popup = Some(
                    Popup::builder(PopupType::DeleteProfileConfirm { profile_name: name }).build(),
                );
            }
            Action::ProfileSaved(Ok(profile_name)) => {
                self.popup = None;
                self.toasts.push(Toast::info(format!(
                    "Profile '{}' saved successfully",
                    profile_name
                )));
            }
            Action::ProfileSaved(Err(error_msg)) => {
                self.toasts.push(Toast::error(error_msg));
            }
            Action::ProfileDeleted(Ok(profile_name)) => {
                self.popup = None;
                self.toasts.push(Toast::info(format!(
                    "Profile '{}' deleted successfully",
                    profile_name
                )));
                // If the deleted profile was the current one, clear the connection context
                if self.profile_name.as_ref() == Some(&profile_name) {
                    self.profile_name = None;
                }
            }
            Action::ProfileDeleted(Err(error_msg)) => {
                self.toasts.push(Toast::error(error_msg));
            }
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use splunk_client::models::{HealthCheckOutput, SplunkHealth};
    use std::sync::Arc;

    #[test]
    fn test_health_status_loaded_action_ok() {
        let mut app = App::new(None, ConnectionContext::default());

        // Simulate receiving a healthy status
        let health = SplunkHealth {
            health: "green".to_string(),
            features: std::collections::HashMap::new(),
        };

        app.update(Action::HealthStatusLoaded(Ok(health)));

        assert_eq!(app.health_state, HealthState::Healthy);
    }

    #[test]
    fn test_health_status_loaded_action_err() {
        let mut app = App::new(None, ConnectionContext::default());
        app.health_state = HealthState::Healthy;

        // Simulate error - should set to unhealthy
        let error = splunk_client::ClientError::ConnectionRefused("test".to_string());
        app.update(Action::HealthStatusLoaded(Err(Arc::new(error))));

        assert_eq!(app.health_state, HealthState::Unhealthy);
        // Should emit toast since we went from Healthy to Unhealthy
        assert_eq!(app.toasts.len(), 1);
    }

    #[test]
    fn test_health_loaded_action_with_splunkd_health() {
        let mut app = App::new(None, ConnectionContext::default());

        // Simulate receiving HealthCheckOutput with splunkd_health
        let health_output = HealthCheckOutput {
            server_info: None,
            splunkd_health: Some(SplunkHealth {
                health: "red".to_string(),
                features: std::collections::HashMap::new(),
            }),
            license_usage: None,
            kvstore_status: None,
            log_parsing_health: None,
        };

        app.update(Action::HealthLoaded(Box::new(Ok(health_output))));

        assert_eq!(app.health_state, HealthState::Unhealthy);
    }

    #[test]
    fn test_set_health_state_healthy_to_unhealthy_emits_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        app.health_state = HealthState::Healthy;

        // Set to unhealthy should emit a toast
        app.set_health_state(HealthState::Unhealthy);

        assert_eq!(app.health_state, HealthState::Unhealthy);
        assert_eq!(app.toasts.len(), 1);
        assert_eq!(
            app.toasts[0].message,
            "Splunk health status changed to unhealthy"
        );
    }

    #[test]
    fn test_set_health_state_unknown_to_unhealthy_emits_no_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        // Default state is Unknown
        assert_eq!(app.health_state, HealthState::Unknown);

        // Set to unhealthy from Unknown should not emit a toast
        app.set_health_state(HealthState::Unhealthy);

        assert_eq!(app.health_state, HealthState::Unhealthy);
        assert_eq!(app.toasts.len(), 0);
    }

    #[test]
    fn test_set_health_state_healthy_to_unknown_emits_no_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        app.health_state = HealthState::Healthy;

        // Set to unknown should not emit a toast
        app.set_health_state(HealthState::Unknown);

        assert_eq!(app.health_state, HealthState::Unknown);
        assert_eq!(app.toasts.len(), 0);
    }

    #[test]
    fn test_set_health_state_unhealthy_to_healthy_emits_no_toast() {
        let mut app = App::new(None, ConnectionContext::default());
        app.health_state = HealthState::Unhealthy;

        // Set to healthy should not emit a toast (only Healthy -> Unhealthy does)
        app.set_health_state(HealthState::Healthy);

        assert_eq!(app.health_state, HealthState::Healthy);
        assert_eq!(app.toasts.len(), 0);
    }
}
