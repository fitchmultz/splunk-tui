//! Application state types and enums.
//!
//! Responsibilities:
//! - Define screen navigation enum (CurrentScreen)
//! - Define health state enum (HealthState)
//! - Define sorting types (SortColumn, SortDirection, SortState)
//!
//! Does NOT handle:
//! - Does NOT handle state mutations (in App impl)
//! - Does NOT define the main App struct

/// Health state of the Splunk instance.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum HealthState {
    /// Health status is unknown (initial state or check pending)
    Unknown,
    /// Splunk is healthy
    Healthy,
    /// Splunk is unhealthy
    Unhealthy,
}

impl HealthState {
    /// Map Splunk health string to HealthState.
    ///
    /// Splunk returns "green", "yellow", or "red" for health status.
    /// - "green" → Healthy
    /// - "yellow" → Unknown (degraded but not failed)
    /// - "red" → Unhealthy
    /// - any other value → Unknown
    pub fn from_health_str(health: &str) -> Self {
        match health.to_lowercase().as_str() {
            "green" => HealthState::Healthy,
            "red" => HealthState::Unhealthy,
            _ => HealthState::Unknown,
        }
    }
}

/// Layout constants for UI components.
/// Header height increased to 4 to accommodate connection context line (RQ-0134)
pub const HEADER_HEIGHT: u16 = 4;
pub const FOOTER_HEIGHT: u16 = 3;

/// Current active screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum CurrentScreen {
    Search,
    Indexes,
    Cluster,
    Jobs,
    JobInspect,
    Health,
    License,
    Kvstore,
    SavedSearches,
    Macros,
    InternalLogs,
    Apps,
    Users,
    Roles,
    SearchPeers,
    Inputs,
    Configs,
    Settings,
    Overview,
    MultiInstance,
    FiredAlerts,
    Forwarders,
    Lookups,
    Audit,
    Dashboards,
    DataModels,
    WorkloadManagement,
    Shc,
}

impl CurrentScreen {
    /// Returns the next screen in cyclic navigation order.
    /// Excludes JobInspect from the cycle (it's only accessible via InspectJob action).
    pub fn next(self) -> Self {
        match self {
            CurrentScreen::Search => CurrentScreen::Indexes,
            CurrentScreen::Indexes => CurrentScreen::Cluster,
            CurrentScreen::Cluster => CurrentScreen::Jobs,
            CurrentScreen::Jobs => CurrentScreen::Health,
            CurrentScreen::JobInspect => CurrentScreen::Jobs, // Special case: return to Jobs
            CurrentScreen::Health => CurrentScreen::License,
            CurrentScreen::License => CurrentScreen::Kvstore,
            CurrentScreen::Kvstore => CurrentScreen::SavedSearches,
            CurrentScreen::SavedSearches => CurrentScreen::Macros,
            CurrentScreen::Macros => CurrentScreen::InternalLogs,
            CurrentScreen::InternalLogs => CurrentScreen::Apps,
            CurrentScreen::Apps => CurrentScreen::Users,
            CurrentScreen::Users => CurrentScreen::Roles,
            CurrentScreen::Roles => CurrentScreen::SearchPeers,
            CurrentScreen::SearchPeers => CurrentScreen::Inputs,
            CurrentScreen::Inputs => CurrentScreen::Configs,
            CurrentScreen::Configs => CurrentScreen::FiredAlerts,
            CurrentScreen::FiredAlerts => CurrentScreen::Forwarders,
            CurrentScreen::Forwarders => CurrentScreen::Lookups,
            CurrentScreen::Lookups => CurrentScreen::Audit,
            CurrentScreen::Audit => CurrentScreen::Dashboards,
            CurrentScreen::Dashboards => CurrentScreen::DataModels,
            CurrentScreen::DataModels => CurrentScreen::WorkloadManagement,
            CurrentScreen::WorkloadManagement => CurrentScreen::Shc,
            CurrentScreen::Shc => CurrentScreen::Settings,
            CurrentScreen::Settings => CurrentScreen::Overview,
            CurrentScreen::Overview => CurrentScreen::MultiInstance,
            CurrentScreen::MultiInstance => CurrentScreen::Search, // Wrap around
        }
    }

    /// Returns the previous screen in cyclic navigation order.
    /// Excludes JobInspect from the cycle (it's only accessible via InspectJob action).
    pub fn previous(self) -> Self {
        match self {
            CurrentScreen::Search => CurrentScreen::MultiInstance, // Wrap around
            CurrentScreen::Indexes => CurrentScreen::Search,
            CurrentScreen::Cluster => CurrentScreen::Indexes,
            CurrentScreen::Jobs => CurrentScreen::Cluster,
            CurrentScreen::JobInspect => CurrentScreen::Jobs, // Special case: return to Jobs
            CurrentScreen::Health => CurrentScreen::Jobs,
            CurrentScreen::License => CurrentScreen::Health,
            CurrentScreen::Kvstore => CurrentScreen::License,
            CurrentScreen::SavedSearches => CurrentScreen::Kvstore,
            CurrentScreen::Macros => CurrentScreen::SavedSearches,
            CurrentScreen::InternalLogs => CurrentScreen::Macros,
            CurrentScreen::Apps => CurrentScreen::InternalLogs,
            CurrentScreen::Users => CurrentScreen::Apps,
            CurrentScreen::Roles => CurrentScreen::Users,
            CurrentScreen::SearchPeers => CurrentScreen::Roles,
            CurrentScreen::Inputs => CurrentScreen::SearchPeers,
            CurrentScreen::Configs => CurrentScreen::Inputs,
            CurrentScreen::FiredAlerts => CurrentScreen::Configs,
            CurrentScreen::Forwarders => CurrentScreen::FiredAlerts,
            CurrentScreen::Settings => CurrentScreen::Shc,
            CurrentScreen::Shc => CurrentScreen::WorkloadManagement,
            CurrentScreen::WorkloadManagement => CurrentScreen::DataModels,
            CurrentScreen::DataModels => CurrentScreen::Dashboards,
            CurrentScreen::Dashboards => CurrentScreen::Audit,
            CurrentScreen::Audit => CurrentScreen::Lookups,
            CurrentScreen::Lookups => CurrentScreen::Forwarders,
            CurrentScreen::Overview => CurrentScreen::Settings,
            CurrentScreen::MultiInstance => CurrentScreen::Overview,
        }
    }

    /// Returns the screen name as a string for serialization.
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Search => "Search",
            Self::Indexes => "Indexes",
            Self::Cluster => "Cluster",
            Self::Jobs => "Jobs",
            Self::JobInspect => "JobInspect",
            Self::Health => "Health",
            Self::License => "License",
            Self::Kvstore => "Kvstore",
            Self::SavedSearches => "SavedSearches",
            Self::Macros => "Macros",
            Self::InternalLogs => "InternalLogs",
            Self::Apps => "Apps",
            Self::Users => "Users",
            Self::Roles => "Roles",
            Self::SearchPeers => "SearchPeers",
            Self::Inputs => "Inputs",
            Self::Configs => "Configs",
            Self::Settings => "Settings",
            Self::Overview => "Overview",
            Self::MultiInstance => "MultiInstance",
            Self::FiredAlerts => "FiredAlerts",
            Self::Forwarders => "Forwarders",
            Self::Lookups => "Lookups",
            Self::Audit => "Audit",
            Self::Dashboards => "Dashboards",
            Self::DataModels => "DataModels",
            Self::WorkloadManagement => "WorkloadManagement",
            Self::Shc => "Shc",
        }
    }
}

/// Parse current screen from string (for deserialization).
pub fn parse_current_screen(s: &str) -> CurrentScreen {
    match s {
        "Search" => CurrentScreen::Search,
        "Indexes" => CurrentScreen::Indexes,
        "Cluster" => CurrentScreen::Cluster,
        "Jobs" => CurrentScreen::Jobs,
        "JobInspect" => CurrentScreen::JobInspect,
        "Health" => CurrentScreen::Health,
        "License" => CurrentScreen::License,
        "Kvstore" => CurrentScreen::Kvstore,
        "SavedSearches" => CurrentScreen::SavedSearches,
        "Macros" => CurrentScreen::Macros,
        "InternalLogs" => CurrentScreen::InternalLogs,
        "Apps" => CurrentScreen::Apps,
        "Users" => CurrentScreen::Users,
        "Roles" => CurrentScreen::Roles,
        "SearchPeers" => CurrentScreen::SearchPeers,
        "Inputs" => CurrentScreen::Inputs,
        "Configs" => CurrentScreen::Configs,
        "Settings" => CurrentScreen::Settings,
        "Overview" => CurrentScreen::Overview,
        "MultiInstance" => CurrentScreen::MultiInstance,
        "FiredAlerts" => CurrentScreen::FiredAlerts,
        "Forwarders" => CurrentScreen::Forwarders,
        "Lookups" => CurrentScreen::Lookups,
        "Audit" => CurrentScreen::Audit,
        "Dashboards" => CurrentScreen::Dashboards,
        "DataModels" => CurrentScreen::DataModels,
        "WorkloadManagement" => CurrentScreen::WorkloadManagement,
        "Shc" => CurrentScreen::Shc,
        _ => CurrentScreen::Search, // Default fallback
    }
}

/// Sort column for jobs table.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortColumn {
    Sid,
    Status,
    Duration,
    Results,
    Events,
}

impl SortColumn {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Sid => "sid",
            Self::Status => "status",
            Self::Duration => "duration",
            Self::Results => "results",
            Self::Events => "events",
        }
    }

    /// Returns the next column in the cycle.
    pub fn next(self) -> Self {
        match self {
            Self::Sid => Self::Status,
            Self::Status => Self::Duration,
            Self::Duration => Self::Results,
            Self::Results => Self::Events,
            Self::Events => Self::Sid,
        }
    }
}

/// Parse sort column from string (for deserialization).
pub fn parse_sort_column(s: &str) -> SortColumn {
    match s.to_lowercase().as_str() {
        "sid" => SortColumn::Sid,
        "status" => SortColumn::Status,
        "duration" => SortColumn::Duration,
        "results" => SortColumn::Results,
        "events" => SortColumn::Events,
        _ => SortColumn::Sid, // Default fallback
    }
}

/// Sort direction.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Asc => "asc",
            Self::Desc => "desc",
        }
    }

    /// Toggle between ascending and descending.
    pub fn toggle(self) -> Self {
        match self {
            Self::Asc => Self::Desc,
            Self::Desc => Self::Asc,
        }
    }
}

/// Parse sort direction from string (for deserialization).
pub fn parse_sort_direction(s: &str) -> SortDirection {
    match s.to_lowercase().as_str() {
        "asc" => SortDirection::Asc,
        "desc" => SortDirection::Desc,
        _ => SortDirection::Asc, // Default fallback
    }
}

/// Sort state for jobs table.
#[derive(Debug, Clone, Copy)]
pub struct SortState {
    pub column: SortColumn,
    pub direction: SortDirection,
}

/// View mode for the cluster screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ClusterViewMode {
    /// Show cluster summary information.
    #[default]
    Summary,
    /// Show cluster peers list.
    Peers,
}

impl ClusterViewMode {
    /// Toggle between summary and peers view.
    pub fn toggle(self) -> Self {
        match self {
            Self::Summary => Self::Peers,
            Self::Peers => Self::Summary,
        }
    }
}

/// View mode for the workload management screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum WorkloadViewMode {
    /// Show workload pools.
    #[default]
    Pools,
    /// Show workload rules.
    Rules,
}

impl WorkloadViewMode {
    /// Toggle between pools and rules view.
    pub fn toggle(self) -> Self {
        match self {
            Self::Pools => Self::Rules,
            Self::Rules => Self::Pools,
        }
    }
}

/// View mode for the SHC screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum ShcViewMode {
    /// Show SHC summary information.
    #[default]
    Summary,
    /// Show SHC members list.
    Members,
}

impl ShcViewMode {
    /// Toggle between summary and members view.
    pub fn toggle(self) -> Self {
        match self {
            Self::Summary => Self::Members,
            Self::Members => Self::Summary,
        }
    }
}

/// Input mode for the search screen.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SearchInputMode {
    /// Query input box is focused; printable characters insert into the query.
    #[default]
    QueryFocused,
    /// Results area is focused; navigation keys work on results.
    ResultsFocused,
}

impl SearchInputMode {
    /// Toggle between query and results focus modes.
    pub fn toggle(self) -> Self {
        match self {
            Self::QueryFocused => Self::ResultsFocused,
            Self::ResultsFocused => Self::QueryFocused,
        }
    }
}

/// Pagination state for list screens (indexes, jobs, apps, users).
#[derive(Debug, Clone, Copy)]
pub struct ListPaginationState {
    /// Number of items per page.
    pub page_size: usize,
    /// Current offset (number of items already loaded).
    pub current_offset: usize,
    /// Whether more items may be available.
    pub has_more: bool,
    /// Total number of items loaded so far.
    pub total_loaded: usize,
    /// Maximum number of items to load (safety cap).
    pub max_items: usize,
    /// Whether a load operation is currently in progress (prevents duplicate requests).
    pub is_loading: bool,
}

impl ListPaginationState {
    /// Create new pagination state with the given page size and max items cap.
    pub fn new(page_size: usize, max_items: usize) -> Self {
        Self {
            page_size,
            current_offset: 0,
            has_more: false,
            total_loaded: 0,
            max_items,
            is_loading: false,
        }
    }

    /// Check if more items can be loaded without exceeding max_items cap.
    /// Returns false if a load is already in progress (prevents race conditions).
    pub fn can_load_more(&self) -> bool {
        !self.is_loading && self.has_more && self.total_loaded < self.max_items
    }

    /// Reset pagination state (e.g., on refresh).
    pub fn reset(&mut self) {
        self.current_offset = 0;
        self.has_more = false;
        self.total_loaded = 0;
        self.is_loading = false;
    }

    /// Mark that a load operation is starting.
    pub fn start_loading(&mut self) {
        self.is_loading = true;
    }

    /// Mark that a load operation has completed (success or failure).
    pub fn finish_loading(&mut self) {
        self.is_loading = false;
    }

    /// Update state after loading items.
    pub fn update_loaded(&mut self, count: usize) {
        self.total_loaded += count;
        self.current_offset = self.total_loaded;
        // If we got a full page, there might be more
        self.has_more = count >= self.page_size;
        self.is_loading = false;
    }

    /// Mark that there are no more items.
    pub fn mark_complete(&mut self) {
        self.has_more = false;
    }
}

impl Default for SortState {
    fn default() -> Self {
        Self::new()
    }
}

impl SortState {
    pub fn new() -> Self {
        Self {
            column: SortColumn::Sid,
            direction: SortDirection::Asc,
        }
    }

    pub fn cycle(&mut self) {
        self.column = match self.column {
            SortColumn::Sid => SortColumn::Status,
            SortColumn::Status => SortColumn::Duration,
            SortColumn::Duration => SortColumn::Results,
            SortColumn::Results => SortColumn::Events,
            SortColumn::Events => SortColumn::Sid,
        };
    }

    pub fn toggle_direction(&mut self) {
        self.direction = match self.direction {
            SortDirection::Asc => SortDirection::Desc,
            SortDirection::Desc => SortDirection::Asc,
        };
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_health_state_from_health_str() {
        // Test "green" maps to Healthy
        assert_eq!(HealthState::from_health_str("green"), HealthState::Healthy);
        assert_eq!(HealthState::from_health_str("GREEN"), HealthState::Healthy);
        assert_eq!(HealthState::from_health_str("Green"), HealthState::Healthy);

        // Test "red" maps to Unhealthy
        assert_eq!(HealthState::from_health_str("red"), HealthState::Unhealthy);
        assert_eq!(HealthState::from_health_str("RED"), HealthState::Unhealthy);
        assert_eq!(HealthState::from_health_str("Red"), HealthState::Unhealthy);

        // Test "yellow" and other values map to Unknown
        assert_eq!(HealthState::from_health_str("yellow"), HealthState::Unknown);
        assert_eq!(HealthState::from_health_str("YELLOW"), HealthState::Unknown);
        assert_eq!(
            HealthState::from_health_str("invalid"),
            HealthState::Unknown
        );
        assert_eq!(HealthState::from_health_str(""), HealthState::Unknown);
    }

    #[test]
    fn test_current_screen_navigation() {
        // Test next navigation
        assert_eq!(CurrentScreen::Search.next(), CurrentScreen::Indexes);
        assert_eq!(CurrentScreen::Indexes.next(), CurrentScreen::Cluster);
        assert_eq!(CurrentScreen::Settings.next(), CurrentScreen::Overview);
        assert_eq!(CurrentScreen::Overview.next(), CurrentScreen::MultiInstance);
        assert_eq!(CurrentScreen::MultiInstance.next(), CurrentScreen::Search); // Wrap around

        // Test previous navigation
        assert_eq!(CurrentScreen::Indexes.previous(), CurrentScreen::Search);
        assert_eq!(CurrentScreen::Cluster.previous(), CurrentScreen::Indexes);
        assert_eq!(CurrentScreen::Overview.previous(), CurrentScreen::Settings);
        assert_eq!(
            CurrentScreen::MultiInstance.previous(),
            CurrentScreen::Overview
        );
        assert_eq!(
            CurrentScreen::Search.previous(),
            CurrentScreen::MultiInstance
        ); // Wrap around

        // Test JobInspect special case
        assert_eq!(CurrentScreen::JobInspect.next(), CurrentScreen::Jobs);
        assert_eq!(CurrentScreen::JobInspect.previous(), CurrentScreen::Jobs);
    }

    #[test]
    fn test_sort_column_cycle() {
        let mut sort = SortState::new();
        assert_eq!(sort.column, SortColumn::Sid);

        sort.cycle();
        assert_eq!(sort.column, SortColumn::Status);

        sort.cycle();
        assert_eq!(sort.column, SortColumn::Duration);

        sort.cycle();
        assert_eq!(sort.column, SortColumn::Results);

        sort.cycle();
        assert_eq!(sort.column, SortColumn::Events);

        sort.cycle();
        assert_eq!(sort.column, SortColumn::Sid); // Wrap around
    }

    #[test]
    fn test_sort_direction_toggle() {
        let mut sort = SortState::new();
        assert_eq!(sort.direction, SortDirection::Asc);

        sort.toggle_direction();
        assert_eq!(sort.direction, SortDirection::Desc);

        sort.toggle_direction();
        assert_eq!(sort.direction, SortDirection::Asc);
    }

    #[test]
    fn test_parse_sort_column() {
        assert_eq!(parse_sort_column("sid"), SortColumn::Sid);
        assert_eq!(parse_sort_column("SID"), SortColumn::Sid);
        assert_eq!(parse_sort_column("status"), SortColumn::Status);
        assert_eq!(parse_sort_column("duration"), SortColumn::Duration);
        assert_eq!(parse_sort_column("results"), SortColumn::Results);
        assert_eq!(parse_sort_column("events"), SortColumn::Events);
        assert_eq!(parse_sort_column("unknown"), SortColumn::Sid); // Default fallback
    }

    #[test]
    fn test_parse_sort_direction() {
        assert_eq!(parse_sort_direction("asc"), SortDirection::Asc);
        assert_eq!(parse_sort_direction("ASC"), SortDirection::Asc);
        assert_eq!(parse_sort_direction("desc"), SortDirection::Desc);
        assert_eq!(parse_sort_direction("DESC"), SortDirection::Desc);
        assert_eq!(parse_sort_direction("unknown"), SortDirection::Asc); // Default fallback
    }

    #[test]
    fn test_cluster_view_mode_default() {
        let mode: ClusterViewMode = Default::default();
        assert_eq!(mode, ClusterViewMode::Summary);
    }

    #[test]
    fn test_cluster_view_mode_toggle() {
        // Start with Summary, toggle to Peers
        let mode = ClusterViewMode::Summary;
        let toggled = mode.toggle();
        assert_eq!(toggled, ClusterViewMode::Peers);

        // Toggle back to Summary
        let toggled_back = toggled.toggle();
        assert_eq!(toggled_back, ClusterViewMode::Summary);
    }

    #[test]
    fn test_cluster_view_mode_toggle_cycle() {
        // Verify that toggling twice returns to original state
        let mode = ClusterViewMode::Summary;
        let after_two_toggles = mode.toggle().toggle();
        assert_eq!(mode, after_two_toggles);
    }

    #[test]
    fn test_list_pagination_state_new() {
        let state = ListPaginationState::new(100, 1000);
        assert_eq!(state.page_size, 100);
        assert_eq!(state.current_offset, 0);
        assert!(!state.has_more);
        assert_eq!(state.total_loaded, 0);
        assert_eq!(state.max_items, 1000);
        assert!(!state.is_loading);
    }

    #[test]
    fn test_list_pagination_state_update_loaded_partial_page() {
        let mut state = ListPaginationState::new(100, 1000);
        state.update_loaded(50);

        assert_eq!(state.total_loaded, 50);
        assert_eq!(state.current_offset, 50);
        assert!(!state.has_more, "Partial page means no more items");
    }

    #[test]
    fn test_list_pagination_state_update_loaded_full_page() {
        let mut state = ListPaginationState::new(100, 1000);
        state.update_loaded(100);

        assert_eq!(state.total_loaded, 100);
        assert_eq!(state.current_offset, 100);
        assert!(state.has_more, "Full page means there might be more items");
    }

    #[test]
    fn test_list_pagination_state_update_loaded_multiple_pages() {
        let mut state = ListPaginationState::new(50, 1000);

        // First page
        state.update_loaded(50);
        assert_eq!(state.total_loaded, 50);
        assert_eq!(state.current_offset, 50);
        assert!(state.has_more);

        // Second page
        state.update_loaded(50);
        assert_eq!(state.total_loaded, 100);
        assert_eq!(state.current_offset, 100);
        assert!(state.has_more);

        // Partial third page
        state.update_loaded(25);
        assert_eq!(state.total_loaded, 125);
        assert_eq!(state.current_offset, 125);
        assert!(!state.has_more);
    }

    #[test]
    fn test_list_pagination_state_reset() {
        let mut state = ListPaginationState::new(100, 1000);
        state.update_loaded(100);
        state.start_loading();
        assert!(state.has_more);
        assert!(state.is_loading);

        state.reset();

        assert_eq!(state.current_offset, 0);
        assert!(!state.has_more);
        assert_eq!(state.total_loaded, 0);
        assert!(!state.is_loading);
        // page_size and max_items should remain unchanged
        assert_eq!(state.page_size, 100);
        assert_eq!(state.max_items, 1000);
    }

    #[test]
    fn test_list_pagination_state_mark_complete() {
        let mut state = ListPaginationState::new(100, 1000);
        state.update_loaded(100);
        assert!(state.has_more);

        state.mark_complete();

        assert!(!state.has_more);
        // Other fields should remain unchanged
        assert_eq!(state.total_loaded, 100);
        assert_eq!(state.current_offset, 100);
    }

    #[test]
    fn test_list_pagination_state_empty_page() {
        let mut state = ListPaginationState::new(100, 1000);
        state.update_loaded(0);

        assert_eq!(state.total_loaded, 0);
        assert_eq!(state.current_offset, 0);
        assert!(!state.has_more, "Empty page means no more items");
    }

    #[test]
    fn test_can_load_more_when_under_cap() {
        let mut state = ListPaginationState::new(100, 1000);
        state.update_loaded(100);
        assert!(
            state.can_load_more(),
            "Should be able to load more when under cap"
        );
    }

    #[test]
    fn test_can_load_more_when_at_cap() {
        let mut state = ListPaginationState::new(100, 100);
        state.update_loaded(100);
        assert!(
            !state.can_load_more(),
            "Should not be able to load more when at cap"
        );
    }

    #[test]
    fn test_can_load_more_when_over_cap() {
        // This shouldn't happen in practice, but test the boundary
        let mut state = ListPaginationState::new(100, 50);
        state.update_loaded(100);
        assert!(
            !state.can_load_more(),
            "Should not be able to load more when over cap"
        );
    }

    #[test]
    fn test_can_load_more_respects_has_more() {
        let mut state = ListPaginationState::new(100, 1000);
        state.update_loaded(50); // Partial page, has_more = false
        assert!(
            !state.can_load_more(),
            "Should not be able to load more when has_more is false"
        );
    }

    #[test]
    fn test_can_load_more_respects_is_loading() {
        let mut state = ListPaginationState::new(100, 1000);
        state.update_loaded(100); // Full page, has_more = true
        assert!(
            state.can_load_more(),
            "Should be able to load more initially"
        );

        state.start_loading();
        assert!(
            !state.can_load_more(),
            "Should not be able to load more while already loading"
        );

        state.finish_loading();
        assert!(
            state.can_load_more(),
            "Should be able to load more after loading finishes"
        );
    }

    #[test]
    fn test_update_loaded_clears_is_loading() {
        let mut state = ListPaginationState::new(100, 1000);
        state.start_loading();
        assert!(state.is_loading);

        state.update_loaded(50);
        assert!(!state.is_loading);
        assert_eq!(state.total_loaded, 50)
    }

    #[test]
    fn test_can_load_more_when_both_conditions_met() {
        let mut state = ListPaginationState::new(100, 1000);
        state.update_loaded(100); // Full page, has_more = true
        assert!(state.has_more);
        assert!((state.total_loaded) < state.max_items);
        assert!(
            state.can_load_more(),
            "Should be able to load when both has_more is true and under cap"
        );
    }
}
