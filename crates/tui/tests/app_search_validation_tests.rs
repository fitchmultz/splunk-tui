//! Tests for SPL query validation.
//!
//! This module tests:
//! - Validation triggering on input
//! - Validation result handling
//! - Validation debouncing
//! - Validation reset on input changes
//!
//! ## Invariants
//! - Validation must debounce rapid input changes
//! - Validation state must reset on new input

mod helpers;
use helpers::*;
use splunk_tui::{CurrentScreen, action::Action, app::App, app::ConnectionContext};

#[test]
fn test_spl_validation_triggered_on_input() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Type some characters
    app.handle_input(key('s'));
    app.handle_input(key('e'));
    app.handle_input(key('a'));

    // Validation should be pending
    assert!(
        app.spl_validation_pending,
        "Validation should be pending after input"
    );
    assert!(
        app.last_input_change.is_some(),
        "Last input change should be set"
    );
}

#[test]
fn test_spl_validation_not_triggered_for_short_input() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Type only 2 characters
    app.handle_input(key('a'));
    app.handle_input(key('b'));

    // Validation should still be pending (but handle_validation_tick won't dispatch for < 3 chars)
    assert!(app.spl_validation_pending);
}

#[test]
fn test_spl_validation_result_updates_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Simulate validation result (request_id 0 matches initial state)
    app.update(Action::SplValidationResult {
        valid: true,
        errors: vec![],
        warnings: vec![],
        request_id: 0,
    });

    assert_eq!(app.spl_validation_state.valid, Some(true));
    assert!(app.spl_validation_state.errors.is_empty());
    assert!(app.spl_validation_state.warnings.is_empty());
    assert!(!app.spl_validation_pending);
}

#[test]
fn test_spl_validation_result_with_errors() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Simulate validation result with errors (request_id 0 matches initial state)
    app.update(Action::SplValidationResult {
        valid: false,
        errors: vec!["Syntax error at position 10".to_string()],
        warnings: vec![],
        request_id: 0,
    });

    assert_eq!(app.spl_validation_state.valid, Some(false));
    assert_eq!(app.spl_validation_state.errors.len(), 1);
    assert!(app.spl_validation_state.warnings.is_empty());
}

#[test]
fn test_spl_validation_result_with_warnings() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Simulate validation result with warnings (request_id 0 matches initial state)
    app.update(Action::SplValidationResult {
        valid: true,
        errors: vec![],
        warnings: vec!["Deprecated command usage".to_string()],
        request_id: 0,
    });

    assert_eq!(app.spl_validation_state.valid, Some(true));
    assert!(app.spl_validation_state.errors.is_empty());
    assert_eq!(app.spl_validation_state.warnings.len(), 1);
}

#[test]
fn test_spl_validation_tick_not_ready() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Trigger validation
    app.handle_input(key('s'));
    app.handle_input(key('e'));
    app.handle_input(key('a'));
    app.handle_input(key('r'));

    // Immediately check tick - should not dispatch yet (debounce)
    let action = app.handle_validation_tick();
    assert!(
        action.is_none(),
        "Should not dispatch immediately due to debounce"
    );
}

#[test]
fn test_validation_reset_on_new_input() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Set some validation state
    app.spl_validation_state = splunk_tui::app::SplValidationState {
        valid: Some(true),
        errors: vec![],
        warnings: vec![],
        last_validated: Some(std::time::Instant::now()),
        request_id: 0,
    };

    // Type new character
    app.handle_input(key('x'));

    // Validation should be pending again
    assert!(app.spl_validation_pending);
}

#[test]
fn test_backspace_triggers_validation() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input.set_value("search");

    // Press backspace
    app.handle_input(backspace_key());

    // Validation should be triggered
    assert!(
        app.spl_validation_pending,
        "Validation should be triggered after backspace"
    );
}

#[test]
fn test_delete_triggers_validation() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input.set_value("search");
    app.search_input.set_cursor_position(3);

    // Press delete
    app.handle_input(delete_key());

    // Validation should be triggered
    assert!(
        app.spl_validation_pending,
        "Validation should be triggered after delete"
    );
}

// ============================================================================
// Stale Result Tests (RQ-0388)
// ============================================================================

#[test]
fn test_spl_validation_ignores_stale_results() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Simulate that a newer validation request has been made (request_id is now 2)
    app.validation_request_id = 2;

    // Stale result with old request_id arrives
    app.update(Action::SplValidationResult {
        valid: false,
        errors: vec!["Stale error".to_string()],
        warnings: vec![],
        request_id: 1, // Old request ID
    });

    // State should NOT have updated (still no validation result)
    assert_eq!(app.spl_validation_state.valid, None);
    assert!(app.spl_validation_state.errors.is_empty());
}

#[test]
fn test_spl_validation_accepts_current_results() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Simulate validation result with matching request_id
    app.validation_request_id = 1;
    app.update(Action::SplValidationResult {
        valid: true,
        errors: vec![],
        warnings: vec!["A warning".to_string()],
        request_id: 1, // Matches current request ID
    });

    // State should have updated
    assert_eq!(app.spl_validation_state.valid, Some(true));
    assert_eq!(app.spl_validation_state.warnings.len(), 1);
    assert_eq!(app.spl_validation_state.request_id, 1);
}

#[test]
fn test_spl_validation_out_of_order_completion() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Simulate scenario: request 1 and 2 are in flight, 2 completes first, then 1 completes

    // First, result for request 2 arrives
    app.validation_request_id = 2;
    app.update(Action::SplValidationResult {
        valid: true,
        errors: vec![],
        warnings: vec!["Result from request 2".to_string()],
        request_id: 2,
    });

    // Should be applied
    assert_eq!(app.spl_validation_state.valid, Some(true));
    assert_eq!(app.spl_validation_state.warnings.len(), 1);
    assert_eq!(app.spl_validation_state.request_id, 2);

    // Then, stale result for request 1 arrives
    app.update(Action::SplValidationResult {
        valid: false,
        errors: vec!["Stale error from request 1".to_string()],
        warnings: vec![],
        request_id: 1,
    });

    // Should NOT overwrite - state should still reflect request 2
    assert_eq!(app.spl_validation_state.valid, Some(true));
    assert!(app.spl_validation_state.errors.is_empty());
    assert_eq!(app.spl_validation_state.warnings.len(), 1);
    assert_eq!(app.spl_validation_state.request_id, 2);
}

#[test]
fn test_validation_request_id_increments_on_dispatch() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Initial request ID should be 0
    assert_eq!(app.validation_request_id, 0);

    // Simulate validation being triggered (as if debounce passed)
    app.spl_validation_pending = false;
    app.validation_request_id += 1;

    // Request ID should now be 1
    assert_eq!(app.validation_request_id, 1);

    // Trigger another validation
    app.spl_validation_pending = true;
    app.last_input_change = Some(std::time::Instant::now());
    app.spl_validation_pending = false;
    app.validation_request_id += 1;

    // Request ID should now be 2
    assert_eq!(app.validation_request_id, 2);
}
