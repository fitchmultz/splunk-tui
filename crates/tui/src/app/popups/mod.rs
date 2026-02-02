//! Popup input handling for the TUI app.
//!
//! Responsibilities:
//! - Handle keyboard input when popups are active
//! - Dispatch to appropriate sub-handlers based on popup type
//! - Manage export popup state and input
//!
//! Non-responsibilities:
//! - Does NOT render popups (handled by ui::popup module)
//! - Does NOT define popup types (handled by ui::popup module)

use crate::action::Action;
use crate::app::App;
use crate::ui::popup::PopupType;
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

mod confirm;
mod export;
mod index;
mod macros;
mod misc;
mod profile;
mod saved_search;
mod user;

impl App {
    /// Handle keyboard input when a popup is active.
    pub fn handle_popup_input(&mut self, key: KeyEvent) -> Option<Action> {
        // Check for global quit first (Ctrl+Q works from any popup)
        if key.code == KeyCode::Char('q') && key.modifiers.contains(KeyModifiers::CONTROL) {
            return Some(Action::Quit);
        }

        // Get the popup type for dispatch
        let popup_type = self.popup.as_ref().map(|p| &p.kind);

        match popup_type {
            // Help popup
            Some(PopupType::Help) => self.handle_help_popup(key),

            // Error details popup
            Some(PopupType::ErrorDetails) => self.handle_error_details_popup(key),

            // Export popup
            Some(PopupType::ExportSearch) => self.handle_export_popup(key),

            // Confirmation dialogs
            Some(
                PopupType::ConfirmCancel(_)
                | PopupType::ConfirmDelete(_)
                | PopupType::ConfirmCancelBatch(_)
                | PopupType::ConfirmDeleteBatch(_)
                | PopupType::ConfirmEnableApp(_)
                | PopupType::ConfirmDisableApp(_)
                | PopupType::ConfirmRemoveApp(_)
                | PopupType::DeleteIndexConfirm { .. }
                | PopupType::DeleteUserConfirm { .. },
            ) => self.handle_confirm_popup(key),

            // Profile management
            Some(
                PopupType::ProfileSelector { .. }
                | PopupType::CreateProfile { .. }
                | PopupType::EditProfile { .. }
                | PopupType::DeleteProfileConfirm { .. },
            ) => self.handle_profile_popup(key),

            // Index management
            Some(
                PopupType::IndexDetails
                | PopupType::CreateIndex { .. }
                | PopupType::ModifyIndex { .. },
            ) => self.handle_index_popup(key),

            // User management
            Some(PopupType::CreateUser { .. } | PopupType::ModifyUser { .. }) => {
                self.handle_user_popup(key)
            }

            // Install app dialog
            Some(PopupType::InstallAppDialog { .. }) => self.handle_install_app_popup(key),

            // Role management (not yet implemented - close popup on Esc)
            Some(PopupType::CreateRole { .. } | PopupType::ModifyRole { .. }) => {
                if key.code == KeyCode::Esc {
                    self.popup = None;
                }
                None
            }
            Some(PopupType::DeleteRoleConfirm { role_name }) => match key.code {
                KeyCode::Char('y') | KeyCode::Enter => {
                    let name = role_name.clone();
                    self.popup = None;
                    Some(Action::DeleteRole { name })
                }
                KeyCode::Char('n') | KeyCode::Esc => {
                    self.popup = None;
                    None
                }
                _ => None,
            },

            // Saved search edit popup
            Some(PopupType::EditSavedSearch { .. }) => self.handle_saved_search_popup(key),

            // Macro creation/editing popups
            Some(PopupType::CreateMacro { .. } | PopupType::EditMacro { .. }) => {
                self.handle_macro_popup(key)
            }

            // No popup active
            None => None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use crate::action::ExportFormat;
    use crate::app::export::ExportTarget;
    use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

    fn key(c: KeyCode) -> KeyEvent {
        KeyEvent::new(c, KeyModifiers::empty())
    }

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    #[test]
    fn test_popup_help_close() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::Help).build());

        // Close with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());

        // Reopen and close with 'q'
        app.popup = Some(Popup::builder(PopupType::Help).build());
        let action = app.handle_popup_input(key(KeyCode::Char('q')));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_help_scroll() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::Help).build());
        app.help_scroll_offset = 0;

        // Scroll down with 'j'
        let action = app.handle_popup_input(key(KeyCode::Char('j')));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 1);

        // Scroll down with Down arrow
        let action = app.handle_popup_input(key(KeyCode::Down));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 2);

        // Scroll up with 'k'
        let action = app.handle_popup_input(key(KeyCode::Char('k')));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 1);

        // Scroll up with Up arrow
        let action = app.handle_popup_input(key(KeyCode::Up));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 0);

        // Scroll up at 0 should stay at 0 (saturating_sub)
        let action = app.handle_popup_input(key(KeyCode::Up));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 0);

        // Page down
        let action = app.handle_popup_input(key(KeyCode::PageDown));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 10);

        // Page up
        let action = app.handle_popup_input(key(KeyCode::PageUp));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 0);

        // Page up at 0 should stay at 0 (saturating_sub)
        let action = app.handle_popup_input(key(KeyCode::PageUp));
        assert!(action.is_none());
        assert_eq!(app.help_scroll_offset, 0);
    }

    #[test]
    fn test_popup_help_close_resets_scroll() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::Help).build());
        app.help_scroll_offset = 5;

        // Close with Esc should reset scroll offset
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
        assert_eq!(app.help_scroll_offset, 0);

        // Reopen and scroll
        app.popup = Some(Popup::builder(PopupType::Help).build());
        app.help_scroll_offset = 3;

        // Close with 'q' should reset scroll offset
        let action = app.handle_popup_input(key(KeyCode::Char('q')));
        assert!(action.is_none());
        assert!(app.popup.is_none());
        assert_eq!(app.help_scroll_offset, 0);
    }

    #[test]
    fn test_popup_confirm_cancel() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());

        // Confirm with 'y'
        let action = app.handle_popup_input(key(KeyCode::Char('y')));
        assert!(matches!(action, Some(Action::CancelJob(sid)) if sid == "test-sid"));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_confirm_cancel_with_enter() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());

        // Confirm with Enter
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(matches!(action, Some(Action::CancelJob(sid)) if sid == "test-sid"));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_confirm_cancel_reject() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());

        // Reject with 'n'
        let action = app.handle_popup_input(key(KeyCode::Char('n')));
        assert!(action.is_none());
        assert!(app.popup.is_none());

        // Reopen and reject with Esc
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_export_search_input() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ExportSearch).build());
        app.export_target = Some(ExportTarget::SearchResults);
        app.export_input = String::new();

        // Type some characters
        app.handle_popup_input(key(KeyCode::Char('t')));
        app.handle_popup_input(key(KeyCode::Char('e')));
        app.handle_popup_input(key(KeyCode::Char('s')));
        app.handle_popup_input(key(KeyCode::Char('t')));

        assert_eq!(app.export_input, "test");

        // Backspace
        app.handle_popup_input(key(KeyCode::Backspace));
        assert_eq!(app.export_input, "tes");
    }

    #[test]
    fn test_popup_export_search_format_toggle() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ExportSearch).build());
        app.export_target = Some(ExportTarget::SearchResults);
        app.export_input = "test.json".to_string();
        app.export_format = ExportFormat::Json;

        // Toggle format with Tab
        app.handle_popup_input(key(KeyCode::Tab));
        assert_eq!(app.export_format, ExportFormat::Csv);
        assert_eq!(app.export_input, "test.csv");

        // Toggle back
        app.handle_popup_input(key(KeyCode::Tab));
        assert_eq!(app.export_format, ExportFormat::Json);
        assert_eq!(app.export_input, "test.json");
    }

    #[test]
    fn test_popup_export_search_cancel() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ExportSearch).build());
        app.export_target = Some(ExportTarget::SearchResults);

        // Cancel with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
        assert!(app.export_target.is_none());
    }

    #[test]
    fn test_popup_error_details_navigation() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ErrorDetails).build());
        app.error_scroll_offset = 0;

        // Scroll down
        app.handle_popup_input(key(KeyCode::Char('j')));
        assert_eq!(app.error_scroll_offset, 1);

        // Scroll down more
        app.handle_popup_input(key(KeyCode::Down));
        assert_eq!(app.error_scroll_offset, 2);

        // Page down
        app.handle_popup_input(key(KeyCode::PageDown));
        assert_eq!(app.error_scroll_offset, 12);

        // Scroll up
        app.handle_popup_input(key(KeyCode::Char('k')));
        assert_eq!(app.error_scroll_offset, 11);

        // Page up
        app.handle_popup_input(key(KeyCode::PageUp));
        assert_eq!(app.error_scroll_offset, 1);

        // Close
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_error_details_close_with_e() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ErrorDetails).build());

        // Close with 'e' key (should close the popup)
        let action = app.handle_popup_input(key(KeyCode::Char('e')));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    // Global quit tests (Ctrl+Q from any popup)

    #[test]
    fn test_global_quit_from_help_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::Help).build());

        // Ctrl+Q should quit even from help popup
        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_error_details_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ErrorDetails).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_confirm_cancel_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmCancel("test-sid".to_string())).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_confirm_delete_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ConfirmDelete("test-sid".to_string())).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_confirm_cancel_batch_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::ConfirmCancelBatch(vec![
                "sid1".to_string(),
                "sid2".to_string(),
            ]))
            .build(),
        );

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_confirm_delete_batch_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::ConfirmDeleteBatch(vec![
                "sid1".to_string(),
                "sid2".to_string(),
            ]))
            .build(),
        );

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_confirm_enable_app_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup =
            Some(Popup::builder(PopupType::ConfirmEnableApp("test-app".to_string())).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_confirm_disable_app_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup =
            Some(Popup::builder(PopupType::ConfirmDisableApp("test-app".to_string())).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_export_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::ExportSearch).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_index_details_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(Popup::builder(PopupType::IndexDetails).build());

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    // Index creation popup tests

    #[test]
    fn test_popup_create_index_input() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
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

        // Type some characters
        app.handle_popup_input(key(KeyCode::Char('t')));
        app.handle_popup_input(key(KeyCode::Char('e')));
        app.handle_popup_input(key(KeyCode::Char('s')));
        app.handle_popup_input(key(KeyCode::Char('t')));

        assert!(
            matches!(app.popup, Some(Popup { kind: PopupType::CreateIndex { ref name_input, .. }, .. }) if name_input == "test")
        );

        // Backspace
        app.handle_popup_input(key(KeyCode::Backspace));
        assert!(
            matches!(app.popup, Some(Popup { kind: PopupType::CreateIndex { ref name_input, .. }, .. }) if name_input == "tes")
        );
    }

    #[test]
    fn test_popup_create_index_submit() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateIndex {
                name_input: "test_index".to_string(),
                max_data_size_mb: Some(1000),
                max_hot_buckets: Some(10),
                max_warm_db_count: None,
                frozen_time_period_secs: None,
                home_path: None,
                cold_db_path: None,
                thawed_path: None,
                cold_to_frozen_dir: None,
            })
            .build(),
        );

        // Submit with Enter
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(
            matches!(action, Some(Action::CreateIndex { params }) if params.name == "test_index" && params.max_data_size_mb == Some(1000))
        );
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_create_index_empty_name() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
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

        // Submit with empty name should not emit action
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(action.is_none());
        assert!(app.popup.is_some());
    }

    #[test]
    fn test_popup_create_index_cancel() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::CreateIndex {
                name_input: "test".to_string(),
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

        // Cancel with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    // Index modification popup tests

    #[test]
    fn test_popup_modify_index_submit() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::ModifyIndex {
                index_name: "main".to_string(),
                current_max_data_size_mb: Some(500000),
                current_max_hot_buckets: Some(10),
                current_max_warm_db_count: Some(300),
                current_frozen_time_period_secs: Some(15552000),
                current_home_path: Some("/splunk/main/db".to_string()),
                current_cold_db_path: Some("/splunk/main/colddb".to_string()),
                current_thawed_path: Some("/splunk/main/thaweddb".to_string()),
                current_cold_to_frozen_dir: None,
                new_max_data_size_mb: Some(2000),
                new_max_hot_buckets: Some(15),
                new_max_warm_db_count: Some(400),
                new_frozen_time_period_secs: Some(2592000),
                new_home_path: None,
                new_cold_db_path: None,
                new_thawed_path: None,
                new_cold_to_frozen_dir: None,
            })
            .build(),
        );

        // Submit with Enter
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(
            matches!(action, Some(Action::ModifyIndex { name, params }) if name == "main" && params.max_data_size_mb == Some(2000))
        );
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_modify_index_cancel() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::ModifyIndex {
                index_name: "main".to_string(),
                current_max_data_size_mb: None,
                current_max_hot_buckets: None,
                current_max_warm_db_count: None,
                current_frozen_time_period_secs: None,
                current_home_path: None,
                current_cold_db_path: None,
                current_thawed_path: None,
                current_cold_to_frozen_dir: None,
                new_max_data_size_mb: None,
                new_max_hot_buckets: None,
                new_max_warm_db_count: None,
                new_frozen_time_period_secs: None,
                new_home_path: None,
                new_cold_db_path: None,
                new_thawed_path: None,
                new_cold_to_frozen_dir: None,
            })
            .build(),
        );

        // Cancel with Esc
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    // Index deletion popup tests

    #[test]
    fn test_popup_delete_index_confirm() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::DeleteIndexConfirm {
                index_name: "test_index".to_string(),
            })
            .build(),
        );

        // Confirm with 'y'
        let action = app.handle_popup_input(key(KeyCode::Char('y')));
        assert!(matches!(action, Some(Action::DeleteIndex { name }) if name == "test_index"));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_delete_index_confirm_with_enter() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::DeleteIndexConfirm {
                index_name: "test_index".to_string(),
            })
            .build(),
        );

        // Confirm with Enter
        let action = app.handle_popup_input(key(KeyCode::Enter));
        assert!(matches!(action, Some(Action::DeleteIndex { name }) if name == "test_index"));
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_popup_delete_index_cancel() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::DeleteIndexConfirm {
                index_name: "test_index".to_string(),
            })
            .build(),
        );

        // Cancel with 'n'
        let action = app.handle_popup_input(key(KeyCode::Char('n')));
        assert!(action.is_none());
        assert!(app.popup.is_none());

        // Reopen and cancel with Esc
        app.popup = Some(
            Popup::builder(PopupType::DeleteIndexConfirm {
                index_name: "test_index".to_string(),
            })
            .build(),
        );
        let action = app.handle_popup_input(key(KeyCode::Esc));
        assert!(action.is_none());
        assert!(app.popup.is_none());
    }

    #[test]
    fn test_global_quit_from_create_index_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
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

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_modify_index_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::ModifyIndex {
                index_name: "main".to_string(),
                current_max_data_size_mb: None,
                current_max_hot_buckets: None,
                current_max_warm_db_count: None,
                current_frozen_time_period_secs: None,
                current_home_path: None,
                current_cold_db_path: None,
                current_thawed_path: None,
                current_cold_to_frozen_dir: None,
                new_max_data_size_mb: None,
                new_max_hot_buckets: None,
                new_max_warm_db_count: None,
                new_frozen_time_period_secs: None,
                new_home_path: None,
                new_cold_db_path: None,
                new_thawed_path: None,
                new_cold_to_frozen_dir: None,
            })
            .build(),
        );

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }

    #[test]
    fn test_global_quit_from_delete_index_popup() {
        use crate::ui::popup::Popup;

        let mut app = App::new(None, ConnectionContext::default());
        app.popup = Some(
            Popup::builder(PopupType::DeleteIndexConfirm {
                index_name: "test_index".to_string(),
            })
            .build(),
        );

        let action = app.handle_popup_input(ctrl_key('q'));
        assert!(matches!(action, Some(Action::Quit)));
    }
}
