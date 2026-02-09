//! Profile management action handlers for the TUI app.
//!
//! Responsibilities:
//! - Handle profile switching actions
//! - Handle profile CRUD operations (create, edit, delete)
//! - Clear cached data on profile switch
//! - Show profile-related popups

use crate::action::Action;
use crate::app::App;
use crate::ui::Toast;

impl App {
    /// Handle profile-related actions.
    pub fn handle_profile_action(&mut self, action: Action) {
        match action {
            Action::OpenProfileSwitcher => {
                // This action is handled in main.rs side effects which will
                // send the profile list and trigger the popup opening
            }
            Action::OpenProfileSelectorWithList(profiles) => {
                self.open_profile_selector(profiles);
            }
            Action::ProfileSelected(_) => {
                // This action is handled in main.rs side effects
                // It triggers the actual profile switch with new client creation
            }
            Action::ProfileSwitchResult(Ok(ctx)) => {
                self.handle_profile_switch_success(ctx);
            }
            Action::ProfileSwitchResult(Err(e)) => {
                self.toasts
                    .push(Toast::error(format!("Failed to switch profile: {}", e)));
            }
            Action::ClearAllData => {
                self.clear_all_cached_data();
            }
            Action::OpenCreateProfileDialog { .. } => {
                self.open_create_profile_dialog();
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
                self.open_edit_profile_dialog(
                    original_name,
                    name_input,
                    base_url_input,
                    username_input,
                    skip_verify,
                    timeout_seconds,
                    max_retries,
                );
            }
            Action::OpenDeleteProfileConfirm { name } => {
                self.open_delete_profile_confirm(name);
            }
            Action::ProfileSaved(Ok(profile_name)) => {
                self.handle_profile_saved(profile_name);
            }
            Action::ProfileSaved(Err(error_msg)) => {
                self.toasts.push(Toast::error(error_msg));
            }
            Action::ProfileDeleted(Ok(profile_name)) => {
                self.handle_profile_deleted(profile_name);
            }
            Action::ProfileDeleted(Err(error_msg)) => {
                self.toasts.push(Toast::error(error_msg));
            }
            _ => {}
        }
    }

    fn open_profile_selector(&mut self, profiles: Vec<String>) {
        use crate::ui::popup::{Popup, PopupType};
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

    fn handle_profile_switch_success(&mut self, ctx: crate::ConnectionContext) {
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

    fn clear_all_cached_data(&mut self) {
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
        self.forwarders = None;
        self.lookups = None;
        self.inputs = None;
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
        self.forwarders_state.select(Some(0));
        self.lookups_state.select(Some(0));
        self.inputs_state.select(Some(0));
        self.fired_alerts_state.select(Some(0));
        // Trigger reload for current screen
        // The load action will be sent by main.rs after this
    }

    fn open_create_profile_dialog(&mut self) {
        use crate::ui::popup::{Popup, PopupType, ProfileField};
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
                use_keyring: true,
                selected_field: ProfileField::Name,
                from_tutorial: false,
            })
            .build(),
        );
    }

    #[allow(clippy::too_many_arguments)]
    fn open_edit_profile_dialog(
        &mut self,
        original_name: String,
        name_input: String,
        base_url_input: String,
        username_input: String,
        skip_verify: bool,
        timeout_seconds: u64,
        max_retries: usize,
    ) {
        use crate::ui::popup::{Popup, PopupType, ProfileField};
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
                use_keyring: true,
                selected_field: ProfileField::Name,
            })
            .build(),
        );
    }

    fn open_delete_profile_confirm(&mut self, name: String) {
        use crate::ui::popup::{Popup, PopupType};
        self.popup =
            Some(Popup::builder(PopupType::DeleteProfileConfirm { profile_name: name }).build());
    }

    fn handle_profile_saved(&mut self, profile_name: String) {
        self.popup = None;
        self.toasts.push(Toast::info(format!(
            "Profile '{}' saved successfully",
            profile_name
        )));
    }

    fn handle_profile_deleted(&mut self, profile_name: String) {
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
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;

    #[test]
    fn test_profile_switch_result_updates_context() {
        let mut app = App::new(None, ConnectionContext::default());

        let ctx = ConnectionContext {
            profile_name: Some("test-profile".to_string()),
            base_url: "https://splunk.example.com".to_string(),
            auth_mode: "session".to_string(),
        };

        app.handle_profile_action(Action::ProfileSwitchResult(Ok(ctx)));

        assert_eq!(app.profile_name, Some("test-profile".to_string()));
        assert_eq!(app.base_url, Some("https://splunk.example.com".to_string()));
        assert!(app.server_version.is_none()); // Cleared until health check
    }

    #[test]
    fn test_profile_switch_result_error_shows_toast() {
        let mut app = App::new(None, ConnectionContext::default());

        app.handle_profile_action(Action::ProfileSwitchResult(Err(
            "Connection failed".to_string()
        )));

        assert_eq!(app.toasts.len(), 1);
        assert!(app.toasts[0].message.contains("Failed to switch profile"));
    }

    #[test]
    fn test_clear_all_data_clears_cached_data() {
        let mut app = App::new(None, ConnectionContext::default());
        // Populate some data
        app.indexes = Some(vec![]);
        app.jobs = Some(vec![]);
        app.search_results.push(Default::default());

        app.handle_profile_action(Action::ClearAllData);

        assert!(app.indexes.is_none());
        assert!(app.jobs.is_none());
        assert!(app.search_results.is_empty());
    }

    #[test]
    fn test_profile_saved_closes_popup() {
        let mut app = App::new(None, ConnectionContext::default());
        use crate::ui::popup::{Popup, PopupType};
        app.popup = Some(Popup::builder(PopupType::Help).build());

        app.handle_profile_action(Action::ProfileSaved(Ok("new-profile".to_string())));

        assert!(app.popup.is_none());
        assert_eq!(app.toasts.len(), 1);
    }

    #[test]
    fn test_profile_deleted_clears_current_profile() {
        let mut app = App::new(None, ConnectionContext::default());
        app.profile_name = Some("to-delete".to_string());

        app.handle_profile_action(Action::ProfileDeleted(Ok("to-delete".to_string())));

        assert!(app.popup.is_none());
        assert!(app.profile_name.is_none()); // Current profile was deleted
    }

    #[test]
    fn test_profile_deleted_keeps_other_profiles() {
        let mut app = App::new(None, ConnectionContext::default());
        app.profile_name = Some("other-profile".to_string());

        app.handle_profile_action(Action::ProfileDeleted(Ok("deleted-profile".to_string())));

        assert_eq!(app.profile_name, Some("other-profile".to_string()));
    }
}
