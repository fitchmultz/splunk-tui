//! Confirmation dialog popup handlers.
//!
//! Responsibilities:
//! - Handle confirmation dialogs for job/app/user/index operations
//! - Confirm or cancel destructive actions
//! - Queue destructive operations for undoable execution
//!
//! Does NOT handle:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT execute the actions directly (operations are queued with grace period)

use crate::action::Action;
use crate::app::App;
use crate::ui::popup::{Popup, PopupType};
use crate::undo::UndoableOperation;
use crossterm::event::{KeyCode, KeyEvent};

impl App {
    /// Handle confirmation dialog popups.
    ///
    /// Destructive operations are queued with the undo system instead of
    /// being executed immediately, giving users a grace period to undo.
    pub fn handle_confirm_popup(&mut self, key: KeyEvent) -> Option<Action> {
        match (self.popup.as_ref().map(|p| &p.kind), key.code) {
            // ConfirmCancel - not destructive, execute immediately
            (Some(PopupType::ConfirmCancel(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sid = if let Some(Popup {
                    kind: PopupType::ConfirmCancel(s),
                    ..
                }) = self.popup.take()
                {
                    s
                } else {
                    unreachable!()
                };
                Some(Action::CancelJob(sid))
            }
            // ConfirmDelete - queue as undoable operation
            (Some(PopupType::ConfirmDelete(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sid = if let Some(Popup {
                    kind: PopupType::ConfirmDelete(s),
                    ..
                }) = self.popup.take()
                {
                    s
                } else {
                    unreachable!()
                };
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteJob { sid: sid.clone() },
                    description: format!("Delete job '{}'", sid),
                })
            }
            // ConfirmCancelBatch - not destructive, execute immediately
            (Some(PopupType::ConfirmCancelBatch(sids)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sids = sids.clone();
                self.popup = None;
                Some(Action::CancelJobsBatch(sids))
            }
            // ConfirmDeleteBatch - queue as undoable operation
            (Some(PopupType::ConfirmDeleteBatch(sids)), KeyCode::Char('y') | KeyCode::Enter) => {
                let sids = sids.clone();
                self.popup = None;
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteJobsBatch { sids: sids.clone() },
                    description: format!("Delete {} job(s)", sids.len()),
                })
            }
            // ConfirmEnableApp - not destructive, execute immediately
            (Some(PopupType::ConfirmEnableApp(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let name = if let Some(Popup {
                    kind: PopupType::ConfirmEnableApp(n),
                    ..
                }) = self.popup.take()
                {
                    n
                } else {
                    unreachable!()
                };
                Some(Action::EnableApp(name))
            }
            // ConfirmDisableApp - not destructive, execute immediately
            (Some(PopupType::ConfirmDisableApp(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let name = if let Some(Popup {
                    kind: PopupType::ConfirmDisableApp(n),
                    ..
                }) = self.popup.take()
                {
                    n
                } else {
                    unreachable!()
                };
                Some(Action::DisableApp(name))
            }
            // ConfirmRemoveApp - queue as undoable operation
            (Some(PopupType::ConfirmRemoveApp(_)), KeyCode::Char('y') | KeyCode::Enter) => {
                let name = if let Some(Popup {
                    kind: PopupType::ConfirmRemoveApp(n),
                    ..
                }) = self.popup.take()
                {
                    n
                } else {
                    unreachable!()
                };
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::RemoveApp {
                        app_name: name.clone(),
                    },
                    description: format!("Remove app '{}'", name),
                })
            }
            // DeleteIndexConfirm - queue as undoable operation
            (
                Some(PopupType::DeleteIndexConfirm { index_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = index_name.clone();
                let description = format!("Delete index '{}'", index_name);
                self.popup = None;
                // Try to capture original settings if we have the index data loaded
                let original_settings = self.get_index_settings_for_undo(&name);
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteIndex {
                        name,
                        original_settings,
                    },
                    description,
                })
            }
            // DeleteUserConfirm - queue as undoable operation
            (
                Some(PopupType::DeleteUserConfirm { user_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = user_name.clone();
                let description = format!("Delete user '{}'", user_name);
                self.popup = None;
                // Try to capture original user data if available
                let original = self.get_user_recovery_data(&name);
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteUser { name, original },
                    description,
                })
            }
            // DeleteLookupConfirm - queue as undoable operation
            (
                Some(PopupType::DeleteLookupConfirm { lookup_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = lookup_name.clone();
                let description = format!("Delete lookup '{}'", lookup_name);
                self.popup = None;
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteLookup {
                        name,
                        app: None,
                        owner: None,
                    },
                    description,
                })
            }
            // DeleteRoleConfirm - queue as undoable operation
            (
                Some(PopupType::DeleteRoleConfirm { role_name }),
                KeyCode::Char('y') | KeyCode::Enter,
            ) => {
                let name = role_name.clone();
                let description = format!("Delete role '{}'", role_name);
                self.popup = None;
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteRole { name },
                    description,
                })
            }
            // Reject confirmations with 'n' or Esc
            (
                Some(
                    PopupType::ConfirmCancel(_)
                    | PopupType::ConfirmDelete(_)
                    | PopupType::ConfirmCancelBatch(_)
                    | PopupType::ConfirmDeleteBatch(_)
                    | PopupType::ConfirmEnableApp(_)
                    | PopupType::ConfirmDisableApp(_)
                    | PopupType::ConfirmRemoveApp(_),
                ),
                KeyCode::Char('n') | KeyCode::Esc,
            ) => {
                self.popup = None;
                None
            }
            (Some(PopupType::DeleteIndexConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (Some(PopupType::DeleteUserConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (Some(PopupType::DeleteLookupConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            (Some(PopupType::DeleteRoleConfirm { .. }), KeyCode::Char('n') | KeyCode::Esc) => {
                self.popup = None;
                None
            }
            _ => None,
        }
    }

    /// Get index settings for undo recovery.
    ///
    /// Attempts to find the index in the currently loaded indexes list
    /// and extract its settings for potential recovery.
    fn get_index_settings_for_undo(&self, name: &str) -> Option<crate::undo::IndexSettings> {
        self.indexes.as_ref().and_then(|indexes| {
            indexes
                .iter()
                .find(|idx| idx.name == name)
                .map(|idx| crate::undo::IndexSettings {
                    max_data_size_mb: idx.max_total_data_size_mb,
                    max_hot_buckets: idx
                        .max_hot_buckets
                        .as_ref()
                        .and_then(|s| Self::parse_max_hot_buckets(s, &idx.name)),
                    max_warm_db_count: idx.max_warm_db_count,
                    frozen_time_period_secs: idx.frozen_time_period_in_secs,
                    home_path: idx.home_path.clone(),
                    cold_db_path: idx.cold_db_path.clone(),
                    thawed_path: idx.thawed_path.clone(),
                    cold_to_frozen_dir: idx.cold_to_frozen_dir.clone(),
                })
        })
    }

    /// Get user recovery data for undo.
    ///
    /// Attempts to find the user in the currently loaded users list
    /// and extract their data for potential recovery.
    fn get_user_recovery_data(&self, name: &str) -> Option<crate::undo::UserRecoveryData> {
        self.users.as_ref().and_then(|users| {
            users
                .iter()
                .find(|u| u.name == name)
                .map(|u| crate::undo::UserRecoveryData {
                    roles: u.roles.clone(),
                    realname: u.realname.clone(),
                    email: u.email.clone(),
                    default_app: u.default_app.clone(),
                })
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::app::App;
    use crate::app::ConnectionContext;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn create_test_app() -> App {
        App::new(None, ConnectionContext::default())
    }

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::empty())
    }

    #[test]
    fn test_confirm_delete_job_queues_undoable() {
        let mut app = create_test_app();
        app.popup = Some(Popup::builder(PopupType::ConfirmDelete("test-sid".to_string())).build());

        let action = app.handle_confirm_popup(key(KeyCode::Char('y')));

        // Should queue as undoable operation, not direct delete
        assert!(
            matches!(
                &action,
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteJob { sid },
                    description,
                }) if sid == "test-sid" && description.contains("Delete job")
            ),
            "Expected QueueUndoableOperation for DeleteJob, got {:?}",
            action
        );
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_confirm_delete_index_queues_undoable() {
        let mut app = create_test_app();
        app.popup = Some(
            Popup::builder(PopupType::DeleteIndexConfirm {
                index_name: "test_idx".to_string(),
            })
            .build(),
        );

        let action = app.handle_confirm_popup(key(KeyCode::Char('y')));

        // Should queue as undoable operation
        assert!(
            matches!(
                &action,
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteIndex { name, .. },
                    description,
                }) if name == "test_idx" && description.contains("Delete index")
            ),
            "Expected QueueUndoableOperation for DeleteIndex, got {:?}",
            action
        );
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_confirm_remove_app_queues_undoable() {
        let mut app = create_test_app();
        app.popup =
            Some(Popup::builder(PopupType::ConfirmRemoveApp("test-app".to_string())).build());

        let action = app.handle_confirm_popup(key(KeyCode::Char('y')));

        // Should queue as undoable operation
        assert!(
            matches!(
                &action,
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::RemoveApp { app_name },
                    description,
                }) if app_name == "test-app" && description.contains("Remove app")
            ),
            "Expected QueueUndoableOperation for RemoveApp, got {:?}",
            action
        );
    }

    #[test]
    fn test_confirm_cancel_job_direct_action() {
        let mut app = create_test_app();
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());

        let action = app.handle_confirm_popup(key(KeyCode::Char('y')));

        // Cancel is not destructive, should be direct action
        assert!(
            matches!(&action, Some(Action::CancelJob(sid)) if sid == "test-sid"),
            "Expected direct CancelJob action, got {:?}",
            action
        );
    }

    #[test]
    fn test_confirm_enable_app_direct_action() {
        let mut app = create_test_app();
        app.popup =
            Some(Popup::builder(PopupType::ConfirmEnableApp("test-app".to_string())).build());

        let action = app.handle_confirm_popup(key(KeyCode::Char('y')));

        // Enable is not destructive, should be direct action
        assert!(
            matches!(&action, Some(Action::EnableApp(name)) if name == "test-app"),
            "Expected direct EnableApp action, got {:?}",
            action
        );
    }

    #[test]
    fn test_confirm_reject_closes_popup() {
        let mut app = create_test_app();
        app.popup = Some(Popup::builder(PopupType::ConfirmDelete("test-sid".to_string())).build());

        let action = app.handle_confirm_popup(key(KeyCode::Char('n')));

        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_confirm_esc_closes_popup() {
        let mut app = create_test_app();
        app.popup = Some(Popup::builder(PopupType::ConfirmDelete("test-sid".to_string())).build());

        let action = app.handle_confirm_popup(key(KeyCode::Esc));

        assert!(action.is_none());
        assert!(app.popup.is_none());
    }
}
