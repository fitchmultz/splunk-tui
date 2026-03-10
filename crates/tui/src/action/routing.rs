//! Action routing metadata for the TUI event system.
//!
//! Purpose:
//! - Centralize action classification used by reducers, startup gating, and side-effect dispatch.
//!
//! Responsibilities:
//! - Provide stable action type names for tracing.
//! - Group actions into reducer route families.
//! - Identify actions translated by the main loop before side-effect dispatch.
//! - Identify actions that are safe to run without an authenticated client.
//!
//! Scope:
//! - Action metadata only; this module does not mutate state or execute async work.
//!
//! Usage:
//! - Called from `App::update()`, startup/bootstrap gating, and the side-effect dispatcher.
//!
//! Invariants/Assumptions:
//! - Classification here must remain behavior-compatible with existing reducer and dispatcher flow.

use super::Action;

/// High-level reducer route for app state mutation.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AppActionRoute {
    Navigation,
    Search,
    Tutorial,
    Profile,
    System,
    Focus,
    Undo,
    DataLoading,
}

impl Action {
    /// Stable action name for tracing and diagnostics.
    pub fn type_name(&self) -> &'static str {
        match self {
            Action::LoadIndexes { .. } => "LoadIndexes",
            Action::LoadJobs { .. } => "LoadJobs",
            Action::LoadClusterInfo => "LoadClusterInfo",
            Action::LoadClusterPeers => "LoadClusterPeers",
            Action::SetMaintenanceMode { .. } => "SetMaintenanceMode",
            Action::RebalanceCluster => "RebalanceCluster",
            Action::DecommissionPeer { .. } => "DecommissionPeer",
            Action::RemovePeer { .. } => "RemovePeer",
            Action::LoadSavedSearches => "LoadSavedSearches",
            Action::LoadMacros => "LoadMacros",
            Action::CreateMacro { .. } => "CreateMacro",
            Action::UpdateMacro { .. } => "UpdateMacro",
            Action::DeleteMacro { .. } => "DeleteMacro",
            Action::UpdateSavedSearch { .. } => "UpdateSavedSearch",
            Action::CreateSavedSearch { .. } => "CreateSavedSearch",
            Action::DeleteSavedSearch { .. } => "DeleteSavedSearch",
            Action::ToggleSavedSearch { .. } => "ToggleSavedSearch",
            Action::LoadInternalLogs { .. } => "LoadInternalLogs",
            Action::LoadApps { .. } => "LoadApps",
            Action::LoadUsers { .. } => "LoadUsers",
            Action::LoadMoreIndexes => "LoadMoreIndexes",
            Action::LoadMoreJobs => "LoadMoreJobs",
            Action::LoadMoreApps => "LoadMoreApps",
            Action::LoadMoreUsers => "LoadMoreUsers",
            Action::LoadMoreInternalLogs => "LoadMoreInternalLogs",
            Action::LoadSearchPeers { .. } => "LoadSearchPeers",
            Action::LoadMoreSearchPeers => "LoadMoreSearchPeers",
            Action::LoadForwarders { .. } => "LoadForwarders",
            Action::LoadMoreForwarders => "LoadMoreForwarders",
            Action::LoadLookups { .. } => "LoadLookups",
            Action::LoadMoreLookups => "LoadMoreLookups",
            Action::DownloadLookup { .. } => "DownloadLookup",
            Action::DeleteLookup { .. } => "DeleteLookup",
            Action::LoadInputs { .. } => "LoadInputs",
            Action::LoadMoreInputs => "LoadMoreInputs",
            Action::LoadConfigFiles => "LoadConfigFiles",
            Action::LoadFiredAlerts { .. } => "LoadFiredAlerts",
            Action::LoadMoreFiredAlerts => "LoadMoreFiredAlerts",
            Action::LoadConfigStanzas { .. } => "LoadConfigStanzas",
            Action::EnableInput { .. } => "EnableInput",
            Action::DisableInput { .. } => "DisableInput",
            Action::SwitchToSettings => "SwitchToSettings",
            Action::RunSearch { .. } => "RunSearch",
            Action::LoadMoreSearchResults { .. } => "LoadMoreSearchResults",
            Action::ValidateSpl { .. } => "ValidateSpl",
            Action::CancelJob(_) => "CancelJob",
            Action::DeleteJob(_) => "DeleteJob",
            Action::CancelJobsBatch(_) => "CancelJobsBatch",
            Action::DeleteJobsBatch(_) => "DeleteJobsBatch",
            Action::EnableApp(_) => "EnableApp",
            Action::DisableApp(_) => "DisableApp",
            Action::InstallApp { .. } => "InstallApp",
            Action::RemoveApp { .. } => "RemoveApp",
            Action::LoadHealth => "LoadHealth",
            Action::RunConnectionDiagnostics => "RunConnectionDiagnostics",
            Action::ConnectionDiagnosticsLoaded(_) => "ConnectionDiagnosticsLoaded",
            Action::LoadLicense => "LoadLicense",
            Action::LoadKvstore => "LoadKvstore",
            Action::LoadOverview => "LoadOverview",
            Action::LoadMultiInstanceOverview => "LoadMultiInstanceOverview",
            Action::RetryInstance(_) => "RetryInstance",
            Action::ExportData(_, _, _) => "ExportData",
            Action::OpenProfileSwitcher => "OpenProfileSwitcher",
            Action::ProfileSelected(_) => "ProfileSelected",
            Action::CreateIndex { .. } => "CreateIndex",
            Action::ModifyIndex { .. } => "ModifyIndex",
            Action::DeleteIndex { .. } => "DeleteIndex",
            Action::CreateUser { .. } => "CreateUser",
            Action::ModifyUser { .. } => "ModifyUser",
            Action::DeleteUser { .. } => "DeleteUser",
            Action::LoadRoles { .. } => "LoadRoles",
            Action::LoadCapabilities => "LoadCapabilities",
            Action::CreateRole { .. } => "CreateRole",
            Action::ModifyRole { .. } => "ModifyRole",
            Action::DeleteRole { .. } => "DeleteRole",
            Action::InstallLicense { .. } => "InstallLicense",
            Action::CreateLicensePool { .. } => "CreateLicensePool",
            Action::ModifyLicensePool { .. } => "ModifyLicensePool",
            Action::DeleteLicensePool { .. } => "DeleteLicensePool",
            Action::ActivateLicense { .. } => "ActivateLicense",
            Action::DeactivateLicense { .. } => "DeactivateLicense",
            Action::OpenEditProfileDialog { .. } => "OpenEditProfileDialog",
            Action::SaveProfile { .. } => "SaveProfile",
            Action::DeleteProfile { .. } => "DeleteProfile",
            Action::LoadAuditEvents { .. } => "LoadAuditEvents",
            Action::LoadRecentAuditEvents { .. } => "LoadRecentAuditEvents",
            Action::LoadDashboards { .. } => "LoadDashboards",
            Action::LoadMoreDashboards => "LoadMoreDashboards",
            Action::LoadDataModels { .. } => "LoadDataModels",
            Action::LoadMoreDataModels => "LoadMoreDataModels",
            Action::RefreshIndexes => "RefreshIndexes",
            Action::RefreshJobs => "RefreshJobs",
            Action::RefreshApps => "RefreshApps",
            Action::RefreshUsers => "RefreshUsers",
            Action::RefreshInternalLogs => "RefreshInternalLogs",
            Action::RefreshDashboards => "RefreshDashboards",
            Action::RefreshDataModels => "RefreshDataModels",
            Action::RefreshInputs => "RefreshInputs",
            Action::LoadWorkloadPools { .. } => "LoadWorkloadPools",
            Action::LoadMoreWorkloadPools => "LoadMoreWorkloadPools",
            Action::LoadWorkloadRules { .. } => "LoadWorkloadRules",
            Action::LoadMoreWorkloadRules => "LoadMoreWorkloadRules",
            Action::LoadShcStatus => "LoadShcStatus",
            Action::LoadShcMembers => "LoadShcMembers",
            Action::LoadShcCaptain => "LoadShcCaptain",
            Action::LoadShcConfig => "LoadShcConfig",
            Action::AddShcMember { .. } => "AddShcMember",
            Action::RemoveShcMember { .. } => "RemoveShcMember",
            Action::RollingRestartShc { .. } => "RollingRestartShc",
            Action::SetShcCaptain { .. } => "SetShcCaptain",
            Action::Input(_) => "Input",
            Action::Mouse(_) => "Mouse",
            Action::Resize(_, _) => "Resize",
            Action::Tick => "Tick",
            Action::Quit => "Quit",
            Action::Loading(_) => "Loading",
            Action::PersistState => "PersistState",
            _ => "Other",
        }
    }

    /// Actions translated by the main loop before side-effect dispatch.
    pub fn is_main_loop_translated(&self) -> bool {
        matches!(
            self,
            Action::LoadMoreIndexes
                | Action::LoadMoreJobs
                | Action::LoadMoreApps
                | Action::LoadMoreUsers
                | Action::LoadMoreRoles
                | Action::LoadMoreInternalLogs
                | Action::LoadMoreSearchPeers
                | Action::LoadMoreForwarders
                | Action::LoadMoreLookups
                | Action::LoadMoreInputs
                | Action::LoadMoreFiredAlerts
                | Action::LoadMoreDashboards
                | Action::LoadMoreDataModels
                | Action::RefreshIndexes
                | Action::RefreshJobs
                | Action::RefreshApps
                | Action::RefreshUsers
                | Action::RefreshRoles
                | Action::RefreshInternalLogs
                | Action::RefreshDashboards
                | Action::RefreshDataModels
                | Action::RefreshInputs
                | Action::LoadMoreWorkloadPools
                | Action::LoadMoreWorkloadRules
        )
    }

    /// Whether this action can be processed while no authenticated client exists.
    pub fn requires_client(&self) -> bool {
        !matches!(
            self,
            Action::Quit
                | Action::Tick
                | Action::Input(_)
                | Action::Mouse(_)
                | Action::Resize(_, _)
                | Action::Loading(_)
                | Action::Notify(_, _)
                | Action::StartTutorial { .. }
                | Action::TutorialProfileCreated { .. }
                | Action::TutorialConnectionResult { .. }
                | Action::TutorialCompleted
                | Action::TutorialSkipped
                | Action::LoadSearchScreenForTutorial
                | Action::OpenCreateProfileDialog { .. }
                | Action::OpenEditProfileDialog { .. }
                | Action::OpenEditProfileDialogWithData { .. }
                | Action::OpenDeleteProfileConfirm { .. }
                | Action::SaveProfile { .. }
                | Action::DeleteProfile { .. }
                | Action::ProfileSaved(_)
                | Action::ProfileDeleted(_)
                | Action::SwitchToSettingsScreen
                | Action::SwitchToSettings
                | Action::SettingsLoaded(_)
                | Action::CycleTheme
                | Action::NextScreen
                | Action::PreviousScreen
                | Action::SwitchToSearch
                | Action::OpenCommandPalette
                | Action::OpenHelpPopup
                | Action::SetFocus(_)
                | Action::NextFocus
                | Action::PreviousFocus
                | Action::ToggleFocusMode
                | Action::PersistState
                | Action::ShowErrorDetails(_)
                | Action::ShowErrorDetailsFromCurrent
                | Action::ClearErrorDetails
                | Action::Progress(_)
                | Action::CopyToClipboard(_)
        )
    }

    /// Reducer route classification for app state updates.
    pub fn app_route(&self) -> AppActionRoute {
        match self {
            Action::OpenHelpPopup
            | Action::OpenCommandPalette
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
            | Action::LoadShcStatus
            | Action::LoadShcMembers
            | Action::LoadShcCaptain
            | Action::LoadShcConfig
            | Action::ToggleShcViewMode
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
            | Action::ExitInspectMode => AppActionRoute::Navigation,

            Action::SearchStarted(_)
            | Action::SearchComplete(_)
            | Action::MoreSearchResultsLoaded(_) => AppActionRoute::Search,

            Action::StartTutorial { .. }
            | Action::TutorialCompleted
            | Action::TutorialSkipped
            | Action::TutorialProfileCreated { .. }
            | Action::TutorialConnectionResult { .. }
            | Action::LoadSearchScreenForTutorial
            | Action::OpenCreateProfileDialog {
                from_tutorial: true,
            } => AppActionRoute::Tutorial,

            Action::OpenProfileSwitcher
            | Action::OpenProfileSelectorWithList(_)
            | Action::ProfileSelected(_)
            | Action::ProfileSwitchResult(_)
            | Action::ClearAllData
            | Action::OpenCreateProfileDialog { .. }
            | Action::OpenEditProfileDialogWithData { .. }
            | Action::OpenDeleteProfileConfirm { .. }
            | Action::ProfileSaved(_)
            | Action::ProfileDeleted(_) => AppActionRoute::Profile,

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
            | Action::OpenModifyIndexDialog { .. }
            | Action::OpenDeleteIndexConfirm { .. }
            | Action::OpenCreateUserDialog
            | Action::OpenModifyUserDialog { .. }
            | Action::OpenDeleteUserConfirm { .. }
            | Action::OpenCreateRoleDialog
            | Action::OpenModifyRoleDialog { .. }
            | Action::OpenDeleteRoleConfirm { .. }
            | Action::EditSavedSearch
            | Action::SavedSearchUpdated(_)
            | Action::OpenCreateSavedSearchDialog
            | Action::OpenDeleteSavedSearchConfirm { .. }
            | Action::SavedSearchCreated(_)
            | Action::SavedSearchDeleted(_)
            | Action::SavedSearchToggled(_)
            | Action::MaintenanceModeSet { .. }
            | Action::ClusterRebalanced { .. }
            | Action::PeerDecommissioned { .. }
            | Action::PeerRemoved { .. }
            | Action::OpenDeleteLookupConfirm { .. }
            | Action::LookupDownloaded(_)
            | Action::LookupDeleted(_)
            | Action::ExportSuccess(_)
            | Action::ConnectionDiagnosticsLoaded(_)
            | Action::DismissOnboardingItem
            | Action::DismissOnboardingAll
            | Action::ShcMemberAdded { .. }
            | Action::ShcMemberRemoved { .. }
            | Action::ShcRollingRestarted { .. }
            | Action::ShcCaptainSet { .. } => AppActionRoute::System,

            Action::NextFocus
            | Action::PreviousFocus
            | Action::SetFocus(_)
            | Action::ToggleFocusMode => AppActionRoute::Focus,

            Action::QueueUndoableOperation { .. }
            | Action::Undo
            | Action::Redo
            | Action::ExecutePendingOperation { .. }
            | Action::OperationUndone { .. }
            | Action::OperationRedone { .. }
            | Action::ShowUndoHistory => AppActionRoute::Undo,

            _ => AppActionRoute::DataLoading,
        }
    }
}
