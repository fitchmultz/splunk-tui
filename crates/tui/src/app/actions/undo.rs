//! Undo/Redo action handlers for the TUI app.
//!
//! This module implements the handling of undo-related actions including
//! queuing operations, undoing, redoing, and executing pending operations
//! after the grace period expires.

use crate::action::Action;
use crate::app::App;
use crate::ui::Toast;
use crate::undo::UndoableOperation;

impl App {
    /// Handle undo-related actions.
    pub fn handle_undo_action(&mut self, action: Action) {
        match action {
            Action::QueueUndoableOperation {
                operation,
                description,
            } => {
                let id = self.undo_buffer.push(operation, description.clone());
                self.undo_toast_id = Some(id);
                self.toasts.push(crate::ui::toast::Toast::undo_countdown(
                    format!("Queued: {}", description),
                    30,
                ));
            }
            Action::Undo => {
                if let Some(entry) = self.undo_buffer.undo() {
                    self.toasts
                        .push(Toast::info(format!("Undone: {}", entry.description)));
                    // Clear the undo toast if this was the tracked operation
                    if self.undo_toast_id == Some(entry.id) {
                        self.undo_toast_id = None;
                    }
                    // Handle specific undo logic based on operation type
                    // Pass whether operation was already executed (true) or just pending (false)
                    self.perform_undo(entry.operation, entry.executed);
                } else {
                    self.toasts.push(Toast::info("Nothing to undo"));
                }
            }
            Action::Redo => {
                if let Some(entry) = self.undo_buffer.redo() {
                    self.toasts
                        .push(Toast::info(format!("Redone: {}", entry.description)));
                    // Update the undo toast tracking to the redone entry
                    self.undo_toast_id = Some(entry.id);
                } else {
                    self.toasts.push(Toast::info("Nothing to redo"));
                }
            }
            Action::ExecutePendingOperation { id } => {
                // Find and execute the pending operation
                if let Some(entry) = self.undo_buffer.peek_pending() {
                    if entry.id == id {
                        let operation = entry.operation.clone();
                        self.execute_undoable_operation(operation);
                        self.undo_buffer.mark_executed(id);
                    }
                }
            }
            Action::OperationUndone { description } => {
                self.toasts.push(Toast::success(format!(
                    "Successfully undone: {}",
                    description
                )));
            }
            Action::OperationRedone { description } => {
                self.toasts.push(Toast::success(format!(
                    "Successfully redone: {}",
                    description
                )));
            }
            Action::ShowUndoHistory => {
                // Open undo history popup
                self.popup = Some(
                    crate::ui::popup::Popup::builder(crate::ui::popup::PopupType::UndoHistory {
                        scroll_offset: 0,
                    })
                    .build(),
                );
            }
            _ => {}
        }
    }

    /// Execute the actual operation after grace period.
    fn execute_undoable_operation(&mut self, operation: UndoableOperation) {
        match operation {
            UndoableOperation::DeleteIndex { name, .. } => {
                // Dispatch to actual delete action
                self.update(Action::DeleteIndex { name });
            }
            UndoableOperation::DeleteJob { sid } => {
                self.update(Action::DeleteJob(sid));
            }
            UndoableOperation::CancelJob { sid, .. } => {
                self.update(Action::CancelJob(sid));
            }
            UndoableOperation::DeleteSavedSearch { name, .. } => {
                self.update(Action::DeleteSavedSearch { name });
            }
            UndoableOperation::DeleteLookup { name, app, owner } => {
                self.update(Action::DeleteLookup { name, app, owner });
            }
            UndoableOperation::DeleteUser { name, .. } => {
                self.update(Action::DeleteUser { name });
            }
            UndoableOperation::DeleteRole { name } => {
                self.update(Action::DeleteRole { name });
            }
            UndoableOperation::RemoveApp { app_name } => {
                self.update(Action::RemoveApp { app_name });
            }
            UndoableOperation::DeleteProfile { name, .. } => {
                self.update(Action::DeleteProfile { name });
            }
            UndoableOperation::ModifyIndex {
                name, new_params, ..
            } => {
                self.update(Action::ModifyIndex {
                    name,
                    params: splunk_client::ModifyIndexParams {
                        max_data_size_mb: new_params.max_data_size_mb,
                        max_hot_buckets: new_params.max_hot_buckets,
                        max_warm_db_count: new_params.max_warm_db_count,
                        frozen_time_period_in_secs: new_params.frozen_time_period_secs,
                        home_path: new_params.home_path,
                        cold_db_path: new_params.cold_db_path,
                        thawed_path: new_params.thawed_path,
                        cold_to_frozen_dir: new_params.cold_to_frozen_dir,
                    },
                });
            }
            UndoableOperation::DeleteJobsBatch { sids } => {
                self.update(Action::DeleteJobsBatch(sids));
            }
            UndoableOperation::CancelJobsBatch { sids } => {
                self.update(Action::CancelJobsBatch(sids));
            }
        }
    }

    /// Perform undo for an operation.
    ///
    /// # Arguments
    /// * `operation` - The operation to undo
    /// * `was_executed` - Whether the operation was already executed (true) or just pending (false)
    fn perform_undo(&mut self, operation: UndoableOperation, was_executed: bool) {
        match operation {
            UndoableOperation::DeleteIndex {
                name,
                original_settings,
            } => {
                if !was_executed {
                    // Operation was still pending, just cancelled it
                    self.toasts.push(Toast::info(format!(
                        "Index '{}' deletion cancelled (was not executed)",
                        name
                    )));
                } else if let Some(settings) = original_settings {
                    // Operation already executed, need to recreate
                    self.toasts
                        .push(Toast::info(format!("Restoring index '{}'...", name)));
                    self.update(Action::CreateIndex {
                        params: splunk_client::CreateIndexParams {
                            name: name.clone(),
                            max_data_size_mb: settings.max_data_size_mb,
                            max_hot_buckets: settings.max_hot_buckets,
                            max_warm_db_count: settings.max_warm_db_count,
                            frozen_time_period_in_secs: settings.frozen_time_period_secs,
                            home_path: settings.home_path,
                            cold_db_path: settings.cold_db_path,
                            thawed_path: settings.thawed_path,
                            cold_to_frozen_dir: settings.cold_to_frozen_dir,
                        },
                    });
                } else {
                    // Executed but no recovery data available
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore index '{}' - original settings not available",
                        name
                    )));
                }
            }
            UndoableOperation::DeleteJob { sid } => {
                if !was_executed {
                    // Operation was still pending, just cancelled it
                    self.toasts.push(Toast::info(format!(
                        "Job '{}' deletion cancelled (was not executed)",
                        sid
                    )));
                } else {
                    // Jobs cannot be restored once deleted
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore job '{}' - deletion is irreversible",
                        sid
                    )));
                }
            }
            UndoableOperation::CancelJob { sid, search_query } => {
                if !was_executed {
                    // Operation was still pending, just cancelled it
                    if let Some(query) = search_query {
                        self.toasts.push(Toast::info(format!(
                            "Job '{}' cancellation cancelled (was not executed). Search query: {}",
                            sid, query
                        )));
                    } else {
                        self.toasts.push(Toast::info(format!(
                            "Job '{}' cancellation cancelled (was not executed)",
                            sid
                        )));
                    }
                } else {
                    // Cancelled jobs can potentially be restarted
                    if let Some(query) = search_query {
                        self.toasts.push(Toast::info(format!(
                            "Cancelled job '{}' can be restarted with search: {}",
                            sid, query
                        )));
                    } else {
                        self.toasts.push(Toast::warning(format!(
                            "Cannot restore cancelled job '{}' - search query not available",
                            sid
                        )));
                    }
                }
            }
            UndoableOperation::DeleteSavedSearch { name, original } => {
                if !was_executed {
                    // Operation was still pending, just cancelled it
                    self.toasts.push(Toast::info(format!(
                        "Saved search '{}' deletion cancelled (was not executed)",
                        name
                    )));
                } else if let Some(data) = original {
                    // Operation already executed, need to recreate
                    self.toasts
                        .push(Toast::info(format!("Restoring saved search '{}'...", name)));
                    self.update(Action::CreateSavedSearch {
                        name: name.clone(),
                        search: data.search,
                        description: data.description,
                        disabled: data.disabled,
                    });
                } else {
                    // Executed but no recovery data available
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore saved search '{}' - original data not available",
                        name
                    )));
                }
            }
            UndoableOperation::DeleteLookup { name, .. } => {
                if !was_executed {
                    // Operation was still pending, just cancelled it
                    self.toasts.push(Toast::info(format!(
                        "Lookup '{}' deletion cancelled (was not executed)",
                        name
                    )));
                } else {
                    // Lookup file content is not captured, cannot restore
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore lookup '{}' - recovery not implemented",
                        name
                    )));
                }
            }
            UndoableOperation::DeleteUser { name, original } => {
                if !was_executed {
                    // Operation was still pending, just cancelled it
                    self.toasts.push(Toast::info(format!(
                        "User '{}' deletion cancelled (was not executed)",
                        name
                    )));
                } else {
                    // User recovery is IMPOSSIBLE - password not stored (intentional security design)
                    // Show explicit warning that this is irreversible
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore user '{}' - deletion is irreversible (password not stored)",
                        name
                    )));
                    if let Some(data) = original {
                        self.toasts.push(Toast::info(format!(
                            "To recreate user '{}', use roles: {:?}",
                            name, data.roles
                        )));
                    }
                }
            }
            UndoableOperation::DeleteRole { name } => {
                if !was_executed {
                    // Operation was still pending, just cancelled it
                    self.toasts.push(Toast::info(format!(
                        "Role '{}' deletion cancelled (was not executed)",
                        name
                    )));
                } else {
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore role '{}' - recovery not implemented",
                        name
                    )));
                }
            }
            UndoableOperation::RemoveApp { app_name } => {
                if !was_executed {
                    // Operation was still pending, just cancelled it
                    self.toasts.push(Toast::info(format!(
                        "App '{}' removal cancelled (was not executed)",
                        app_name
                    )));
                } else {
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore app '{}' - recovery not implemented",
                        app_name
                    )));
                }
            }
            UndoableOperation::DeleteProfile { name, original } => {
                if !was_executed {
                    // Operation was still pending, just cancelled it
                    self.toasts.push(Toast::info(format!(
                        "Profile '{}' deletion cancelled (was not executed)",
                        name
                    )));
                } else if let Some(data) = original {
                    // Operation already executed, need to recreate
                    self.toasts
                        .push(Toast::info(format!("Restoring profile '{}'...", name)));
                    self.update(Action::SaveProfile {
                        name: name.clone(),
                        profile: splunk_config::types::ProfileConfig {
                            base_url: Some(data.base_url),
                            username: Some(data.username),
                            password: None, // Passwords not stored in recovery data
                            api_token: None,
                            skip_verify: Some(data.skip_verify),
                            timeout_seconds: Some(data.timeout_seconds),
                            max_retries: Some(data.max_retries),
                            session_expiry_buffer_seconds: None,
                            session_ttl_seconds: None,
                            health_check_interval_seconds: None,
                        },
                        use_keyring: data.use_keyring,
                        original_name: None,
                        from_tutorial: false,
                    });
                } else {
                    // Executed but no recovery data available
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore profile '{}' - original data not available",
                        name
                    )));
                }
            }
            UndoableOperation::ModifyIndex {
                name,
                original_params,
                ..
            } => {
                // Restore original index settings
                self.toasts.push(Toast::info(format!(
                    "Restored original settings for index '{}'",
                    name
                )));
                self.update(Action::ModifyIndex {
                    name,
                    params: splunk_client::ModifyIndexParams {
                        max_data_size_mb: original_params.max_data_size_mb,
                        max_hot_buckets: original_params.max_hot_buckets,
                        max_warm_db_count: original_params.max_warm_db_count,
                        frozen_time_period_in_secs: original_params.frozen_time_period_secs,
                        home_path: original_params.home_path,
                        cold_db_path: original_params.cold_db_path,
                        thawed_path: original_params.thawed_path,
                        cold_to_frozen_dir: original_params.cold_to_frozen_dir,
                    },
                });
            }
            UndoableOperation::DeleteJobsBatch { sids } => {
                if !was_executed {
                    // Operation was still pending, just cancelled it
                    self.toasts.push(Toast::info(format!(
                        "Batch deletion of {} jobs cancelled (was not executed)",
                        sids.len()
                    )));
                } else {
                    // Batch jobs cannot be restored once deleted
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore {} deleted jobs - deletion is irreversible",
                        sids.len()
                    )));
                }
            }
            UndoableOperation::CancelJobsBatch { sids } => {
                if !was_executed {
                    // Operation was still pending, just cancelled it
                    self.toasts.push(Toast::info(format!(
                        "Batch cancellation of {} jobs cancelled (was not executed)",
                        sids.len()
                    )));
                } else {
                    // Cancelled batch jobs can potentially be restarted
                    self.toasts.push(Toast::info(format!(
                        "Cancelled {} jobs. To restart, resubmit the original searches",
                        sids.len()
                    )));
                }
            }
        }
    }

    /// Process the undo buffer: execute expired operations, update toasts.
    pub fn process_undo_buffer(&mut self) {
        let mut operations_to_execute: Vec<(uuid::Uuid, UndoableOperation, String)> = Vec::new();

        // Collect ALL expired entries (not just until first non-expired)
        // Iterating history directly to avoid blocking older expired entries
        // behind newer non-expired ones (push_front adds newest at front)
        for entry in self.undo_buffer.history().iter() {
            if !entry.executed && !entry.undone && entry.is_expired() {
                operations_to_execute.push((
                    entry.id,
                    entry.operation.clone(),
                    entry.description.clone(),
                ));
            }
        }

        // Execute and mark all expired operations
        for (id, operation, description) in operations_to_execute {
            self.execute_undoable_operation(operation);
            self.toasts.push(Toast::success(format!(
                "Operation executed: {}",
                description
            )));
            if self.undo_toast_id == Some(id) {
                self.undo_toast_id = None;
            }
            self.undo_buffer.mark_executed(id);
        }

        // Clear fully expired entries (executed + old)
        self.undo_buffer.clear_expired();

        // Update undo countdown toast using peek_pending (non-expired only for UI)
        if let Some(entry) = self.undo_buffer.peek_pending() {
            let remaining = entry.remaining_secs();
            // Find and update the undo toast using the tracked ID
            if let Some(toast_id) = self.undo_toast_id {
                if let Some(toast) = self.toasts.iter_mut().find(|t| t.id == toast_id) {
                    toast.update_undo_countdown(remaining);
                }
            }
        }
    }
}
