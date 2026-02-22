//! System action handlers for the TUI app.
//!
//! Purpose: Apply non-domain-specific UI/system actions to `App` state.
//! Responsibilities: Manage loading/progress, toasts, resize, filter mode, theme, and system popups.
//! Non-scope: Does not dispatch API calls or own resource-loading workflows.
//! Invariants/Assumptions: System actions keep focus/selection state clamped to current data bounds.

use crate::action::Action;
use crate::app::App;
use crate::app::clipboard;
use crate::app::input::components::SingleLineInput;
use crate::ui::{Toast, ToastLevel};
use ratatui::widgets::{ListState, TableState};

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
            // SHC management result actions
            Action::ShcMemberAdded { result } => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts.push(Toast::success("SHC member added"));
                    }
                    Err(e) => {
                        self.toasts
                            .push(Toast::error(format!("Failed to add SHC member: {}", e)));
                    }
                }
            }
            Action::ShcMemberRemoved { result } => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts.push(Toast::success("SHC member removed"));
                    }
                    Err(e) => {
                        self.toasts
                            .push(Toast::error(format!("Failed to remove SHC member: {}", e)));
                    }
                }
            }
            Action::ShcRollingRestarted { result } => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts
                            .push(Toast::info("SHC rolling restart initiated"));
                    }
                    Err(e) => {
                        self.toasts.push(Toast::error(format!(
                            "Failed to initiate SHC rolling restart: {}",
                            e
                        )));
                    }
                }
            }
            Action::ShcCaptainSet { result } => {
                self.loading = false;
                match result {
                    Ok(()) => {
                        self.toasts.push(Toast::success("SHC captain set"));
                    }
                    Err(e) => {
                        self.toasts
                            .push(Toast::error(format!("Failed to set SHC captain: {}", e)));
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
                let message = format!("Clipboard error: {e}");
                self.push_error_toast_once(message);
            }
        }
    }

    fn push_error_toast_once(&mut self, message: String) {
        let duplicate_active = self
            .toasts
            .iter()
            .any(|t| !t.is_expired() && t.level == ToastLevel::Error && t.message == message);
        if duplicate_active {
            return;
        }
        self.toasts.push(Toast::error(message));
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
    fn clamp_list_selection(state: &mut ListState, len: usize) {
        if let Some(selected) = state.selected() {
            let max = len.saturating_sub(1);
            if selected > max {
                state.select(Some(max));
            }
        }
    }

    fn clamp_table_selection(state: &mut TableState, len: usize) {
        if let Some(selected) = state.selected() {
            let max = len.saturating_sub(1);
            if selected > max {
                state.select(Some(max));
            }
        }
    }

    pub fn clamp_scroll_offsets(&mut self) {
        // Clamp search results scroll offset
        let max_search_offset = self.search_results.len().saturating_sub(1);
        self.search_scroll_offset = self.search_scroll_offset.min(max_search_offset);

        let filtered_jobs_len = self.filtered_jobs_len();
        Self::clamp_table_selection(&mut self.jobs_state, filtered_jobs_len);

        if let Some(items) = self.indexes.as_ref() {
            Self::clamp_list_selection(&mut self.indexes_state, items.len());
        }
        if let Some(items) = self.saved_searches.as_ref() {
            Self::clamp_list_selection(&mut self.saved_searches_state, items.len());
        }
        if let Some(items) = self.apps.as_ref() {
            Self::clamp_list_selection(&mut self.apps_state, items.len());
        }
        if let Some(items) = self.users.as_ref() {
            Self::clamp_list_selection(&mut self.users_state, items.len());
        }
        if let Some(items) = self.macros.as_ref() {
            Self::clamp_list_selection(&mut self.macros_state, items.len());
        }
        if let Some(items) = self.fired_alerts.as_ref() {
            Self::clamp_list_selection(&mut self.fired_alerts_state, items.len());
        }
        if let Some(items) = self.dashboards.as_ref() {
            Self::clamp_list_selection(&mut self.dashboards_state, items.len());
        }
        if let Some(items) = self.data_models.as_ref() {
            Self::clamp_list_selection(&mut self.data_models_state, items.len());
        }
        if let Some(items) = self.roles.as_ref() {
            Self::clamp_list_selection(&mut self.roles_state, items.len());
        }

        if let Some(items) = self.internal_logs.as_ref() {
            Self::clamp_table_selection(&mut self.internal_logs_state, items.len());
        }
        if let Some(items) = self.cluster_peers.as_ref() {
            Self::clamp_table_selection(&mut self.cluster_peers_state, items.len());
        }
        if let Some(items) = self.search_peers.as_ref() {
            Self::clamp_table_selection(&mut self.search_peers_state, items.len());
        }
        if let Some(items) = self.inputs.as_ref() {
            Self::clamp_table_selection(&mut self.inputs_state, items.len());
        }
        if let Some(items) = self.forwarders.as_ref() {
            Self::clamp_table_selection(&mut self.forwarders_state, items.len());
        }
        if let Some(items) = self.lookups.as_ref() {
            Self::clamp_table_selection(&mut self.lookups_state, items.len());
        }
        if let Some(items) = self.workload_pools.as_ref() {
            Self::clamp_table_selection(&mut self.workload_pools_state, items.len());
        }
        if let Some(items) = self.workload_rules.as_ref() {
            Self::clamp_table_selection(&mut self.workload_rules_state, items.len());
        }
        if let Some(items) = self.shc_members.as_ref() {
            Self::clamp_table_selection(&mut self.shc_members_state, items.len());
        }
        if let Some(items) = self.config_files.as_ref() {
            Self::clamp_table_selection(&mut self.config_files_state, items.len());
        }
        if let Some(items) = self.config_stanzas.as_ref() {
            Self::clamp_table_selection(&mut self.config_stanzas_state, items.len());
        }
        if let Some(items) = self.audit_events.as_ref() {
            Self::clamp_table_selection(&mut self.audit_state, items.len());
        }
    }
}

#[cfg(test)]
#[path = "system_tests.rs"]
mod tests;
