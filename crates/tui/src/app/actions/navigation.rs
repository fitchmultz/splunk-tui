//! Navigation action handlers for the TUI app.
//!
//! Responsibilities:
//! - Handle screen switching actions (SwitchTo*, NextScreen, PreviousScreen)
//! - Handle data loading triggers that also switch screens
//! - Handle list navigation (NavigateDown, NavigateUp, PageDown, PageUp, etc.)
//! - Handle job inspection mode transitions

use crate::action::Action;
use crate::app::App;
use crate::app::state::CurrentScreen;
use crate::onboarding::OnboardingMilestone;

impl App {
    /// Handle navigation-related actions.
    pub fn handle_navigation_action(&mut self, action: Action) {
        match action {
            Action::OpenHelpPopup => {
                self.open_help_popup();
                self.mark_onboarding_milestone(OnboardingMilestone::HelpOpened);
            }
            Action::OpenCommandPalette => {
                self.open_command_palette();
            }
            Action::SwitchToSearch => {
                self.current_screen = CurrentScreen::Search;
                self.init_focus_manager_for_screen(CurrentScreen::Search);
                self.clear_error_on_navigation();
            }
            Action::SwitchToSettingsScreen => {
                self.current_screen = CurrentScreen::Settings;
                self.init_focus_manager_for_screen(CurrentScreen::Settings);
                self.clear_error_on_navigation();
            }
            Action::NextScreen => {
                let next_screen = self.current_screen.next();
                self.current_screen = next_screen;
                self.init_focus_manager_for_screen(next_screen);
                self.clear_error_on_navigation();
                self.mark_onboarding_milestone(OnboardingMilestone::NavigationCycleCompleted);
            }
            Action::PreviousScreen => {
                let prev_screen = self.current_screen.previous();
                self.current_screen = prev_screen;
                self.init_focus_manager_for_screen(prev_screen);
                self.clear_error_on_navigation();
                self.mark_onboarding_milestone(OnboardingMilestone::NavigationCycleCompleted);
            }
            Action::LoadIndexes { offset, .. } => {
                self.current_screen = CurrentScreen::Indexes;
                self.init_focus_manager_for_screen(CurrentScreen::Indexes);
                if offset == 0 {
                    self.indexes_pagination.reset();
                }
            }
            Action::LoadClusterInfo => {
                self.current_screen = CurrentScreen::Cluster;
                self.init_focus_manager_for_screen(CurrentScreen::Cluster);
            }
            Action::ToggleClusterViewMode => {
                self.toggle_cluster_view_mode();
            }
            Action::LoadJobs { offset, .. } => {
                self.current_screen = CurrentScreen::Jobs;
                self.init_focus_manager_for_screen(CurrentScreen::Jobs);
                if offset == 0 {
                    self.jobs_pagination.reset();
                }
            }
            Action::LoadHealth => {
                self.current_screen = CurrentScreen::Health;
                self.init_focus_manager_for_screen(CurrentScreen::Health);
            }
            Action::LoadLicense => {
                self.current_screen = CurrentScreen::License;
                self.init_focus_manager_for_screen(CurrentScreen::License);
            }
            Action::LoadKvstore => {
                self.current_screen = CurrentScreen::Kvstore;
                self.init_focus_manager_for_screen(CurrentScreen::Kvstore);
            }
            Action::LoadSavedSearches => {
                self.current_screen = CurrentScreen::SavedSearches;
                self.init_focus_manager_for_screen(CurrentScreen::SavedSearches);
            }
            Action::LoadInternalLogs { .. } => {
                self.current_screen = CurrentScreen::InternalLogs;
                self.init_focus_manager_for_screen(CurrentScreen::InternalLogs);
            }
            Action::LoadApps { offset, .. } => {
                self.current_screen = CurrentScreen::Apps;
                self.init_focus_manager_for_screen(CurrentScreen::Apps);
                if offset == 0 {
                    self.apps_pagination.reset();
                }
            }
            Action::LoadUsers { offset, .. } => {
                self.current_screen = CurrentScreen::Users;
                self.init_focus_manager_for_screen(CurrentScreen::Users);
                if offset == 0 {
                    self.users_pagination.reset();
                }
            }
            Action::LoadSearchPeers { offset, .. } => {
                self.current_screen = CurrentScreen::SearchPeers;
                self.init_focus_manager_for_screen(CurrentScreen::SearchPeers);
                if offset == 0 {
                    self.search_peers_pagination.reset();
                }
            }
            Action::LoadInputs { offset, .. } => {
                self.current_screen = CurrentScreen::Inputs;
                self.init_focus_manager_for_screen(CurrentScreen::Inputs);
                if offset == 0 {
                    self.inputs_pagination.reset();
                }
            }
            Action::LoadFiredAlerts { offset, .. } => {
                self.current_screen = CurrentScreen::FiredAlerts;
                self.init_focus_manager_for_screen(CurrentScreen::FiredAlerts);
                if offset == 0 {
                    self.fired_alerts_pagination.reset();
                }
            }
            Action::LoadLookups { offset, .. } => {
                self.current_screen = CurrentScreen::Lookups;
                self.init_focus_manager_for_screen(CurrentScreen::Lookups);
                if offset == 0 {
                    self.lookups_pagination.reset();
                }
            }
            Action::LoadDashboards { offset, .. } => {
                self.current_screen = CurrentScreen::Dashboards;
                self.init_focus_manager_for_screen(CurrentScreen::Dashboards);
                if offset == 0 {
                    self.dashboards_pagination.reset();
                }
            }
            Action::LoadDataModels { offset, .. } => {
                self.current_screen = CurrentScreen::DataModels;
                self.init_focus_manager_for_screen(CurrentScreen::DataModels);
                if offset == 0 {
                    self.data_models_pagination.reset();
                }
            }
            Action::LoadMoreIndexes
            | Action::LoadMoreJobs
            | Action::LoadMoreApps
            | Action::LoadMoreUsers
            | Action::LoadMoreSearchPeers
            | Action::LoadMoreInputs
            | Action::LoadMoreFiredAlerts
            | Action::LoadMoreLookups
            | Action::LoadMoreWorkloadPools
            | Action::LoadMoreWorkloadRules
            | Action::LoadMoreDashboards
            | Action::LoadMoreDataModels => {
                // These are handled in the main loop which has access to pagination state
            }
            Action::LoadWorkloadPools { offset, .. } => {
                self.current_screen = CurrentScreen::WorkloadManagement;
                self.init_focus_manager_for_screen(CurrentScreen::WorkloadManagement);
                if offset == 0 {
                    self.workload_pools_pagination.reset();
                }
            }
            Action::LoadWorkloadRules { offset, .. } => {
                self.current_screen = CurrentScreen::WorkloadManagement;
                self.init_focus_manager_for_screen(CurrentScreen::WorkloadManagement);
                if offset == 0 {
                    self.workload_rules_pagination.reset();
                }
            }
            Action::LoadForwarders { offset, .. } => {
                self.current_screen = CurrentScreen::Forwarders;
                self.init_focus_manager_for_screen(CurrentScreen::Forwarders);
                if offset == 0 {
                    self.forwarders_pagination.reset();
                }
            }
            Action::ToggleWorkloadViewMode => {
                self.toggle_workload_view_mode();
            }
            Action::NavigateDown => self.next_item(),
            Action::NavigateUp => self.previous_item(),
            Action::PageDown => self.next_page(),
            Action::PageUp => self.previous_page(),
            Action::GoToTop => self.go_to_top(),
            Action::GoToBottom => self.go_to_bottom(),
            Action::InspectJob => {
                self.enter_job_inspect_mode();
            }
            Action::ExitInspectMode => {
                self.current_screen = CurrentScreen::Jobs;
            }
            _ => {}
        }
    }

    fn open_help_popup(&mut self) {
        use crate::ui::popup::{Popup, PopupType};
        self.help_scroll_offset = 0; // Reset scroll on open
        self.popup = Some(Popup::builder(PopupType::Help).build());
    }

    fn toggle_cluster_view_mode(&mut self) {
        use crate::app::state::ClusterViewMode;
        self.cluster_view_mode = self.cluster_view_mode.toggle();
        // When switching to peers view, trigger peers load if not already loaded
        if self.cluster_view_mode == ClusterViewMode::Peers && self.cluster_peers.is_none() {
            // The side effect handler will trigger the actual load
        }
    }

    fn toggle_workload_view_mode(&mut self) {
        use crate::app::state::WorkloadViewMode;
        self.workload_view_mode = self.workload_view_mode.toggle();
        // When switching to rules view, trigger rules load if not already loaded
        if self.workload_view_mode == WorkloadViewMode::Rules && self.workload_rules.is_none() {
            // The side effect handler will trigger the actual load
        }
    }

    fn enter_job_inspect_mode(&mut self) {
        if self.jobs.as_ref().map(|j| !j.is_empty()).unwrap_or(false)
            && self.jobs_state.selected().is_some()
        {
            self.current_screen = CurrentScreen::JobInspect;
        }
    }

    /// Clear error state when navigating to a new screen.
    /// This prevents errors from persisting across screen changes.
    fn clear_error_on_navigation(&mut self) {
        self.current_error = None;
        // Also clear validation state if leaving the Search screen
        if self.current_screen != CurrentScreen::Search {
            self.clear_validation_state();
        }
    }

    /// Initialize the FocusManager for the given screen.
    /// Sets up focusable component IDs based on the screen type.
    fn init_focus_manager_for_screen(&mut self, screen: CurrentScreen) {
        use crate::focus::FocusManager;

        let component_ids: Vec<String> = match screen {
            CurrentScreen::Search => {
                vec!["search_query".to_string(), "search_results".to_string()]
            }
            CurrentScreen::Configs => {
                vec![
                    "config_search".to_string(),
                    "config_files".to_string(),
                    "config_stanzas".to_string(),
                ]
            }
            CurrentScreen::Jobs => {
                // Jobs screen has filter input and job list
                vec!["jobs_filter".to_string(), "jobs_list".to_string()]
            }
            CurrentScreen::Indexes => {
                vec!["indexes_list".to_string()]
            }
            CurrentScreen::Cluster => {
                vec!["cluster_summary".to_string(), "cluster_peers".to_string()]
            }
            CurrentScreen::Health => {
                vec!["health_status".to_string()]
            }
            CurrentScreen::License => {
                vec!["license_usage".to_string()]
            }
            CurrentScreen::Kvstore => {
                vec!["kvstore_status".to_string()]
            }
            CurrentScreen::SavedSearches => {
                vec!["saved_searches_list".to_string()]
            }
            CurrentScreen::Macros => {
                vec!["macros_list".to_string()]
            }
            CurrentScreen::InternalLogs => {
                vec!["internal_logs_list".to_string()]
            }
            CurrentScreen::Apps => {
                vec!["apps_list".to_string()]
            }
            CurrentScreen::Users => {
                vec!["users_list".to_string()]
            }
            CurrentScreen::Roles => {
                vec!["roles_list".to_string()]
            }
            CurrentScreen::SearchPeers => {
                vec!["search_peers_list".to_string()]
            }
            CurrentScreen::Inputs => {
                vec!["inputs_list".to_string()]
            }
            CurrentScreen::FiredAlerts => {
                vec!["fired_alerts_list".to_string()]
            }
            CurrentScreen::Forwarders => {
                vec!["forwarders_list".to_string()]
            }
            CurrentScreen::Lookups => {
                vec!["lookups_list".to_string()]
            }
            CurrentScreen::Dashboards => {
                vec!["dashboards_list".to_string()]
            }
            CurrentScreen::DataModels => {
                vec!["data_models_list".to_string()]
            }
            CurrentScreen::WorkloadManagement => {
                vec!["workload_pools".to_string(), "workload_rules".to_string()]
            }
            CurrentScreen::Shc => {
                vec!["shc_summary".to_string(), "shc_members".to_string()]
            }
            CurrentScreen::Audit => {
                vec!["audit_events_list".to_string()]
            }
            CurrentScreen::Overview => {
                vec!["overview_resources".to_string()]
            }
            CurrentScreen::MultiInstance => {
                vec!["multi_instance_list".to_string()]
            }
            CurrentScreen::Settings => {
                vec!["settings_options".to_string()]
            }
            CurrentScreen::JobInspect => {
                // Job inspect is a detail view with single focus
                vec!["job_inspect_details".to_string()]
            }
        };

        self.focus_manager = FocusManager::new(component_ids);
        // Enable focus navigation mode if there are focusable components
        self.focus_navigation_mode = self.focus_manager.len() > 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ConnectionContext;
    use crate::app::state::{ClusterViewMode, CurrentScreen};

    #[test]
    fn test_next_screen_navigation() {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Indexes;

        app.handle_navigation_action(Action::NextScreen);

        assert_eq!(app.current_screen, CurrentScreen::Indexes.next());
    }

    #[test]
    fn test_previous_screen_navigation() {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Jobs;

        app.handle_navigation_action(Action::PreviousScreen);

        assert_eq!(app.current_screen, CurrentScreen::Jobs.previous());
    }

    #[test]
    fn test_switch_to_search() {
        let mut app = App::new(None, ConnectionContext::default());

        app.handle_navigation_action(Action::SwitchToSearch);

        assert_eq!(app.current_screen, CurrentScreen::Search);
    }

    #[test]
    fn test_load_indexes_resets_pagination() {
        let mut app = App::new(None, ConnectionContext::default());
        // Simulate some pagination state
        app.indexes_pagination.update_loaded(10);

        app.handle_navigation_action(Action::LoadIndexes {
            count: 100,
            offset: 0,
        });

        assert_eq!(app.current_screen, CurrentScreen::Indexes);
        // Pagination should be reset
        assert_eq!(app.indexes_pagination.total_loaded, 0);
    }

    #[test]
    fn test_toggle_cluster_view_mode() {
        let mut app = App::new(None, ConnectionContext::default());
        let initial_mode = app.cluster_view_mode;

        app.handle_navigation_action(Action::ToggleClusterViewMode);

        assert_eq!(app.cluster_view_mode, initial_mode.toggle());
    }

    #[test]
    fn test_toggle_cluster_view_mode_triggers_peers_load() {
        let mut app = App::new(None, ConnectionContext::default());
        app.cluster_view_mode = ClusterViewMode::Summary;
        app.cluster_peers = None;

        app.handle_navigation_action(Action::ToggleClusterViewMode);

        // Should switch to Peers mode
        assert_eq!(app.cluster_view_mode, ClusterViewMode::Peers);
        // Note: actual peers load is triggered by side effects, not the action handler
    }

    #[test]
    fn test_inspect_job_with_no_jobs_does_nothing() {
        let mut app = App::new(None, ConnectionContext::default());
        app.jobs = None;
        app.current_screen = CurrentScreen::Jobs;

        app.handle_navigation_action(Action::InspectJob);

        // Should remain on Jobs screen
        assert_eq!(app.current_screen, CurrentScreen::Jobs);
    }

    #[test]
    fn test_exit_inspect_mode_returns_to_jobs() {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::JobInspect;

        app.handle_navigation_action(Action::ExitInspectMode);

        assert_eq!(app.current_screen, CurrentScreen::Jobs);
    }
}
