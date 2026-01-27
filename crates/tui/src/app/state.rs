//! Application state types and enums.
//!
//! Responsibilities:
//! - Define screen navigation enum (CurrentScreen)
//! - Define health state enum (HealthState)
//! - Define sorting types (SortColumn, SortDirection, SortState)
//!
//! Non-responsibilities:
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
pub const HEADER_HEIGHT: u16 = 3;
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
    SavedSearches,
    InternalLogs,
    Apps,
    Users,
    Settings,
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
            CurrentScreen::Health => CurrentScreen::SavedSearches,
            CurrentScreen::SavedSearches => CurrentScreen::InternalLogs,
            CurrentScreen::InternalLogs => CurrentScreen::Apps,
            CurrentScreen::Apps => CurrentScreen::Users,
            CurrentScreen::Users => CurrentScreen::Settings,
            CurrentScreen::Settings => CurrentScreen::Search, // Wrap around
        }
    }

    /// Returns the previous screen in cyclic navigation order.
    /// Excludes JobInspect from the cycle (it's only accessible via InspectJob action).
    pub fn previous(self) -> Self {
        match self {
            CurrentScreen::Search => CurrentScreen::Settings, // Wrap around
            CurrentScreen::Indexes => CurrentScreen::Search,
            CurrentScreen::Cluster => CurrentScreen::Indexes,
            CurrentScreen::Jobs => CurrentScreen::Cluster,
            CurrentScreen::JobInspect => CurrentScreen::Jobs, // Special case: return to Jobs
            CurrentScreen::Health => CurrentScreen::Jobs,
            CurrentScreen::SavedSearches => CurrentScreen::Health,
            CurrentScreen::InternalLogs => CurrentScreen::SavedSearches,
            CurrentScreen::Apps => CurrentScreen::InternalLogs,
            CurrentScreen::Users => CurrentScreen::Apps,
            CurrentScreen::Settings => CurrentScreen::Users,
        }
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
        assert_eq!(CurrentScreen::Settings.next(), CurrentScreen::Search); // Wrap around

        // Test previous navigation
        assert_eq!(CurrentScreen::Indexes.previous(), CurrentScreen::Search);
        assert_eq!(CurrentScreen::Cluster.previous(), CurrentScreen::Indexes);
        assert_eq!(CurrentScreen::Search.previous(), CurrentScreen::Settings); // Wrap around

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
}
