//! Tests for search status messaging (RQ-0111).
//!
//! This module tests:
//! - Search complete uses running_query, not current input
//! - running_query cleared on completion
//! - running_query cleared on error
//! - Fallback to search_input when running_query is None
//!
//! ## Invariants
//! - Status message must reference the submitted query, not current input
//! - running_query must be cleared after completion or error
//!
//! ## Test Organization
//! Tests focus on search status message behavior.

mod helpers;
use helpers::error_details_from_string;
use splunk_tui::{action::Action, app::App, app::ConnectionContext};

#[test]
fn test_search_complete_uses_running_query_not_current_input() {
    let mut app = App::new(None, ConnectionContext::default());

    // User types and submits a query
    app.search_input = "index=main | stats count".to_string();
    app.update(Action::SearchStarted(
        "index=main | stats count".to_string(),
    ));

    // While search is running, user modifies the input
    app.search_input = "different query".to_string();

    // Search completes
    app.update(Action::SearchComplete(Ok((
        vec![],
        "sid123".to_string(),
        Some(0),
    ))));

    // Status should reference the submitted query, not the current input
    assert!(app.search_status.contains("index=main | stats count"));
    assert!(!app.search_status.contains("different query"));
}

#[test]
fn test_running_query_cleared_on_completion() {
    let mut app = App::new(None, ConnectionContext::default());

    app.update(Action::SearchStarted("index=main".to_string()));
    assert_eq!(app.running_query, Some("index=main".to_string()));

    app.update(Action::SearchComplete(Ok((
        vec![],
        "sid123".to_string(),
        Some(0),
    ))));

    // running_query should be cleared after completion
    assert_eq!(app.running_query, None);
}

#[test]
fn test_running_query_cleared_on_error() {
    let mut app = App::new(None, ConnectionContext::default());

    app.update(Action::SearchStarted("index=main".to_string()));
    assert_eq!(app.running_query, Some("index=main".to_string()));

    app.update(Action::SearchComplete(Err((
        "Search failed".to_string(),
        error_details_from_string("test error"),
    ))));

    // running_query should be cleared on error
    assert_eq!(app.running_query, None);
}

#[test]
fn test_search_complete_fallback_to_search_input() {
    let mut app = App::new(None, ConnectionContext::default());

    // No SearchStarted action was sent (e.g., old code path or edge case)
    app.search_input = "fallback query".to_string();

    app.update(Action::SearchComplete(Ok((
        vec![],
        "sid123".to_string(),
        Some(0),
    ))));

    // Should fall back to search_input
    assert!(app.search_status.contains("fallback query"));
}
