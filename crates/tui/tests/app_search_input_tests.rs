//! Tests for search input handling and history navigation.
//!
//! This module tests:
//! - Search input handling (digits, characters)
//! - Search history navigation and deduplication
//! - Input mode interactions with search results
//!
//! ## Invariants
//! - Digits typed in search screen must go to search input, not trigger navigation
//! - History navigation must preserve saved input when going back
//! - Typing while in history must reset history index

mod helpers;
use helpers::*;
use splunk_tui::{CurrentScreen, action::Action, app::App, app::ConnectionContext};

#[test]
fn test_digits_typed_in_search_query() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Type digits - should be added to search_input, not trigger navigation
    app.handle_input(key('1'));
    app.handle_input(key('2'));
    app.handle_input(key('3'));
    app.handle_input(key('0'));
    app.handle_input(key('9'));

    assert_eq!(
        app.search_input, "12309",
        "Digits should be typed into search query"
    );
}

#[test]
fn test_search_history_navigation() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_history = vec!["query1".to_string(), "query2".to_string()];
    app.search_input = "current".to_string();

    // Press Up once - should show query1 (index 0)
    app.handle_input(up_key());
    assert_eq!(app.search_input, "query1");
    assert_eq!(app.history_index, Some(0));
    assert_eq!(app.saved_search_input, "current");

    // Press Up again - should show query2 (index 1)
    app.handle_input(up_key());
    assert_eq!(app.search_input, "query2");
    assert_eq!(app.history_index, Some(1));

    // Press Up again - should stay at query2 (last item)
    app.handle_input(up_key());
    assert_eq!(app.search_input, "query2");
    assert_eq!(app.history_index, Some(1));

    // Press Down - should go back to query1
    app.handle_input(down_key());
    assert_eq!(app.search_input, "query1");
    assert_eq!(app.history_index, Some(0));

    // Press Down again - should return to "current" (saved input)
    app.handle_input(down_key());
    assert_eq!(app.search_input, "current");
    assert_eq!(app.history_index, None);
}

#[test]
fn test_search_history_add_on_enter() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input = "new query".to_string();

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
    app.search_input = "other".to_string();

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
fn test_search_result_scrolling_by_line() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results((0..10).map(|i| serde_json::json!(i)).collect());
    app.search_scroll_offset = 0;

    // Verify we're in QueryFocused mode (default)
    assert!(matches!(
        app.search_input_mode,
        splunk_tui::SearchInputMode::QueryFocused
    ));

    // Use Ctrl+j to scroll down (NavigateDown)
    // Note: Ctrl+j works in QueryFocused mode since it's not a printable character (has CONTROL modifier)
    let action = app.handle_input(ctrl_key('j'));
    assert!(
        matches!(action, Some(Action::NavigateDown)),
        "Ctrl+j should return NavigateDown action"
    );
    app.update(action.unwrap());
    assert_eq!(app.search_scroll_offset, 1);

    // Use Ctrl+k to scroll up (NavigateUp)
    let action = app.handle_input(ctrl_key('k'));
    assert!(
        matches!(action, Some(Action::NavigateUp)),
        "Ctrl+k should return NavigateUp action"
    );
    app.update(action.unwrap());
    assert_eq!(app.search_scroll_offset, 0);
}
