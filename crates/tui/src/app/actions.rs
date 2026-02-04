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
//!
//! This module delegates to domain-specific submodules:
//! - `navigation`: Screen switching, list navigation
//! - `data_loading`: Data load results (*Loaded actions)
//! - `search`: Search lifecycle and results
//! - `profiles`: Profile switching and management
//! - `system`: Loading, notifications, clipboard, etc.

use crate::action::Action;
use crate::app::App;
use crate::app::state::SearchInputMode;

// Domain-specific action handler modules
mod data_loading;
mod navigation;
mod profiles;
mod search;
mod system;

impl App {
    /// Pure state mutation based on Action.
    ///
    /// This method delegates to domain-specific handlers based on action type.
    pub fn update(&mut self, action: Action) {
        match action {
            // Navigation actions
            Action::OpenHelpPopup
            | Action::SwitchToSearch
            | Action::SwitchToSettingsScreen
            | Action::NextScreen
            | Action::PreviousScreen
            | Action::LoadIndexes { .. }
            | Action::LoadClusterInfo
            | Action::ToggleClusterViewMode
            | Action::LoadJobs { .. }
            | Action::LoadHealth
            | Action::LoadLicense
            | Action::LoadKvstore
            | Action::LoadSavedSearches
            | Action::LoadInternalLogs { .. }
            | Action::LoadApps { .. }
            | Action::LoadUsers { .. }
            | Action::LoadRoles { .. }
            | Action::LoadSearchPeers { .. }
            | Action::LoadInputs { .. }
            | Action::LoadForwarders { .. }
            | Action::LoadFiredAlerts { .. }
            | Action::LoadLookups { .. }
            | Action::LoadDashboards { .. }
            | Action::LoadDataModels { .. }
            | Action::LoadMoreIndexes
            | Action::LoadMoreJobs
            | Action::LoadMoreApps
            | Action::LoadMoreUsers
            | Action::LoadMoreSearchPeers
            | Action::LoadMoreInputs
            | Action::LoadMoreFiredAlerts
            | Action::LoadMoreLookups
            | Action::NavigateDown
            | Action::NavigateUp
            | Action::PageDown
            | Action::PageUp
            | Action::GoToTop
            | Action::GoToBottom
            | Action::InspectJob
            | Action::ExitInspectMode => {
                self.handle_navigation_action(action);
            }

            // Data loading actions
            Action::IndexesLoaded(_)
            | Action::MoreIndexesLoaded(_)
            | Action::JobsLoaded(_)
            | Action::MoreJobsLoaded(_)
            | Action::SavedSearchesLoaded(_)
            | Action::InternalLogsLoaded(_)
            | Action::ClusterInfoLoaded(_)
            | Action::ClusterPeersLoaded(_)
            | Action::HealthLoaded(_)
            | Action::HealthStatusLoaded(_)
            | Action::LicenseLoaded(_)
            | Action::KvstoreLoaded(_)
            | Action::AppsLoaded(_)
            | Action::MoreAppsLoaded(_)
            | Action::UsersLoaded(_)
            | Action::MoreUsersLoaded(_)
            | Action::SearchPeersLoaded(_)
            | Action::MoreSearchPeersLoaded(_)
            | Action::ForwardersLoaded(_)
            | Action::MoreForwardersLoaded(_)
            | Action::LookupsLoaded(_)
            | Action::MoreLookupsLoaded(_)
            | Action::InputsLoaded(_)
            | Action::MoreInputsLoaded(_)
            | Action::FiredAlertsLoaded(_)
            | Action::MoreFiredAlertsLoaded(_)
            | Action::AuditEventsLoaded(_)
            | Action::ConfigFilesLoaded(_)
            | Action::ConfigStanzasLoaded(_)
            | Action::SettingsLoaded(_)
            | Action::OverviewLoaded(_)
            | Action::MultiInstanceOverviewLoaded(_)
            // Macros
            | Action::MacrosLoaded(_)
            | Action::MacroCreated(_)
            | Action::MacroUpdated(_)
            | Action::MacroDeleted(_) => {
                self.handle_data_loading_action(action);
            }

            // Search actions
            Action::SearchStarted(_)
            | Action::SearchComplete(_)
            | Action::MoreSearchResultsLoaded(_) => {
                self.handle_search_action(action);
            }

            // Profile actions
            Action::OpenProfileSwitcher
            | Action::OpenProfileSelectorWithList(_)
            | Action::ProfileSelected(_)
            | Action::ProfileSwitchResult(_)
            | Action::ClearAllData
            | Action::OpenCreateProfileDialog
            | Action::OpenEditProfileDialogWithData { .. }
            | Action::OpenDeleteProfileConfirm { .. }
            | Action::ProfileSaved(_)
            | Action::ProfileDeleted(_) => {
                self.handle_profile_action(action);
            }

            // System actions
            Action::Loading(_)
            | Action::Progress(_)
            | Action::Notify(_, _)
            | Action::Tick
            | Action::CopyToClipboard(_)
            | Action::Resize(_, _)
            | Action::EnterSearchMode
            | Action::SearchInput(_)
            | Action::ClearSearch
            | Action::CycleSortColumn
            | Action::ToggleSortDirection
            | Action::CycleTheme
            | Action::SplValidationResult { .. }
            | Action::ShowErrorDetails(_)
            | Action::ShowErrorDetailsFromCurrent
            | Action::ClearErrorDetails
            | Action::JobOperationComplete(_)
            | Action::OpenCreateIndexDialog
            | Action::EditSavedSearch
            | Action::SavedSearchUpdated(_)
            | Action::MaintenanceModeSet { .. }
            | Action::ClusterRebalanced { .. }
            | Action::PeerDecommissioned { .. }
            | Action::PeerRemoved { .. } => {
                self.handle_system_action(action);
            }

            // Focus management actions
            Action::NextFocus | Action::PreviousFocus | Action::SetFocus(_) | Action::ToggleFocusMode => {
                self.handle_focus_action(action);
            }

            // Catch-all for unhandled actions
            _ => {}
        }
    }

    /// Handle focus management actions.
    fn handle_focus_action(&mut self, action: Action) {
        match action {
            Action::NextFocus => {
                self.focus_manager.next();
                self.update_focus_for_current_screen();
            }
            Action::PreviousFocus => {
                self.focus_manager.prev();
                self.update_focus_for_current_screen();
            }
            Action::SetFocus(id) => {
                self.focus_manager.set_focus(&id);
                self.update_focus_for_current_screen();
            }
            Action::ToggleFocusMode => {
                self.focus_navigation_mode = !self.focus_navigation_mode;
                let msg = if self.focus_navigation_mode {
                    "Focus navigation mode ON (Ctrl+Tab to navigate)"
                } else {
                    "Focus navigation mode OFF"
                };
                self.toasts.push(crate::ui::Toast::new(
                    msg.to_string(),
                    crate::ui::ToastLevel::Info,
                ));
            }
            _ => {}
        }
    }

    /// Update component focus states based on FocusManager for the current screen.
    fn update_focus_for_current_screen(&mut self) {
        use crate::app::state::CurrentScreen;

        match self.current_screen {
            CurrentScreen::Search => {
                // Update SearchInputMode based on focus
                if let Some(focused_id) = self.focus_manager.current_id() {
                    self.search_input_mode = if focused_id == "search_query" {
                        SearchInputMode::QueryFocused
                    } else {
                        SearchInputMode::ResultsFocused
                    };
                }
            }
            CurrentScreen::Configs => {
                // Configs screen has search and results components
                if let Some(focused_id) = self.focus_manager.current_id() {
                    self.config_search_mode = focused_id == "config_search";
                }
            }
            // Add other screens as needed
            _ => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use crate::app::state::HealthState;
    use splunk_client::models::{HealthCheckOutput, SplunkHealth};
    use std::collections::HashMap;
    use std::sync::Arc;

    #[test]
    fn test_health_status_loaded_action_ok() {
        let mut app = App::new(None, ConnectionContext::default());

        // Simulate receiving a healthy status
        let health = SplunkHealth {
            health: "green".to_string(),
            features: HashMap::new(),
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
                features: HashMap::new(),
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
