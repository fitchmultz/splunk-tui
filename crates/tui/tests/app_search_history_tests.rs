//! Tests for search history navigation and management.
//!
//! This module tests:
//! - History navigation (up/down arrows)
//! - History deduplication
//! - Input preservation during history navigation
//! - History-triggered validation
//!
//! ## Invariants
//! - History navigation must preserve saved input when going back
//! - Duplicate entries must be moved to front, not duplicated

mod helpers;
use helpers::*;
use splunk_tui::{CurrentScreen, app::App, app::ConnectionContext};

#[test]
fn test_search_history_navigation() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_history = vec!["query1".to_string(), "query2".to_string()];
    app.search_input.set_value("current");

    // Press Up once - should show query1 (index 0)
    app.handle_input(up_key());
    assert_eq!(app.search_input.value(), "query1");
    assert_eq!(app.history_index, Some(0));
    assert_eq!(app.saved_search_input.value(), "current");

    // Press Up again - should show query2 (index 1)
    app.handle_input(up_key());
    assert_eq!(app.search_input.value(), "query2");
    assert_eq!(app.history_index, Some(1));

    // Press Up again - should stay at query2 (last item)
    app.handle_input(up_key());
    assert_eq!(app.search_input.value(), "query2");
    assert_eq!(app.history_index, Some(1));

    // Press Down - should go back to query1
    app.handle_input(down_key());
    assert_eq!(app.search_input.value(), "query1");
    assert_eq!(app.history_index, Some(0));

    // Press Down again - should return to "current" (saved input)
    app.handle_input(down_key());
    assert_eq!(app.search_input.value(), "current");
    assert_eq!(app.history_index, None);
}

#[test]
fn test_search_history_add_on_enter() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input.set_value("new query");

    // Press Enter to execute search
    app.handle_input(enter_key());

    // Should be added to history
    assert_eq!(app.search_history.len(), 1);
    assert_eq!(app.search_history[0], "new query");
}

#[test]
fn test_search_history_deduplication() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_history = vec!["old".to_string(), "other".to_string()];
    app.search_input.set_value("other");

    // Press Enter with a query already in history
    app.handle_input(enter_key());

    // Should move to front, not duplicate
    assert_eq!(app.search_history.len(), 2);
    assert_eq!(app.search_history[0], "other");
    assert_eq!(app.search_history[1], "old");
}

#[test]
fn test_search_input_resets_history_index() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_history = vec!["query1".to_string()];

    // Navigate to history
    app.handle_input(up_key());
    assert_eq!(app.history_index, Some(0));

    // Type something
    app.handle_input(key('x'));
    assert_eq!(app.history_index, None);

    // Navigate again
    app.handle_input(up_key());
    assert_eq!(app.history_index, Some(0));

    // Backspace
    app.handle_input(backspace_key());
    assert_eq!(app.history_index, None);
}

#[test]
fn test_history_navigation_triggers_validation() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_history = vec!["query1".to_string(), "query2".to_string()];

    // Navigate up to history
    app.handle_input(up_key());

    // Validation should be triggered
    assert!(
        app.spl_validation_pending,
        "Validation should be triggered after history navigation"
    );
}
