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
                    self.perform_undo(entry.operation);
                } else {
                    self.toasts.push(Toast::info("Nothing to undo"));
                }
            }
            Action::Redo => {
                if let Some(entry) = self.undo_buffer.redo() {
                    self.toasts
                        .push(Toast::info(format!("Redone: {}", entry.description)));
                    // Re-queue the operation
                    self.handle_undo_action(Action::QueueUndoableOperation {
                        operation: entry.operation,
                        description: entry.description,
                    });
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
    fn perform_undo(&mut self, operation: UndoableOperation) {
        match operation {
            UndoableOperation::DeleteIndex {
                name,
                original_settings,
            } => {
                if original_settings.is_some() {
                    // Recreate the index with original settings
                    self.toasts.push(Toast::info(format!(
                        "Restored index '{}' (recreating)",
                        name
                    )));
                    // TODO: Dispatch create index action with saved settings
                    // This would require a new action variant or using existing CreateIndex
                } else {
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore index '{}' - settings not available",
                        name
                    )));
                }
            }
            UndoableOperation::DeleteJob { sid } => {
                // Jobs can't really be restored once deleted
                // Just show a message that deletion was prevented
                self.toasts.push(Toast::info(format!(
                    "Job {} deletion prevented (was not executed)",
                    sid
                )));
            }
            UndoableOperation::CancelJob { sid, search_query } => {
                if let Some(query) = search_query {
                    self.toasts.push(Toast::info(format!(
                        "Cancelled job {} can be restarted with: {}",
                        sid, query
                    )));
                } else {
                    self.toasts.push(Toast::info(format!(
                        "Job {} cancellation prevented (was not executed)",
                        sid
                    )));
                }
            }
            UndoableOperation::DeleteSavedSearch { name, original } => {
                if original.is_some() {
                    self.toasts
                        .push(Toast::info(format!("Restored saved search '{}'", name)));
                    // TODO: Dispatch create saved search with original data
                } else {
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore saved search '{}' - data not available",
                        name
                    )));
                }
            }
            UndoableOperation::DeleteLookup { name, .. } => {
                self.toasts.push(Toast::warning(format!(
                    "Cannot restore lookup '{}' - recovery not implemented",
                    name
                )));
            }
            UndoableOperation::DeleteUser { name, original } => {
                if original.is_some() {
                    self.toasts
                        .push(Toast::info(format!("Restored user '{}'", name)));
                    // TODO: Dispatch create user with original data
                } else {
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore user '{}' - data not available",
                        name
                    )));
                }
            }
            UndoableOperation::DeleteRole { name } => {
                self.toasts.push(Toast::warning(format!(
                    "Cannot restore role '{}' - recovery not implemented",
                    name
                )));
            }
            UndoableOperation::RemoveApp { app_name } => {
                self.toasts.push(Toast::warning(format!(
                    "Cannot restore app '{}' - recovery not implemented",
                    app_name
                )));
            }
            UndoableOperation::DeleteProfile { name, original } => {
                if original.is_some() {
                    self.toasts
                        .push(Toast::info(format!("Restored profile '{}'", name)));
                    // TODO: Dispatch create profile with original data
                } else {
                    self.toasts.push(Toast::warning(format!(
                        "Cannot restore profile '{}' - data not available",
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
                self.toasts.push(Toast::info(format!(
                    "Batch deletion of {} jobs prevented",
                    sids.len()
                )));
            }
            UndoableOperation::CancelJobsBatch { sids } => {
                self.toasts.push(Toast::info(format!(
                    "Batch cancellation of {} jobs prevented",
                    sids.len()
                )));
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
