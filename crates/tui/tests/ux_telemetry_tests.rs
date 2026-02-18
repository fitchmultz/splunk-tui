//! Tests for UX telemetry collection.

use splunk_tui::error_details::AuthRecoveryKind;
use splunk_tui::ux_telemetry::{
    AuthRecoveryAction, NavigationHistory, ScreenLabel, UxTelemetryCollector,
};
use std::thread;
use std::time::Duration;

#[test]
fn test_navigation_reversal_detects_quick_back() {
    let mut history = NavigationHistory::new();

    // Navigate A → B → A quickly
    assert!(
        history
            .record_and_check_reversal(ScreenLabel::Search)
            .is_none()
    );
    assert!(
        history
            .record_and_check_reversal(ScreenLabel::Indexes)
            .is_none()
    );

    let result = history.record_and_check_reversal(ScreenLabel::Search);
    assert!(result.is_some());
    let (from, to) = result.unwrap();
    assert_eq!(from, ScreenLabel::Indexes);
    assert_eq!(to, ScreenLabel::Search);
}

#[test]
fn test_navigation_no_reversal_after_delay() {
    let mut history = NavigationHistory::new();

    history.record_and_check_reversal(ScreenLabel::Search);
    history.record_and_check_reversal(ScreenLabel::Indexes);

    // Wait longer than threshold
    thread::sleep(Duration::from_millis(2100));

    let result = history.record_and_check_reversal(ScreenLabel::Search);
    assert!(
        result.is_none(),
        "Should not detect reversal after 2 second threshold"
    );
}

#[test]
fn test_navigation_different_destination_no_reversal() {
    let mut history = NavigationHistory::new();

    // A → B → C is not a reversal
    history.record_and_check_reversal(ScreenLabel::Search);
    history.record_and_check_reversal(ScreenLabel::Indexes);
    let result = history.record_and_check_reversal(ScreenLabel::Cluster);
    assert!(result.is_none());
}

#[test]
fn test_screen_label_as_str_values() {
    assert_eq!(ScreenLabel::Search.as_str(), "search");
    assert_eq!(ScreenLabel::Indexes.as_str(), "indexes");
    assert_eq!(
        ScreenLabel::WorkloadManagement.as_str(),
        "workload_management"
    );
    assert_eq!(ScreenLabel::Unknown.as_str(), "unknown");
}

#[test]
fn test_auth_recovery_action_as_str_values() {
    assert_eq!(AuthRecoveryAction::Retry.as_str(), "retry");
    assert_eq!(AuthRecoveryAction::SwitchProfile.as_str(), "switch_profile");
    assert_eq!(AuthRecoveryAction::CreateProfile.as_str(), "create_profile");
    assert_eq!(AuthRecoveryAction::ViewError.as_str(), "view_error");
    assert_eq!(AuthRecoveryAction::Dismiss.as_str(), "dismiss");
}

#[test]
fn test_ux_telemetry_collector_disabled_no_panic() {
    let collector = UxTelemetryCollector::new(false);

    // These should all succeed without panicking
    collector.record_auth_recovery_shown(AuthRecoveryKind::SessionExpired);
    collector.record_auth_recovery_action(
        AuthRecoveryKind::SessionExpired,
        AuthRecoveryAction::Retry,
        true,
    );
    collector.record_help_opened(ScreenLabel::Search);
    collector.record_bootstrap_connect(true, "test");
}

#[test]
fn test_navigation_history_tracks_multiple_transitions() {
    let mut history = NavigationHistory::new();

    // A → B → A → C → B (A→B→A is reversal, C→B→C would be reversal if we continued)
    history.record_and_check_reversal(ScreenLabel::Search);
    history.record_and_check_reversal(ScreenLabel::Indexes);
    let r1 = history.record_and_check_reversal(ScreenLabel::Search);
    assert!(r1.is_some());

    // Continue navigating
    history.record_and_check_reversal(ScreenLabel::Cluster);
    let r2 = history.record_and_check_reversal(ScreenLabel::Indexes);
    // Search → Cluster → Indexes is not a reversal (A→C→B not A→B→A)
    assert!(r2.is_none());
}
