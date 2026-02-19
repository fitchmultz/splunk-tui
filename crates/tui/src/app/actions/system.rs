//! System action handlers for the TUI app.
//!
//! Responsibilities:
//! - Handle loading/progress state updates
//! - Handle notifications and toast messages
//! - Handle clipboard operations
//! - Handle terminal resize events
//! - Handle search/filter mode transitions
//! - Handle theme cycling
//! - Handle error detail popups
//! - Handle SPL validation results

use crate::action::Action;
use crate::app::App;
use crate::app::clipboard;
use crate::app::input::components::SingleLineInput;
use crate::ui::Toast;

impl App {
    /// Handle system/miscellaneous actions.
    pub fn handle_system_action(&mut self, action: Action) {
        match action {
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
            Action::Tick => {
                // Prune expired toasts
                self.toasts.retain(|t| !t.is_expired());
                // Prune expired undo buffer entries and execute pending ones
                self.process_undo_buffer();
                // Advance spinner animation frame
                if self.loading {
                    self.spinner_frame = (self.spinner_frame + 1) % 8;
                }
            }
            Action::CopyToClipboard(content) => {
                self.handle_copy_to_clipboard(content);
            }
            Action::Resize(width, height) => {
                // Update last_area to reflect new terminal dimensions
                self.last_area = ratatui::layout::Rect::new(0, 0, width, height);
                // Clamp scroll offsets to ensure they don't exceed available data
                self.clamp_scroll_offsets();
            }
            Action::EnterSearchMode => {
                self.enter_search_mode();
            }
            Action::SearchInput(c) => {
                use crate::app::input::components::SingleLineInput;
                let mut input = SingleLineInput::from(self.filter_input.value().to_string());
                input.push(c);
                self.filter_input = input;
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
                self.cycle_theme();
            }
            Action::SplValidationResult {
                valid,
                errors,
                warnings,
                request_id,
            } => {
                self.handle_spl_validation_result(valid, errors, warnings, request_id);
            }
            Action::ShowErrorDetails(details) => {
                self.show_error_details(details);
            }
            Action::ShowErrorDetailsFromCurrent => {
                self.show_error_details_from_current();
            }
            Action::ClearErrorDetails => {
                self.current_error = None;
                self.popup = None;
            }
            Action::JobOperationComplete(msg) => {
                self.selected_jobs.clear();
                self.search_status = msg;
                self.loading = false;
            }
            Action::OpenCreateIndexDialog => {
                self.open_create_index_dialog();
            }
            Action::OpenModifyIndexDialog { name } => {
                self.open_modify_index_dialog(name);
            }
            Action::OpenDeleteIndexConfirm { name } => {
                self.popup = Some(
                    crate::ui::popup::Popup::builder(
                        crate::ui::popup::PopupType::DeleteIndexConfirm { index_name: name },
                    )
                    .build(),
                );
            }
            Action::OpenCreateUserDialog => {
                self.open_create_user_dialog();
            }
            Action::OpenModifyUserDialog { name } => {
                self.open_modify_user_dialog(name);
            }
            Action::OpenDeleteUserConfirm { name } => {
                self.popup = Some(
                    crate::ui::popup::Popup::builder(
                        crate::ui::popup::PopupType::DeleteUserConfirm { user_name: name },
                    )
                    .build(),
                );
            }
            Action::OpenCreateRoleDialog => {
                self.open_create_role_dialog();
            }
            Action::OpenModifyRoleDialog { name } => {
                self.open_modify_role_dialog(name);
            }
            Action::OpenDeleteRoleConfirm { name } => {
                self.popup = Some(
                    crate::ui::popup::Popup::builder(
                        crate::ui::popup::PopupType::DeleteRoleConfirm { role_name: name },
                    )
                    .build(),
                );
            }
            Action::OpenCreateMacroDialog => {
                self.open_create_macro_dialog();
            }
            Action::EditSavedSearch => {
                self.open_edit_saved_search_dialog();
            }
            Action::EditMacro => {
                self.open_edit_macro_dialog();
            }
            Action::SavedSearchUpdated(result) => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts.push(Toast::info("Saved search updated"));
                        // Refresh the saved searches list
                        self.saved_searches = None;
                    }
                    Err(e) => {
                        self.toasts.push(Toast::error(format!(
                            "Failed to update saved search: {}",
                            e
                        )));
                    }
                }
            }
            Action::OpenCreateSavedSearchDialog => {
                self.popup = Some(
                    crate::ui::popup::Popup::builder(
                        crate::ui::popup::PopupType::CreateSavedSearch {
                            name_input: String::new(),
                            search_input: String::new(),
                            description_input: String::new(),
                            disabled: false,
                            selected_field: crate::ui::popup::SavedSearchField::Name,
                        },
                    )
                    .build(),
                );
            }
            Action::OpenDeleteSavedSearchConfirm { name } => {
                self.popup = Some(
                    crate::ui::popup::Popup::builder(
                        crate::ui::popup::PopupType::DeleteSavedSearchConfirm { search_name: name },
                    )
                    .build(),
                );
            }
            Action::SavedSearchCreated(result) => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts
                            .push(Toast::success("Saved search created successfully"));
                        // Refresh the saved searches list
                        self.saved_searches = None;
                    }
                    Err(e) => {
                        self.toasts.push(Toast::error(format!(
                            "Failed to create saved search: {}",
                            e
                        )));
                    }
                }
            }
            Action::SavedSearchDeleted(result) => {
                self.loading = false;
                match result {
                    Ok(name) => {
                        self.toasts
                            .push(Toast::success(format!("Saved search '{}' deleted", name)));
                        // Remove from local list if present
                        if let Some(searches) = &mut self.saved_searches {
                            searches.retain(|s| s.name != name);
                        }
                    }
                    Err(e) => {
                        self.toasts.push(Toast::error(format!(
                            "Failed to delete saved search: {}",
                            e
                        )));
                    }
                }
            }
            Action::SavedSearchToggled(result) => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts
                            .push(Toast::success("Saved search state updated"));
                        // Refresh to get updated state
                        self.saved_searches = None;
                    }
                    Err(e) => {
                        self.toasts.push(Toast::error(format!(
                            "Failed to toggle saved search: {}",
                            e
                        )));
                    }
                }
            }
            // Cluster management result actions
            Action::MaintenanceModeSet { result } => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts.push(Toast::info("Maintenance mode updated"));
                        // Refresh cluster info to show updated state
                        self.trigger_load_cluster_info();
                    }
                    Err(e) => {
                        self.toasts.push(Toast::error(format!(
                            "Failed to set maintenance mode: {}",
                            e
                        )));
                    }
                }
            }
            Action::ClusterRebalanced { result } => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts.push(Toast::info("Cluster rebalance initiated"));
                    }
                    Err(e) => {
                        self.toasts
                            .push(Toast::error(format!("Failed to rebalance cluster: {}", e)));
                    }
                }
            }
            Action::PeerDecommissioned { result } => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts.push(Toast::info("Peer decommission initiated"));
                        // Refresh peers list
                        self.trigger_load_cluster_peers();
                    }
                    Err(e) => {
                        self.toasts
                            .push(Toast::error(format!("Failed to decommission peer: {}", e)));
                    }
                }
            }
            Action::PeerRemoved { result } => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts.push(Toast::info("Peer removed from cluster"));
                        // Refresh peers list
                        self.trigger_load_cluster_peers();
                    }
                    Err(e) => {
                        self.toasts
                            .push(Toast::error(format!("Failed to remove peer: {}", e)));
                    }
                }
            }
            // Lookup operations
            Action::OpenDeleteLookupConfirm { name } => {
                self.popup = Some(
                    crate::ui::popup::Popup::builder(
                        crate::ui::popup::PopupType::DeleteLookupConfirm { lookup_name: name },
                    )
                    .build(),
                );
            }
            Action::LookupDownloaded(result) => {
                self.loading = false;
                match result {
                    Ok(name) => {
                        self.toasts
                            .push(Toast::success(format!("Lookup '{}' downloaded", name)));
                    }
                    Err(e) => {
                        self.toasts
                            .push(Toast::error(format!("Failed to download lookup: {}", e)));
                    }
                }
            }
            Action::LookupDeleted(result) => {
                self.loading = false;
                match result {
                    Ok(name) => {
                        self.toasts
                            .push(Toast::success(format!("Lookup '{}' deleted", name)));
                        // Remove from local list if present
                        if let Some(lookups) = &mut self.lookups {
                            lookups.retain(|l| l.name != name);
                        }
                    }
                    Err(e) => {
                        self.toasts
                            .push(Toast::error(format!("Failed to delete lookup: {}", e)));
                    }
                }
            }
            Action::ExportSuccess(path) => {
                // Add to recent export paths, keeping most recent first and limiting size
                const MAX_RECENT_EXPORTS: usize = 10;
                let path_str = path.to_string_lossy().to_string();
                // Remove if already exists to avoid duplicates
                self.recent_export_paths.retain(|p| p != &path_str);
                // Insert at the beginning
                self.recent_export_paths.insert(0, path_str);
                // Keep only the most recent paths
                if self.recent_export_paths.len() > MAX_RECENT_EXPORTS {
                    self.recent_export_paths.truncate(MAX_RECENT_EXPORTS);
                }
            }
            Action::ConnectionDiagnosticsLoaded(result) => {
                self.loading = false;
                match result {
                    Ok(diagnostics) => {
                        self.popup = Some(
                            crate::ui::popup::Popup::builder(
                                crate::ui::popup::PopupType::ConnectionDiagnostics {
                                    result: diagnostics,
                                },
                            )
                            .build(),
                        );
                    }
                    Err(e) => {
                        self.toasts
                            .push(Toast::error(format!("Diagnostics failed: {}", e)));
                    }
                }
            }
            Action::DismissOnboardingItem => {
                if let Some(milestone) = self.onboarding_checklist.incomplete_milestones().first() {
                    self.onboarding_checklist.dismiss_item(milestone);
                }
            }
            Action::DismissOnboardingAll => {
                self.onboarding_checklist.dismiss_all();
            }
            _ => {}
        }
    }

    fn trigger_load_cluster_info(&mut self) {
        // This is handled by the main loop, just set a flag or send action
        // For now, we'll refresh on next tick or user action
    }

    fn trigger_load_cluster_peers(&mut self) {
        // This is handled by the main loop, just set a flag or send action
        // For now, we'll refresh on next tick or user action
    }

    fn handle_copy_to_clipboard(&mut self, content: String) {
        match clipboard::copy_to_clipboard(content.clone()) {
            Ok(()) => {
                let preview = Self::clipboard_preview(&content);
                self.toasts.push(Toast::info(format!("Copied: {preview}")));
            }
            Err(e) => {
                self.toasts
                    .push(Toast::error(format!("Clipboard error: {e}")));
            }
        }
    }

    fn enter_search_mode(&mut self) {
        self.is_filtering = true;
        // Save current filter for potential cancel
        self.filter_before_edit = self.search_filter.clone();
        // Pre-populate filter_input with existing filter for editing
        self.filter_input =
            SingleLineInput::with_value(self.search_filter.clone().unwrap_or_default());
    }

    fn cycle_theme(&mut self) {
        self.color_theme = self.color_theme.cycle_next();
        self.theme = splunk_config::Theme::from(self.color_theme);
        self.toasts
            .push(Toast::info(format!("Theme: {}", self.color_theme)));
    }

    fn handle_spl_validation_result(
        &mut self,
        valid: bool,
        errors: Vec<String>,
        warnings: Vec<String>,
        request_id: u64,
    ) {
        // Ignore stale results - only apply if this matches the latest request
        if request_id != self.validation_request_id {
            tracing::debug!(
                "Ignoring stale SPL validation result (got {}, current {})",
                request_id,
                self.validation_request_id
            );
            return;
        }

        self.spl_validation_state = crate::app::SplValidationState {
            valid: Some(valid),
            errors,
            warnings,
            last_validated: Some(std::time::Instant::now()),
            request_id,
        };
        self.spl_validation_pending = false;
    }

    fn show_error_details(&mut self, details: crate::error_details::ErrorDetails) {
        use crate::ui::popup::{Popup, PopupType};
        self.error_scroll_offset = 0; // Reset scroll on open

        // Route auth errors to AuthRecovery popup, others to ErrorDetails
        if let Some(ref auth_recovery) = details.auth_recovery {
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
        } else {
            self.popup = Some(Popup::builder(PopupType::ErrorDetails).build());
        }

        self.current_error = Some(details);
    }

    fn show_error_details_from_current(&mut self) {
        use crate::ui::popup::{Popup, PopupType};
        if let Some(ref details) = self.current_error {
            self.error_scroll_offset = 0; // Reset scroll on open

            // Route auth errors to AuthRecovery popup, others to ErrorDetails
            if let Some(ref auth_recovery) = details.auth_recovery {
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
            } else {
                self.popup = Some(Popup::builder(PopupType::ErrorDetails).build());
            }
        }
    }

    fn open_create_index_dialog(&mut self) {
        use crate::ui::popup::{Popup, PopupType};
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

    fn open_modify_index_dialog(&mut self, name: String) {
        use crate::ui::popup::{Popup, PopupType};
        if let Some(indexes) = &self.indexes {
            if let Some(index) = indexes.iter().find(|i| i.name == name) {
                let current_max_hot_buckets = index
                    .max_hot_buckets
                    .as_ref()
                    .and_then(|s| Self::parse_max_hot_buckets(s, &index.name));
                self.popup = Some(
                    Popup::builder(PopupType::ModifyIndex {
                        index_name: index.name.clone(),
                        current_max_data_size_mb: index.max_total_data_size_mb,
                        current_max_hot_buckets,
                        current_max_warm_db_count: index.max_warm_db_count,
                        current_frozen_time_period_secs: index.frozen_time_period_in_secs,
                        current_home_path: index.home_path.clone(),
                        current_cold_db_path: index.cold_db_path.clone(),
                        current_thawed_path: index.thawed_path.clone(),
                        current_cold_to_frozen_dir: index.cold_to_frozen_dir.clone(),
                        new_max_data_size_mb: index.max_total_data_size_mb,
                        new_max_hot_buckets: current_max_hot_buckets,
                        new_max_warm_db_count: index.max_warm_db_count,
                        new_frozen_time_period_secs: index.frozen_time_period_in_secs,
                        new_home_path: index.home_path.clone(),
                        new_cold_db_path: index.cold_db_path.clone(),
                        new_thawed_path: index.thawed_path.clone(),
                        new_cold_to_frozen_dir: index.cold_to_frozen_dir.clone(),
                    })
                    .build(),
                );
                return;
            }
        }
        self.toasts.push(Toast::info("Index not found"));
    }

    fn open_create_user_dialog(&mut self) {
        use crate::ui::popup::{Popup, PopupType};
        self.popup = Some(
            Popup::builder(PopupType::CreateUser {
                name_input: String::new(),
                password_input: String::new(),
                roles_input: String::new(),
                realname_input: String::new(),
                email_input: String::new(),
                default_app_input: String::new(),
            })
            .build(),
        );
    }

    fn open_modify_user_dialog(&mut self, name: String) {
        use crate::ui::popup::{Popup, PopupType};
        if let Some(users) = &self.users {
            if let Some(user) = users.iter().find(|u| u.name == name) {
                let current_roles = user.roles.clone();
                let current_realname = user.realname.clone();
                let current_email = user.email.clone();
                let current_default_app = user.default_app.clone();
                self.popup = Some(
                    Popup::builder(PopupType::ModifyUser {
                        user_name: user.name.clone(),
                        current_roles: current_roles.clone(),
                        current_realname: current_realname.clone(),
                        current_email: current_email.clone(),
                        current_default_app: current_default_app.clone(),
                        password_input: String::new(),
                        roles_input: current_roles.join(","),
                        realname_input: current_realname.unwrap_or_default(),
                        email_input: current_email.unwrap_or_default(),
                        default_app_input: current_default_app.unwrap_or_default(),
                    })
                    .build(),
                );
                return;
            }
        }
        self.toasts.push(Toast::info("User not found"));
    }

    fn open_create_role_dialog(&mut self) {
        use crate::ui::popup::{Popup, PopupType};
        self.popup = Some(
            Popup::builder(PopupType::CreateRole {
                name_input: String::new(),
                capabilities_input: String::new(),
                search_indexes_input: String::new(),
                search_filter_input: String::new(),
                imported_roles_input: String::new(),
                default_app_input: String::new(),
            })
            .build(),
        );
    }

    fn open_modify_role_dialog(&mut self, name: String) {
        use crate::ui::popup::{Popup, PopupType};
        if let Some(roles) = &self.roles {
            if let Some(role) = roles.iter().find(|r| r.name == name) {
                let current_capabilities = role.capabilities.clone();
                let current_search_indexes = role.search_indexes.clone();
                let current_search_filter = role.search_filter.clone();
                let current_imported_roles = role.imported_roles.clone();
                let current_default_app = role.default_app.clone();
                self.popup = Some(
                    Popup::builder(PopupType::ModifyRole {
                        role_name: role.name.clone(),
                        current_capabilities: current_capabilities.clone(),
                        current_search_indexes: current_search_indexes.clone(),
                        current_search_filter: current_search_filter.clone(),
                        current_imported_roles: current_imported_roles.clone(),
                        current_default_app: current_default_app.clone(),
                        capabilities_input: current_capabilities.join(","),
                        search_indexes_input: current_search_indexes.join(","),
                        search_filter_input: current_search_filter.unwrap_or_default(),
                        imported_roles_input: current_imported_roles.join(","),
                        default_app_input: current_default_app.unwrap_or_default(),
                    })
                    .build(),
                );
                return;
            }
        }
        self.toasts.push(Toast::info("Role not found"));
    }

    fn open_create_macro_dialog(&mut self) {
        use crate::ui::popup::{MacroField, Popup, PopupType};
        self.popup = Some(
            Popup::builder(PopupType::CreateMacro {
                name_input: String::new(),
                definition_input: String::new(),
                args_input: String::new(),
                description_input: String::new(),
                disabled: false,
                iseval: false,
                selected_field: MacroField::Name,
            })
            .build(),
        );
    }

    fn open_edit_saved_search_dialog(&mut self) {
        use crate::ui::popup::{Popup, PopupType, SavedSearchField};
        if let Some(searches) = &self.saved_searches
            && let Some(selected) = self.saved_searches_state.selected()
            && let Some(search) = searches.get(selected)
        {
            self.popup = Some(
                Popup::builder(PopupType::EditSavedSearch {
                    search_name: search.name.clone(),
                    search_input: String::new(),
                    description_input: String::new(),
                    disabled: search.disabled,
                    selected_field: SavedSearchField::Search,
                })
                .build(),
            );
        } else {
            self.toasts.push(Toast::info("No saved search selected"));
        }
    }

    fn open_edit_macro_dialog(&mut self) {
        use crate::ui::popup::{MacroField, Popup, PopupType};
        if let Some(macros) = &self.macros
            && let Some(selected) = self.macros_state.selected()
            && let Some(macro_item) = macros.get(selected)
        {
            self.popup = Some(
                Popup::builder(PopupType::EditMacro {
                    macro_name: macro_item.name.clone(),
                    definition_input: String::new(),
                    args_input: String::new(),
                    description_input: String::new(),
                    disabled: macro_item.disabled,
                    iseval: macro_item.iseval,
                    selected_field: MacroField::Definition,
                })
                .build(),
            );
        } else {
            self.toasts.push(Toast::info("No macro selected"));
        }
    }

    /// Clamp all scroll offsets to ensure they don't exceed available data.
    /// Called after terminal resize to prevent out-of-bounds scrolling.
    pub fn clamp_scroll_offsets(&mut self) {
        // Clamp search results scroll offset
        let max_search_offset = self.search_results.len().saturating_sub(1);
        self.search_scroll_offset = self.search_scroll_offset.min(max_search_offset);

        // Clamp jobs selection to visible items
        if let Some(selected) = self.jobs_state.selected() {
            let max = self.filtered_jobs_len().saturating_sub(1);
            if selected > max {
                self.jobs_state.select(Some(max));
            }
        }

        // Clamp indexes selection
        if let Some(ref indexes) = self.indexes
            && let Some(selected) = self.indexes_state.selected()
        {
            let max = indexes.len().saturating_sub(1);
            if selected > max {
                self.indexes_state.select(Some(max));
            }
        }

        // Clamp saved searches selection
        if let Some(ref searches) = self.saved_searches
            && let Some(selected) = self.saved_searches_state.selected()
        {
            let max = searches.len().saturating_sub(1);
            if selected > max {
                self.saved_searches_state.select(Some(max));
            }
        }

        // Clamp apps selection
        if let Some(ref apps) = self.apps
            && let Some(selected) = self.apps_state.selected()
        {
            let max = apps.len().saturating_sub(1);
            if selected > max {
                self.apps_state.select(Some(max));
            }
        }

        // Clamp users selection
        if let Some(ref users) = self.users
            && let Some(selected) = self.users_state.selected()
        {
            let max = users.len().saturating_sub(1);
            if selected > max {
                self.users_state.select(Some(max));
            }
        }

        // Clamp internal logs selection
        if let Some(ref logs) = self.internal_logs
            && let Some(selected) = self.internal_logs_state.selected()
        {
            let max = logs.len().saturating_sub(1);
            if selected > max {
                self.internal_logs_state.select(Some(max));
            }
        }

        // Clamp cluster peers selection
        if let Some(ref peers) = self.cluster_peers
            && let Some(selected) = self.cluster_peers_state.selected()
        {
            let max = peers.len().saturating_sub(1);
            if selected > max {
                self.cluster_peers_state.select(Some(max));
            }
        }

        // Clamp macros selection
        if let Some(ref macros) = self.macros
            && let Some(selected) = self.macros_state.selected()
        {
            let max = macros.len().saturating_sub(1);
            if selected > max {
                self.macros_state.select(Some(max));
            }
        }

        // Clamp search peers selection
        if let Some(ref peers) = self.search_peers
            && let Some(selected) = self.search_peers_state.selected()
        {
            let max = peers.len().saturating_sub(1);
            if selected > max {
                self.search_peers_state.select(Some(max));
            }
        }

        // Clamp inputs selection
        if let Some(ref inputs) = self.inputs
            && let Some(selected) = self.inputs_state.selected()
        {
            let max = inputs.len().saturating_sub(1);
            if selected > max {
                self.inputs_state.select(Some(max));
            }
        }

        // Clamp fired alerts selection
        if let Some(ref alerts) = self.fired_alerts
            && let Some(selected) = self.fired_alerts_state.selected()
        {
            let max = alerts.len().saturating_sub(1);
            if selected > max {
                self.fired_alerts_state.select(Some(max));
            }
        }

        // Clamp forwarders selection
        if let Some(ref forwarders) = self.forwarders
            && let Some(selected) = self.forwarders_state.selected()
        {
            let max = forwarders.len().saturating_sub(1);
            if selected > max {
                self.forwarders_state.select(Some(max));
            }
        }

        // Clamp lookups selection
        if let Some(ref lookups) = self.lookups
            && let Some(selected) = self.lookups_state.selected()
        {
            let max = lookups.len().saturating_sub(1);
            if selected > max {
                self.lookups_state.select(Some(max));
            }
        }

        // Clamp dashboards selection
        if let Some(ref dashboards) = self.dashboards
            && let Some(selected) = self.dashboards_state.selected()
        {
            let max = dashboards.len().saturating_sub(1);
            if selected > max {
                self.dashboards_state.select(Some(max));
            }
        }

        // Clamp data models selection
        if let Some(ref data_models) = self.data_models
            && let Some(selected) = self.data_models_state.selected()
        {
            let max = data_models.len().saturating_sub(1);
            if selected > max {
                self.data_models_state.select(Some(max));
            }
        }

        // Clamp workload pools selection
        if let Some(ref pools) = self.workload_pools
            && let Some(selected) = self.workload_pools_state.selected()
        {
            let max = pools.len().saturating_sub(1);
            if selected > max {
                self.workload_pools_state.select(Some(max));
            }
        }

        // Clamp workload rules selection
        if let Some(ref rules) = self.workload_rules
            && let Some(selected) = self.workload_rules_state.selected()
        {
            let max = rules.len().saturating_sub(1);
            if selected > max {
                self.workload_rules_state.select(Some(max));
            }
        }

        // Clamp SHC members selection
        if let Some(ref members) = self.shc_members
            && let Some(selected) = self.shc_members_state.selected()
        {
            let max = members.len().saturating_sub(1);
            if selected > max {
                self.shc_members_state.select(Some(max));
            }
        }

        // Clamp config files selection
        if let Some(ref files) = self.config_files
            && let Some(selected) = self.config_files_state.selected()
        {
            let max = files.len().saturating_sub(1);
            if selected > max {
                self.config_files_state.select(Some(max));
            }
        }

        // Clamp config stanzas selection
        if let Some(ref stanzas) = self.config_stanzas
            && let Some(selected) = self.config_stanzas_state.selected()
        {
            let max = stanzas.len().saturating_sub(1);
            if selected > max {
                self.config_stanzas_state.select(Some(max));
            }
        }

        // Clamp audit events selection
        if let Some(ref events) = self.audit_events
            && let Some(selected) = self.audit_state.selected()
        {
            let max = events.len().saturating_sub(1);
            if selected > max {
                self.audit_state.select(Some(max));
            }
        }

        // Clamp roles selection
        if let Some(ref roles) = self.roles
            && let Some(selected) = self.roles_state.selected()
        {
            let max = roles.len().saturating_sub(1);
            if selected > max {
                self.roles_state.select(Some(max));
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;

    #[test]
    fn test_loading_sets_progress_to_zero() {
        let mut app = App::new(None, ConnectionContext::default());
        app.progress = 0.5;

        app.handle_system_action(Action::Loading(true));

        assert!(app.loading);
        assert_eq!(app.progress, 0.0);
    }

    #[test]
    fn test_progress_updates_value() {
        let mut app = App::new(None, ConnectionContext::default());

        app.handle_system_action(Action::Progress(0.75));

        assert_eq!(app.progress, 0.75);
    }

    #[test]
    fn test_notify_adds_toast() {
        let mut app = App::new(None, ConnectionContext::default());

        app.handle_system_action(Action::Notify(
            crate::ui::ToastLevel::Info,
            "Test message".to_string(),
        ));

        assert_eq!(app.toasts.len(), 1);
        assert_eq!(app.toasts[0].message, "Test message");
    }

    #[test]
    fn test_tick_prunes_expired_toasts() {
        let mut app = App::new(None, ConnectionContext::default());
        // Add a toast that's already expired (using created_at field)
        let mut expired_toast = Toast::info("Expired");
        expired_toast.created_at = std::time::Instant::now() - std::time::Duration::from_secs(100);
        app.toasts.push(expired_toast);

        // Add a fresh toast
        app.toasts.push(Toast::info("Fresh"));

        app.handle_system_action(Action::Tick);

        // Only the fresh toast should remain
        assert_eq!(app.toasts.len(), 1);
        assert_eq!(app.toasts[0].message, "Fresh");
    }

    #[test]
    fn test_enter_search_mode_saves_current_filter() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_filter = Some("existing filter".to_string());

        app.handle_system_action(Action::EnterSearchMode);

        assert!(app.is_filtering);
        assert_eq!(app.filter_before_edit, Some("existing filter".to_string()));
        assert_eq!(app.filter_input.value(), "existing filter");
    }

    #[test]
    fn test_enter_search_mode_with_no_filter() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_filter = None;

        app.handle_system_action(Action::EnterSearchMode);

        assert!(app.is_filtering);
        assert!(app.filter_before_edit.is_none());
        assert!(app.filter_input.is_empty());
    }

    #[test]
    fn test_search_input_appends_character() {
        let mut app = App::new(None, ConnectionContext::default());
        app.filter_input.set_value("hel");

        app.handle_system_action(Action::SearchInput('l'));
        app.handle_system_action(Action::SearchInput('o'));

        assert_eq!(app.filter_input.value(), "hello");
    }

    #[test]
    fn test_clear_search_clears_filter() {
        let mut app = App::new(None, ConnectionContext::default());
        app.search_filter = Some("test".to_string());

        app.handle_system_action(Action::ClearSearch);

        assert!(app.search_filter.is_none());
    }

    #[test]
    fn test_cycle_theme_changes_theme() {
        let mut app = App::new(None, ConnectionContext::default());
        let initial_theme = app.color_theme;

        app.handle_system_action(Action::CycleTheme);

        assert_ne!(app.color_theme, initial_theme);
        assert_eq!(app.toasts.len(), 1);
        assert!(app.toasts[0].message.contains("Theme:"));
    }

    #[test]
    fn test_spl_validation_result_updates_state() {
        let mut app = App::new(None, ConnectionContext::default());

        app.handle_system_action(Action::SplValidationResult {
            valid: true,
            errors: vec![],
            warnings: vec!["Warning 1".to_string()],
            request_id: 0,
        });

        assert_eq!(app.spl_validation_state.valid, Some(true));
        assert!(
            app.spl_validation_state
                .warnings
                .contains(&"Warning 1".to_string())
        );
        assert!(!app.spl_validation_pending);
    }

    #[test]
    fn test_show_error_details_from_current_with_no_error() {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_error = None;

        app.handle_system_action(Action::ShowErrorDetailsFromCurrent);

        assert!(app.popup.is_none());
    }

    #[test]
    fn test_clear_error_details_clears_state() {
        let mut app = App::new(None, ConnectionContext::default());
        use crate::ui::popup::{Popup, PopupType};
        app.current_error = Some(crate::error_details::ErrorDetails::from_error_string(
            "Error",
        ));
        app.popup = Some(Popup::builder(PopupType::ErrorDetails).build());

        app.handle_system_action(Action::ClearErrorDetails);

        assert!(app.current_error.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_job_operation_complete_clears_selection() {
        let mut app = App::new(None, ConnectionContext::default());
        app.selected_jobs.insert("job1".to_string());
        app.selected_jobs.insert("job2".to_string());

        app.handle_system_action(Action::JobOperationComplete("Jobs finalized".to_string()));

        assert!(app.selected_jobs.is_empty());
        assert_eq!(app.search_status, "Jobs finalized");
        assert!(!app.loading);
    }

    #[test]
    fn test_resize_updates_last_area() {
        let mut app = App::new(None, ConnectionContext::default());

        app.handle_system_action(Action::Resize(100, 50));

        assert_eq!(app.last_area.width, 100);
        assert_eq!(app.last_area.height, 50);
    }

    #[test]
    fn test_open_create_macro_dialog() {
        let mut app = App::new(None, ConnectionContext::default());

        app.handle_system_action(Action::OpenCreateMacroDialog);

        assert!(app.popup.is_some());
        assert!(matches!(
            app.popup,
            Some(crate::ui::popup::Popup {
                kind: crate::ui::popup::PopupType::CreateMacro { .. },
                ..
            })
        ));
    }

    #[test]
    fn test_edit_macro_action_opens_popup() {
        use crate::ui::popup::{MacroField, PopupType};
        use splunk_client::models::Macro;

        let mut app = App::new(None, ConnectionContext::default());

        // Set up test macro data
        app.macros = Some(vec![Macro {
            name: "test_macro".to_string(),
            definition: "index=main".to_string(),
            args: Some("arg1,arg2".to_string()),
            description: Some("Test description".to_string()),
            disabled: false,
            iseval: true,
            validation: None,
            errormsg: None,
        }]);
        app.macros_state.select(Some(0));

        // Trigger edit action
        app.handle_system_action(Action::EditMacro);

        assert!(app.popup.is_some());
        assert!(matches!(
            app.popup,
            Some(crate::ui::popup::Popup {
                kind: PopupType::EditMacro {
                    macro_name,
                    disabled: false,
                    iseval: true,
                    selected_field: MacroField::Definition,
                    ..
                },
                ..
            }) if macro_name == "test_macro"
        ));
    }

    #[test]
    fn test_edit_macro_action_no_selection() {
        let mut app = App::new(None, ConnectionContext::default());

        // No macros loaded
        app.macros = None;

        // Trigger edit action
        app.handle_system_action(Action::EditMacro);

        // Should show toast and not open popup
        assert!(app.popup.is_none());
        assert_eq!(app.toasts.len(), 1);
        assert_eq!(app.toasts[0].message, "No macro selected");
    }

    #[test]
    fn test_edit_macro_action_no_macro_selected() {
        use splunk_client::models::Macro;

        let mut app = App::new(None, ConnectionContext::default());

        // Set up test macro data but no selection
        app.macros = Some(vec![Macro {
            name: "test_macro".to_string(),
            definition: "index=main".to_string(),
            args: Some("arg1,arg2".to_string()),
            description: Some("Test description".to_string()),
            disabled: false,
            iseval: true,
            validation: None,
            errormsg: None,
        }]);
        // No selection made
        app.macros_state.select(None);

        // Trigger edit action
        app.handle_system_action(Action::EditMacro);

        // Should show toast and not open popup
        assert!(app.popup.is_none());
        assert_eq!(app.toasts.len(), 1);
        assert_eq!(app.toasts[0].message, "No macro selected");
    }
}
