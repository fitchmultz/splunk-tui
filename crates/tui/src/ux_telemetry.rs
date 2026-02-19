//! UX telemetry collection for measuring user friction points.
//!
//! Responsibilities:
//! - Emit low-cardinality counters for auth recovery events
//! - Track navigation reversal patterns (quick back-navigation)
//! - Record help usage with screen context
//!
//! Non-goals:
//! - High-frequency metrics (use main.rs frame metrics)
//! - PII in labels (strict enum-only label values)
//! - Alerting (handled by metrics backend)

use std::time::Instant;

/// Time threshold for considering navigation a "reversal" (2 seconds)
const NAVIGATION_REVERSAL_THRESHOLD_SECS: u64 = 2;
const NAVIGATION_REVERSAL_THRESHOLD_MS: u128 = NAVIGATION_REVERSAL_THRESHOLD_SECS as u128 * 1000;

/// Screen label for metrics - uses fixed enum names to prevent cardinality explosion.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ScreenLabel {
    Search,
    Indexes,
    Cluster,
    Jobs,
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
    Unknown,
}

impl ScreenLabel {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Search => "search",
            Self::Indexes => "indexes",
            Self::Cluster => "cluster",
            Self::Jobs => "jobs",
            Self::Health => "health",
            Self::License => "license",
            Self::Kvstore => "kvstore",
            Self::SavedSearches => "saved_searches",
            Self::Macros => "macros",
            Self::InternalLogs => "internal_logs",
            Self::Apps => "apps",
            Self::Users => "users",
            Self::Roles => "roles",
            Self::SearchPeers => "search_peers",
            Self::Inputs => "inputs",
            Self::Configs => "configs",
            Self::Settings => "settings",
            Self::Overview => "overview",
            Self::MultiInstance => "multi_instance",
            Self::FiredAlerts => "fired_alerts",
            Self::Forwarders => "forwarders",
            Self::Lookups => "lookups",
            Self::Audit => "audit",
            Self::Dashboards => "dashboards",
            Self::DataModels => "data_models",
            Self::WorkloadManagement => "workload_management",
            Self::Shc => "shc",
            Self::Unknown => "unknown",
        }
    }
}

impl From<crate::app::state::CurrentScreen> for ScreenLabel {
    fn from(screen: crate::app::state::CurrentScreen) -> Self {
        match screen {
            crate::app::state::CurrentScreen::Search => Self::Search,
            crate::app::state::CurrentScreen::Indexes => Self::Indexes,
            crate::app::state::CurrentScreen::Cluster => Self::Cluster,
            crate::app::state::CurrentScreen::Jobs => Self::Jobs,
            crate::app::state::CurrentScreen::JobInspect => Self::Jobs, // Collapse to Jobs
            crate::app::state::CurrentScreen::Health => Self::Health,
            crate::app::state::CurrentScreen::License => Self::License,
            crate::app::state::CurrentScreen::Kvstore => Self::Kvstore,
            crate::app::state::CurrentScreen::SavedSearches => Self::SavedSearches,
            crate::app::state::CurrentScreen::Macros => Self::Macros,
            crate::app::state::CurrentScreen::InternalLogs => Self::InternalLogs,
            crate::app::state::CurrentScreen::Apps => Self::Apps,
            crate::app::state::CurrentScreen::Users => Self::Users,
            crate::app::state::CurrentScreen::Roles => Self::Roles,
            crate::app::state::CurrentScreen::SearchPeers => Self::SearchPeers,
            crate::app::state::CurrentScreen::Inputs => Self::Inputs,
            crate::app::state::CurrentScreen::Configs => Self::Configs,
            crate::app::state::CurrentScreen::Settings => Self::Settings,
            crate::app::state::CurrentScreen::Overview => Self::Overview,
            crate::app::state::CurrentScreen::MultiInstance => Self::MultiInstance,
            crate::app::state::CurrentScreen::FiredAlerts => Self::FiredAlerts,
            crate::app::state::CurrentScreen::Forwarders => Self::Forwarders,
            crate::app::state::CurrentScreen::Lookups => Self::Lookups,
            crate::app::state::CurrentScreen::Audit => Self::Audit,
            crate::app::state::CurrentScreen::Dashboards => Self::Dashboards,
            crate::app::state::CurrentScreen::DataModels => Self::DataModels,
            crate::app::state::CurrentScreen::WorkloadManagement => Self::WorkloadManagement,
            crate::app::state::CurrentScreen::Shc => Self::Shc,
        }
    }
}

/// Auth recovery action labels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AuthRecoveryAction {
    Retry,
    SwitchProfile,
    CreateProfile,
    ViewError,
    Dismiss,
}

impl AuthRecoveryAction {
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Retry => "retry",
            Self::SwitchProfile => "switch_profile",
            Self::CreateProfile => "create_profile",
            Self::ViewError => "view_error",
            Self::Dismiss => "dismiss",
        }
    }
}

/// Navigation state for reversal detection.
#[derive(Debug, Default)]
pub struct NavigationHistory {
    /// Previous screen and timestamp.
    prev: Option<(ScreenLabel, Instant)>,
    /// Screen before previous (for A→B→A pattern detection).
    prev_prev: Option<ScreenLabel>,
}

impl NavigationHistory {
    pub fn new() -> Self {
        Self::default()
    }

    /// Record a navigation and check for reversal, using provided time source.
    pub fn record_and_check_reversal_at(
        &mut self,
        new_screen: ScreenLabel,
        now: Instant,
    ) -> Option<(ScreenLabel, ScreenLabel)> {
        // Same logic as record_and_check_reversal but using `now` parameter
        // instead of Instant::now()
        let reversal = if let Some((prev_screen, prev_time)) = self.prev {
            if let Some(prev_prev_screen) = self.prev_prev {
                if new_screen == prev_prev_screen {
                    let elapsed = now.duration_since(prev_time);
                    if elapsed.as_millis() <= NAVIGATION_REVERSAL_THRESHOLD_MS {
                        Some((prev_screen, new_screen))
                    } else {
                        None
                    }
                } else {
                    None
                }
            } else {
                None
            }
        } else {
            None
        };

        if let Some((prev_screen, _)) = self.prev {
            self.prev_prev = Some(prev_screen);
        }
        self.prev = Some((new_screen, now));

        reversal
    }

    /// Record a navigation and return whether it's a reversal (wall-clock time).
    pub fn record_and_check_reversal(
        &mut self,
        new_screen: ScreenLabel,
    ) -> Option<(ScreenLabel, ScreenLabel)> {
        self.record_and_check_reversal_at(new_screen, Instant::now())
    }
}

/// UX telemetry collector.
#[derive(Debug, Default)]
pub struct UxTelemetryCollector {
    /// Whether metrics are enabled (set from main.rs based on --metrics-bind flag).
    enabled: bool,
    /// Navigation history for reversal detection.
    nav_history: NavigationHistory,
}

impl UxTelemetryCollector {
    pub fn new(enabled: bool) -> Self {
        Self {
            enabled,
            nav_history: NavigationHistory::new(),
        }
    }

    /// Record an auth recovery popup shown event.
    pub fn record_auth_recovery_shown(&self, kind: crate::error_details::AuthRecoveryKind) {
        if !self.enabled {
            return;
        }
        metrics::counter!(
            splunk_client::metrics::METRIC_UX_AUTH_RECOVERY_TOTAL,
            "kind" => kind_as_str(kind),
        )
        .increment(1);
    }

    /// Record an auth recovery action taken.
    pub fn record_auth_recovery_action(
        &self,
        kind: crate::error_details::AuthRecoveryKind,
        action: AuthRecoveryAction,
        success: bool,
    ) {
        if !self.enabled {
            return;
        }
        metrics::counter!(
            splunk_client::metrics::METRIC_UX_AUTH_RECOVERY_SUCCESS,
            "kind" => kind_as_str(kind),
            "action" => action.as_str(),
            "success" => success.to_string(),
        )
        .increment(1);
    }

    /// Record navigation and check for reversal pattern.
    /// Returns true if a reversal was detected and recorded.
    pub fn record_navigation(&mut self, screen: ScreenLabel) -> bool {
        if !self.enabled {
            // Still update history even when disabled (for potential future enable)
            self.nav_history.record_and_check_reversal(screen);
            return false;
        }

        if let Some((from, to)) = self.nav_history.record_and_check_reversal(screen) {
            metrics::counter!(
                splunk_client::metrics::METRIC_UX_NAVIGATION_REVERSAL,
                "from_screen" => from.as_str().to_string(),
                "to_screen" => to.as_str().to_string(),
            )
            .increment(1);
            return true;
        }
        false
    }

    /// Record help popup opened with screen context.
    pub fn record_help_opened(&self, screen: ScreenLabel) {
        if !self.enabled {
            return;
        }
        metrics::counter!(
            splunk_client::metrics::METRIC_UX_HELP_OPENED,
            "screen" => screen.as_str().to_string(),
        )
        .increment(1);
    }

    /// Record bootstrap mode connection attempt result.
    pub fn record_bootstrap_connect(&self, success: bool, reason: &str) {
        if !self.enabled {
            return;
        }
        metrics::counter!(
            splunk_client::metrics::METRIC_UX_BOOTSTRAP_CONNECT,
            "success" => success.to_string(),
            "reason" => reason.to_string(),
        )
        .increment(1);
    }
}

fn kind_as_str(kind: crate::error_details::AuthRecoveryKind) -> String {
    match kind {
        crate::error_details::AuthRecoveryKind::InvalidCredentials => "invalid_credentials",
        crate::error_details::AuthRecoveryKind::SessionExpired => "session_expired",
        crate::error_details::AuthRecoveryKind::MissingAuthConfig => "missing_auth_config",
        crate::error_details::AuthRecoveryKind::TlsOrCertificate => "tls_error",
        crate::error_details::AuthRecoveryKind::ConnectionRefused => "connection_refused",
        crate::error_details::AuthRecoveryKind::Timeout => "timeout",
        crate::error_details::AuthRecoveryKind::Unknown => "unknown",
    }
    .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::thread;
    use std::time::Duration;

    #[test]
    fn test_navigation_reversal_detection() {
        let mut history = NavigationHistory::new();

        // First navigation: no reversal
        let result = history.record_and_check_reversal(ScreenLabel::Search);
        assert!(result.is_none());

        // Second navigation: no reversal yet
        let result = history.record_and_check_reversal(ScreenLabel::Indexes);
        assert!(result.is_none());

        // Third navigation back to Search: reversal detected
        let result = history.record_and_check_reversal(ScreenLabel::Search);
        assert!(result.is_some());
        let (from, to) = result.unwrap();
        assert_eq!(from, ScreenLabel::Indexes);
        assert_eq!(to, ScreenLabel::Search);
    }

    #[test]
    fn test_navigation_no_reversal_after_threshold() {
        let mut history = NavigationHistory::new();

        history.record_and_check_reversal(ScreenLabel::Search);
        history.record_and_check_reversal(ScreenLabel::Indexes);

        // Wait longer than threshold
        thread::sleep(Duration::from_millis(2100));

        // No reversal because too much time passed
        let result = history.record_and_check_reversal(ScreenLabel::Search);
        assert!(result.is_none());
    }

    #[test]
    fn test_screen_label_from_current_screen() {
        use crate::app::state::CurrentScreen;

        assert_eq!(ScreenLabel::from(CurrentScreen::Search).as_str(), "search");
        assert_eq!(
            ScreenLabel::from(CurrentScreen::Indexes).as_str(),
            "indexes"
        );
        assert_eq!(
            ScreenLabel::from(CurrentScreen::JobInspect).as_str(),
            "jobs"
        ); // Collapsed
    }

    #[test]
    fn test_disabled_collector_does_not_emit() {
        let collector = UxTelemetryCollector::new(false);
        // These should not panic and should not emit metrics
        collector
            .record_auth_recovery_shown(crate::error_details::AuthRecoveryKind::SessionExpired);
        collector.record_help_opened(ScreenLabel::Search);
        collector.record_bootstrap_connect(false, "test");
    }
}
