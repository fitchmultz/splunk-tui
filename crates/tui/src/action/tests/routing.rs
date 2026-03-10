//! Tests for centralized action routing metadata.
//!
//! Purpose:
//! - Validate the shared action classification used by reducer routing, bootstrap gating,
//!   and side-effect dispatch.
//!
//! Responsibilities:
//! - Verify stable tracing names for representative actions.
//! - Verify main-loop translation detection for pagination and refresh actions.
//! - Verify bootstrap-safe client gating decisions.
//! - Verify reducer route classification for major action families.
//!
//! Scope:
//! - Covers action metadata only; these tests do not execute reducers or side effects.
//!
//! Usage:
//! - Run with `cargo test -p splunk-tui action::tests::routing`.
//!
//! Invariants/Assumptions:
//! - Representative actions here should stay aligned with the routing contracts consumed by the app.

use crate::action::variants::{ConnectionDiagnosticsResult, DiagnosticCheck, DiagnosticStatus};
use crate::action::{Action, AppActionRoute};

#[test]
fn type_name_matches_representative_variants() {
    assert_eq!(Action::Quit.type_name(), "Quit");
    assert_eq!(
        Action::LoadIndexes {
            count: 10,
            offset: 20
        }
        .type_name(),
        "LoadIndexes"
    );
    assert_eq!(
        Action::ProfileSelected("default".to_string()).type_name(),
        "ProfileSelected"
    );
    assert_eq!(
        Action::ConnectionDiagnosticsLoaded(Ok(ConnectionDiagnosticsResult {
            reachable: DiagnosticCheck {
                name: "Reachability".to_string(),
                status: DiagnosticStatus::Pass,
                error: None,
                duration_ms: 0,
            },
            auth: DiagnosticCheck {
                name: "Authentication".to_string(),
                status: DiagnosticStatus::Pass,
                error: None,
                duration_ms: 0,
            },
            tls: DiagnosticCheck {
                name: "TLS".to_string(),
                status: DiagnosticStatus::Pass,
                error: None,
                duration_ms: 0,
            },
            server_info: None,
            overall_status: DiagnosticStatus::Pass,
            remediation_hints: Vec::new(),
            timestamp: "2026-03-09T19:00:00Z".to_string(),
        }))
        .type_name(),
        "ConnectionDiagnosticsLoaded"
    );
}

#[test]
fn main_loop_translation_detection_covers_pagination_and_refresh() {
    assert!(Action::LoadMoreRoles.is_main_loop_translated());
    assert!(Action::RefreshInputs.is_main_loop_translated());
    assert!(Action::LoadMoreWorkloadRules.is_main_loop_translated());
    assert!(
        !Action::LoadRoles {
            count: 25,
            offset: 0
        }
        .is_main_loop_translated()
    );
}

#[test]
fn requires_client_allows_bootstrap_safe_actions() {
    assert!(!Action::Quit.requires_client());
    assert!(
        !Action::OpenCreateProfileDialog {
            from_tutorial: false,
        }
        .requires_client()
    );
    assert!(!Action::SettingsLoaded(Default::default()).requires_client());
    assert!(Action::LoadHealth.requires_client());
}

#[test]
fn reducer_routes_are_classified_by_family() {
    assert_eq!(
        Action::LoadIndexes {
            count: 10,
            offset: 0
        }
        .app_route(),
        AppActionRoute::Navigation
    );
    assert_eq!(
        Action::SearchStarted("index=_internal".to_string()).app_route(),
        AppActionRoute::Search
    );
    assert_eq!(
        Action::OpenCreateProfileDialog {
            from_tutorial: true,
        }
        .app_route(),
        AppActionRoute::Tutorial
    );
    assert_eq!(
        Action::ProfileSelected("default".to_string()).app_route(),
        AppActionRoute::Profile
    );
    assert_eq!(Action::CycleTheme.app_route(), AppActionRoute::System);
    assert_eq!(Action::NextFocus.app_route(), AppActionRoute::Focus);
    assert_eq!(Action::Undo.app_route(), AppActionRoute::Undo);
    assert_eq!(
        Action::IndexesLoaded(Ok(Vec::new())).app_route(),
        AppActionRoute::DataLoading
    );
}
