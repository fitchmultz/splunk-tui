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

impl App {
    /// Handle navigation-related actions.
    pub fn handle_navigation_action(&mut self, action: Action) {
        match action {
            Action::OpenHelpPopup => {
                self.open_help_popup();
            }
            Action::SwitchToSearch => {
                self.current_screen = CurrentScreen::Search;
            }
            Action::SwitchToSettingsScreen => {
                self.current_screen = CurrentScreen::Settings;
            }
            Action::NextScreen => {
                self.current_screen = self.current_screen.next();
            }
            Action::PreviousScreen => {
                self.current_screen = self.current_screen.previous();
            }
            Action::LoadIndexes { offset, .. } => {
                self.current_screen = CurrentScreen::Indexes;
                if offset == 0 {
                    self.indexes_pagination.reset();
                }
            }
            Action::LoadClusterInfo => {
                self.current_screen = CurrentScreen::Cluster;
            }
            Action::ToggleClusterViewMode => {
                self.toggle_cluster_view_mode();
            }
            Action::LoadJobs { offset, .. } => {
                self.current_screen = CurrentScreen::Jobs;
                if offset == 0 {
                    self.jobs_pagination.reset();
                }
            }
            Action::LoadHealth => {
                self.current_screen = CurrentScreen::Health;
            }
            Action::LoadLicense => {
                self.current_screen = CurrentScreen::License;
            }
            Action::LoadKvstore => {
                self.current_screen = CurrentScreen::Kvstore;
            }
            Action::LoadSavedSearches => {
                self.current_screen = CurrentScreen::SavedSearches;
            }
            Action::LoadInternalLogs { .. } => {
                self.current_screen = CurrentScreen::InternalLogs;
            }
            Action::LoadApps { offset, .. } => {
                self.current_screen = CurrentScreen::Apps;
                if offset == 0 {
                    self.apps_pagination.reset();
                }
            }
            Action::LoadUsers { offset, .. } => {
                self.current_screen = CurrentScreen::Users;
                if offset == 0 {
                    self.users_pagination.reset();
                }
            }
            Action::LoadSearchPeers { offset, .. } => {
                self.current_screen = CurrentScreen::SearchPeers;
                if offset == 0 {
                    self.search_peers_pagination.reset();
                }
            }
            Action::LoadInputs { offset, .. } => {
                self.current_screen = CurrentScreen::Inputs;
                if offset == 0 {
                    self.inputs_pagination.reset();
                }
            }
            Action::LoadFiredAlerts { offset, .. } => {
                self.current_screen = CurrentScreen::FiredAlerts;
                if offset == 0 {
                    self.fired_alerts_pagination.reset();
                }
            }
            Action::LoadLookups { offset, .. } => {
                self.current_screen = CurrentScreen::Lookups;
                if offset == 0 {
                    self.lookups_pagination.reset();
                }
            }
            Action::LoadDashboards { offset, .. } => {
                self.current_screen = CurrentScreen::Dashboards;
                if offset == 0 {
                    self.dashboards_pagination.reset();
                }
            }
            Action::LoadDataModels { offset, .. } => {
                self.current_screen = CurrentScreen::DataModels;
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
                if offset == 0 {
                    self.workload_pools_pagination.reset();
                }
            }
            Action::LoadWorkloadRules { offset, .. } => {
                self.current_screen = CurrentScreen::WorkloadManagement;
                if offset == 0 {
                    self.workload_rules_pagination.reset();
                }
            }
            Action::LoadForwarders { offset, .. } => {
                self.current_screen = CurrentScreen::Forwarders;
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
