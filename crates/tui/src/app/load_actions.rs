//! Load-action normalization for the TUI app.
//!
//! Purpose:
//! - Centralize screen load construction, pagination continuation, and main-loop translation.
//!
//! Responsibilities:
//! - Build initial load actions for each screen.
//! - Build follow-up paginated load actions from explicit `LoadMore*` triggers.
//! - Translate `Refresh*` and `LoadMore*` actions into concrete `Load*` requests.
//!
//! Scope:
//! - Action construction only; this module does not mutate state or execute side effects.
//!
//! Usage:
//! - Called by the main loop after input dispatch and by screen navigation reload flow.
//!
//! Invariants/Assumptions:
//! - Translation must key off the action variant itself instead of ambient screen state.

use crate::action::Action;
use crate::app::App;
use crate::app::state::{CurrentScreen, ListPaginationState, WorkloadViewMode};

impl App {
    fn paged_load_action(
        &self,
        pagination: &ListPaginationState,
        build: impl FnOnce(usize, usize) -> Action,
    ) -> Action {
        build(pagination.page_size, 0)
    }

    fn paged_load_more_action(
        &self,
        pagination: &ListPaginationState,
        build: impl FnOnce(usize, usize) -> Action,
    ) -> Option<Action> {
        pagination
            .can_load_more()
            .then(|| build(pagination.page_size, pagination.current_offset))
    }

    fn internal_logs_action(&self) -> Action {
        Action::LoadInternalLogs {
            count: self.internal_logs_defaults.count,
            earliest: self.internal_logs_defaults.earliest_time.clone(),
        }
    }

    fn workload_initial_action(&self, view_mode: WorkloadViewMode) -> Action {
        match view_mode {
            WorkloadViewMode::Pools => self
                .paged_load_action(&self.workload_pools_pagination, |count, offset| {
                    Action::LoadWorkloadPools { count, offset }
                }),
            WorkloadViewMode::Rules => self
                .paged_load_action(&self.workload_rules_pagination, |count, offset| {
                    Action::LoadWorkloadRules { count, offset }
                }),
        }
    }

    fn workload_load_more_action(&self, view_mode: WorkloadViewMode) -> Option<Action> {
        match view_mode {
            WorkloadViewMode::Pools => self
                .paged_load_more_action(&self.workload_pools_pagination, |count, offset| {
                    Action::LoadWorkloadPools { count, offset }
                }),
            WorkloadViewMode::Rules => self
                .paged_load_more_action(&self.workload_rules_pagination, |count, offset| {
                    Action::LoadWorkloadRules { count, offset }
                }),
        }
    }

    fn translated_load_more_action(&self, action: &Action) -> Option<Action> {
        match action {
            Action::LoadMoreIndexes => self
                .paged_load_more_action(&self.indexes_pagination, |count, offset| {
                    Action::LoadIndexes { count, offset }
                }),
            Action::LoadMoreJobs => {
                self.paged_load_more_action(&self.jobs_pagination, |count, offset| {
                    Action::LoadJobs { count, offset }
                })
            }
            Action::LoadMoreApps => {
                self.paged_load_more_action(&self.apps_pagination, |count, offset| {
                    Action::LoadApps { count, offset }
                })
            }
            Action::LoadMoreUsers => self
                .paged_load_more_action(&self.users_pagination, |count, offset| {
                    Action::LoadUsers { count, offset }
                }),
            Action::LoadMoreRoles => self
                .paged_load_more_action(&self.roles_pagination, |count, offset| {
                    Action::LoadRoles { count, offset }
                }),
            Action::LoadMoreInternalLogs => Some(self.internal_logs_action()),
            Action::LoadMoreSearchPeers => self
                .paged_load_more_action(&self.search_peers_pagination, |count, offset| {
                    Action::LoadSearchPeers { count, offset }
                }),
            Action::LoadMoreForwarders => self
                .paged_load_more_action(&self.forwarders_pagination, |count, offset| {
                    Action::LoadForwarders { count, offset }
                }),
            Action::LoadMoreLookups => self
                .paged_load_more_action(&self.lookups_pagination, |count, offset| {
                    Action::LoadLookups { count, offset }
                }),
            Action::LoadMoreInputs => self
                .paged_load_more_action(&self.inputs_pagination, |count, offset| {
                    Action::LoadInputs { count, offset }
                }),
            Action::LoadMoreFiredAlerts => self
                .paged_load_more_action(&self.fired_alerts_pagination, |count, offset| {
                    Action::LoadFiredAlerts { count, offset }
                }),
            Action::LoadMoreDashboards => self
                .paged_load_more_action(&self.dashboards_pagination, |count, offset| {
                    Action::LoadDashboards { count, offset }
                }),
            Action::LoadMoreDataModels => self
                .paged_load_more_action(&self.data_models_pagination, |count, offset| {
                    Action::LoadDataModels { count, offset }
                }),
            Action::LoadMoreWorkloadPools => {
                self.workload_load_more_action(WorkloadViewMode::Pools)
            }
            Action::LoadMoreWorkloadRules => {
                self.workload_load_more_action(WorkloadViewMode::Rules)
            }
            _ => None,
        }
    }

    fn translated_refresh_action(&self, action: &Action) -> Option<Action> {
        match action {
            Action::RefreshIndexes => Some(
                self.paged_load_action(&self.indexes_pagination, |count, offset| {
                    Action::LoadIndexes { count, offset }
                }),
            ),
            Action::RefreshJobs => Some(self.paged_load_action(
                &self.jobs_pagination,
                |count, offset| Action::LoadJobs { count, offset },
            )),
            Action::RefreshApps => Some(self.paged_load_action(
                &self.apps_pagination,
                |count, offset| Action::LoadApps { count, offset },
            )),
            Action::RefreshUsers => Some(self.paged_load_action(
                &self.users_pagination,
                |count, offset| Action::LoadUsers { count, offset },
            )),
            Action::RefreshRoles => Some(self.paged_load_action(
                &self.roles_pagination,
                |count, offset| Action::LoadRoles { count, offset },
            )),
            Action::RefreshInternalLogs => Some(self.internal_logs_action()),
            Action::RefreshDashboards => Some(
                self.paged_load_action(&self.dashboards_pagination, |count, offset| {
                    Action::LoadDashboards { count, offset }
                }),
            ),
            Action::RefreshDataModels => Some(
                self.paged_load_action(&self.data_models_pagination, |count, offset| {
                    Action::LoadDataModels { count, offset }
                }),
            ),
            Action::RefreshInputs => Some(
                self.paged_load_action(&self.inputs_pagination, |count, offset| {
                    Action::LoadInputs { count, offset }
                }),
            ),
            _ => None,
        }
    }

    /// Returns the load action for the current screen, if one is needed.
    /// Used after screen navigation to trigger data loading.
    pub fn load_action_for_screen(&self) -> Option<Action> {
        match self.current_screen {
            CurrentScreen::Search => None,
            CurrentScreen::Indexes => Some(
                self.paged_load_action(&self.indexes_pagination, |count, offset| {
                    Action::LoadIndexes { count, offset }
                }),
            ),
            CurrentScreen::Cluster => Some(Action::LoadClusterInfo),
            CurrentScreen::Jobs => Some(self.paged_load_action(
                &self.jobs_pagination,
                |count, offset| Action::LoadJobs { count, offset },
            )),
            CurrentScreen::JobInspect => None,
            CurrentScreen::Health => Some(Action::LoadHealth),
            CurrentScreen::License => Some(Action::LoadLicense),
            CurrentScreen::Kvstore => Some(Action::LoadKvstore),
            CurrentScreen::SavedSearches => Some(Action::LoadSavedSearches),
            CurrentScreen::Macros => Some(Action::LoadMacros),
            CurrentScreen::InternalLogs => Some(self.internal_logs_action()),
            CurrentScreen::Apps => Some(self.paged_load_action(
                &self.apps_pagination,
                |count, offset| Action::LoadApps { count, offset },
            )),
            CurrentScreen::Users => Some(self.paged_load_action(
                &self.users_pagination,
                |count, offset| Action::LoadUsers { count, offset },
            )),
            CurrentScreen::Roles => Some(self.paged_load_action(
                &self.roles_pagination,
                |count, offset| Action::LoadRoles { count, offset },
            )),
            CurrentScreen::SearchPeers => Some(
                self.paged_load_action(&self.search_peers_pagination, |count, offset| {
                    Action::LoadSearchPeers { count, offset }
                }),
            ),
            CurrentScreen::Inputs => Some(
                self.paged_load_action(&self.inputs_pagination, |count, offset| {
                    Action::LoadInputs { count, offset }
                }),
            ),
            CurrentScreen::Configs => Some(Action::LoadConfigFiles),
            CurrentScreen::FiredAlerts => Some(
                self.paged_load_action(&self.fired_alerts_pagination, |count, offset| {
                    Action::LoadFiredAlerts { count, offset }
                }),
            ),
            CurrentScreen::Forwarders => Some(
                self.paged_load_action(&self.forwarders_pagination, |count, offset| {
                    Action::LoadForwarders { count, offset }
                }),
            ),
            CurrentScreen::Lookups => Some(
                self.paged_load_action(&self.lookups_pagination, |count, offset| {
                    Action::LoadLookups { count, offset }
                }),
            ),
            CurrentScreen::Audit => Some(Action::LoadAuditEvents {
                count: 50,
                offset: 0,
                earliest: "-24h".to_string(),
                latest: "now".to_string(),
            }),
            CurrentScreen::Dashboards => Some(
                self.paged_load_action(&self.dashboards_pagination, |count, offset| {
                    Action::LoadDashboards { count, offset }
                }),
            ),
            CurrentScreen::DataModels => Some(
                self.paged_load_action(&self.data_models_pagination, |count, offset| {
                    Action::LoadDataModels { count, offset }
                }),
            ),
            CurrentScreen::WorkloadManagement => {
                Some(self.workload_initial_action(self.workload_view_mode))
            }
            CurrentScreen::Shc => (!self.shc_unavailable).then_some(Action::LoadShcStatus),
            CurrentScreen::Settings => Some(Action::SwitchToSettings),
            CurrentScreen::Overview => Some(Action::LoadOverview),
            CurrentScreen::MultiInstance => Some(Action::LoadMultiInstanceOverview),
        }
    }

    /// Returns a load-more action for the current screen if pagination is available.
    pub fn load_more_action_for_current_screen(&self) -> Option<Action> {
        match self.current_screen {
            CurrentScreen::Indexes => self.translated_load_more_action(&Action::LoadMoreIndexes),
            CurrentScreen::Jobs => self.translated_load_more_action(&Action::LoadMoreJobs),
            CurrentScreen::Apps => self.translated_load_more_action(&Action::LoadMoreApps),
            CurrentScreen::Users => self.translated_load_more_action(&Action::LoadMoreUsers),
            CurrentScreen::Roles => self.translated_load_more_action(&Action::LoadMoreRoles),
            CurrentScreen::SearchPeers => {
                self.translated_load_more_action(&Action::LoadMoreSearchPeers)
            }
            CurrentScreen::Forwarders => {
                self.translated_load_more_action(&Action::LoadMoreForwarders)
            }
            CurrentScreen::Lookups => self.translated_load_more_action(&Action::LoadMoreLookups),
            CurrentScreen::Inputs => self.translated_load_more_action(&Action::LoadMoreInputs),
            CurrentScreen::FiredAlerts => {
                self.translated_load_more_action(&Action::LoadMoreFiredAlerts)
            }
            CurrentScreen::Dashboards => {
                self.translated_load_more_action(&Action::LoadMoreDashboards)
            }
            CurrentScreen::DataModels => {
                self.translated_load_more_action(&Action::LoadMoreDataModels)
            }
            CurrentScreen::WorkloadManagement => {
                self.workload_load_more_action(self.workload_view_mode)
            }
            _ => None,
        }
    }

    /// Translate a `LoadMore*` action into a concrete `Load*` request.
    pub fn translate_load_more_action(&self, action: Action) -> Action {
        self.translated_load_more_action(&action).unwrap_or(action)
    }

    /// Translate a `Refresh*` action into a concrete `Load*` request with offset zero.
    pub fn translate_refresh_action(&self, action: Action) -> Action {
        self.translated_refresh_action(&action).unwrap_or(action)
    }

    /// Normalize main-loop-only actions before reducer and side-effect dispatch.
    pub fn translate_main_loop_action(&self, action: Action) -> Action {
        if let Some(translated) = self.translated_load_more_action(&action) {
            return translated;
        }

        if let Some(translated) = self.translated_refresh_action(&action) {
            return translated;
        }

        action
    }
}
