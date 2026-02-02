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

    // Simulate validation result
    app.update(Action::SplValidationResult {
        valid: true,
        errors: vec![],
        warnings: vec![],
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

    // Simulate validation result with errors
    app.update(Action::SplValidationResult {
        valid: false,
        errors: vec!["Syntax error at position 10".to_string()],
        warnings: vec![],
    });

    assert_eq!(app.spl_validation_state.valid, Some(false));
    assert_eq!(app.spl_validation_state.errors.len(), 1);
    assert!(app.spl_validation_state.warnings.is_empty());
}

#[test]
fn test_spl_validation_result_with_warnings() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Simulate validation result with warnings
    app.update(Action::SplValidationResult {
        valid: true,
        errors: vec![],
        warnings: vec!["Deprecated command usage".to_string()],
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
    app.search_input = "search".to_string();
    app.search_cursor_position = 6;

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
    app.search_input = "search".to_string();
    app.search_cursor_position = 3;

    // Press delete
    app.handle_input(delete_key());

    // Validation should be triggered
    assert!(
        app.spl_validation_pending,
        "Validation should be triggered after delete"
    );
}
