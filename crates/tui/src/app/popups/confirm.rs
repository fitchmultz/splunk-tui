//! Confirmation dialog popup handlers.
//!
//! Responsibilities:
//! - Handle confirmation dialogs for job/app/user/index operations
//! - Confirm or cancel destructive actions
//! - Queue destructive operations for undoable execution
//! - Provide shared action execution logic for keyboard and mouse handlers
//!
//! Does NOT handle:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT execute the actions directly (operations are queued with grace period)

use crate::action::Action;
use crate::app::App;
use crate::ui::popup::PopupType;
use crate::undo::UndoableOperation;
use crossterm::event::{KeyCode, KeyEvent};

impl PopupType {
    pub fn is_confirmation(&self) -> bool {
        matches!(
            self,
            PopupType::ConfirmCancel(_)
                | PopupType::ConfirmDelete(_)
                | PopupType::ConfirmCancelBatch(_)
                | PopupType::ConfirmDeleteBatch(_)
                | PopupType::ConfirmEnableApp(_)
                | PopupType::ConfirmDisableApp(_)
                | PopupType::ConfirmRemoveApp(_)
                | PopupType::DeleteIndexConfirm { .. }
                | PopupType::DeleteUserConfirm { .. }
                | PopupType::DeleteLookupConfirm { .. }
                | PopupType::DeleteRoleConfirm { .. }
                | PopupType::DeleteProfileConfirm { .. }
                | PopupType::DeleteSavedSearchConfirm { .. }
        )
    }
}

impl App {
    /// Execute the confirm action for a confirmation popup type.
    ///
    /// This is the single source of truth for mapping confirmation popups to actions.
    /// Used by both keyboard handling (this module) and mouse handling (mouse.rs).
    pub fn execute_confirmation_action(&mut self, popup_type: PopupType) -> Option<Action> {
        match popup_type {
            PopupType::ConfirmCancel(sid) => Some(Action::CancelJob(sid)),
            PopupType::ConfirmDelete(sid) => Some(Action::QueueUndoableOperation {
                operation: UndoableOperation::DeleteJob { sid: sid.clone() },
                description: format!("Delete job '{}'", sid),
            }),
            PopupType::ConfirmCancelBatch(sids) => Some(Action::CancelJobsBatch(sids)),
            PopupType::ConfirmDeleteBatch(sids) => Some(Action::QueueUndoableOperation {
                operation: UndoableOperation::DeleteJobsBatch { sids: sids.clone() },
                description: format!("Delete {} job(s)", sids.len()),
            }),
            PopupType::ConfirmEnableApp(name) => Some(Action::EnableApp(name)),
            PopupType::ConfirmDisableApp(name) => Some(Action::DisableApp(name)),
            PopupType::ConfirmRemoveApp(name) => Some(Action::QueueUndoableOperation {
                operation: UndoableOperation::RemoveApp {
                    app_name: name.clone(),
                },
                description: format!("Remove app '{}'", name),
            }),
            PopupType::DeleteIndexConfirm { index_name } => {
                let original_settings = self.get_index_settings_for_undo(&index_name);
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteIndex {
                        name: index_name.clone(),
                        original_settings,
                    },
                    description: format!("Delete index '{}'", index_name),
                })
            }
            PopupType::DeleteUserConfirm { user_name } => {
                let original = self.get_user_recovery_data(&user_name);
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteUser {
                        name: user_name.clone(),
                        original,
                    },
                    description: format!("Delete user '{}'", user_name),
                })
            }
            PopupType::DeleteLookupConfirm { lookup_name } => {
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteLookup {
                        name: lookup_name.clone(),
                        app: None,
                        owner: None,
                    },
                    description: format!("Delete lookup '{}'", lookup_name),
                })
            }
            PopupType::DeleteRoleConfirm { role_name } => Some(Action::QueueUndoableOperation {
                operation: UndoableOperation::DeleteRole {
                    name: role_name.clone(),
                },
                description: format!("Delete role '{}'", role_name),
            }),
            PopupType::DeleteProfileConfirm { profile_name } => {
                Some(Action::DeleteProfile { name: profile_name })
            }
            PopupType::DeleteSavedSearchConfirm { search_name } => {
                let name = search_name.clone();
                let description = format!("Delete saved search '{}'", search_name);
                let original = self.saved_searches.as_ref().and_then(|searches| {
                    searches.iter().find(|s| s.name == name).map(|s| {
                        crate::undo::SavedSearchRecoveryData {
                            search: s.search.clone(),
                            description: s.description.clone(),
                            disabled: s.disabled,
                        }
                    })
                });
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteSavedSearch { name, original },
                    description,
                })
            }
            _ => None,
        }
    }

    /// Handle confirmation dialog popups.
    ///
    /// Destructive operations are queued with the undo system instead of
    /// being executed immediately, giving users a grace period to undo.
    pub fn handle_confirm_popup(&mut self, key: KeyEvent) -> Option<Action> {
        let is_confirm = matches!(key.code, KeyCode::Char('y') | KeyCode::Enter);
        let is_reject = matches!(key.code, KeyCode::Char('n') | KeyCode::Esc);

        match (self.popup.as_ref().map(|p| &p.kind), is_confirm, is_reject) {
            (Some(kind), true, false) if kind.is_confirmation() => {
                let popup = self.popup.take()?;
                self.execute_confirmation_action(popup.kind)
            }
            (Some(kind), false, true) if kind.is_confirmation() => {
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
    pub(crate) fn get_index_settings_for_undo(
        &self,
        name: &str,
    ) -> Option<crate::undo::IndexSettings> {
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
    pub(crate) fn get_user_recovery_data(
        &self,
        name: &str,
    ) -> Option<crate::undo::UserRecoveryData> {
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
    use crate::ui::popup::Popup;
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

    #[test]
    fn test_confirm_delete_profile_direct_action() {
        let mut app = create_test_app();
        app.popup = Some(
            Popup::builder(PopupType::DeleteProfileConfirm {
                profile_name: "test_profile".to_string(),
            })
            .build(),
        );

        let action = app.handle_confirm_popup(key(KeyCode::Char('y')));

        // Profile deletion is local config only, direct action
        assert!(
            matches!(&action, Some(Action::DeleteProfile { name }) if name == "test_profile"),
            "Expected DeleteProfile action, got {:?}",
            action
        );
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_confirm_delete_profile_reject() {
        let mut app = create_test_app();
        app.popup = Some(
            Popup::builder(PopupType::DeleteProfileConfirm {
                profile_name: "test_profile".to_string(),
            })
            .build(),
        );

        let action = app.handle_confirm_popup(key(KeyCode::Char('n')));

        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_confirm_delete_saved_search_queues_undoable() {
        let mut app = create_test_app();
        app.popup = Some(
            Popup::builder(PopupType::DeleteSavedSearchConfirm {
                search_name: "test_search".to_string(),
            })
            .build(),
        );

        let action = app.handle_confirm_popup(key(KeyCode::Char('y')));

        // Saved search deletion should queue as undoable
        assert!(
            matches!(
                &action,
                Some(Action::QueueUndoableOperation {
                    operation: UndoableOperation::DeleteSavedSearch { name, .. },
                    description,
                }) if name == "test_search" && description.contains("Delete saved search")
            ),
            "Expected QueueUndoableOperation for DeleteSavedSearch, got {:?}",
            action
        );
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_confirm_delete_saved_search_reject() {
        let mut app = create_test_app();
        app.popup = Some(
            Popup::builder(PopupType::DeleteSavedSearchConfirm {
                search_name: "test_search".to_string(),
            })
            .build(),
        );

        let action = app.handle_confirm_popup(key(KeyCode::Esc));

        assert!(action.is_none());
        assert!(app.popup.is_none());
    }
}
