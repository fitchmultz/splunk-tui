//! Tests for search results scrolling.
//!
//! This module tests:
//! - Page up/down scrolling
//! - Go to top/bottom navigation
//! - Line-by-line scrolling
//! - Scroll clamping at boundaries
//!
//! ## Invariants
//! - Scrolling must clamp to valid result indices
//! - Page scroll should move by 10 lines

mod helpers;
use helpers::*;
use splunk_tui::{CurrentScreen, action::Action, app::App, app::ConnectionContext};

#[test]
fn test_search_page_down_scrolls_by_10() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results((0..25).map(|i| serde_json::json!(i)).collect());
    app.search_scroll_offset = 0;

    // Page down
    app.update(Action::PageDown);

    assert_eq!(app.search_scroll_offset, 10, "Should scroll to offset 10");
}

#[test]
fn test_search_page_up_scrolls_back() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results((0..25).map(|i| serde_json::json!(i)).collect());
    app.search_scroll_offset = 20;

    // Page up
    app.update(Action::PageUp);

    assert_eq!(
        app.search_scroll_offset, 10,
        "Should scroll back to offset 10"
    );
}

#[test]
fn test_search_page_down_clamps_at_end() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results((0..15).map(|i| serde_json::json!(i)).collect());
    app.search_scroll_offset = 5;

    // Page down from offset 5 with 15 results
    app.update(Action::PageDown);

    // Should clamp to 14 (last index), not scroll past end
    assert_eq!(
        app.search_scroll_offset, 14,
        "Should clamp to last valid offset"
    );
}

#[test]
fn test_search_page_up_clamps_at_zero() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results((0..25).map(|i| serde_json::json!(i)).collect());
    app.search_scroll_offset = 5;

    // Page up from offset 5
    app.update(Action::PageUp);

    // saturating_sub prevents going below 0
    assert_eq!(app.search_scroll_offset, 0, "Should clamp to 0");
}

#[test]
fn test_search_page_up_from_zero_stays_at_zero() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results((0..25).map(|i| serde_json::json!(i)).collect());
    app.search_scroll_offset = 0;

    // Page up from offset 0
    app.update(Action::PageUp);

    assert_eq!(app.search_scroll_offset, 0, "Should stay at 0");
}

#[test]
fn test_search_go_to_top() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results((0..25).map(|i| serde_json::json!(i)).collect());
    app.search_scroll_offset = 20;

    // Go to top
    app.update(Action::GoToTop);

    assert_eq!(app.search_scroll_offset, 0, "Should go to offset 0");
}

#[test]
fn test_search_go_to_bottom() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results((0..25).map(|i| serde_json::json!(i)).collect());
    app.search_scroll_offset = 5;

    // Go to bottom
    app.update(Action::GoToBottom);

    // Should go to offset 24 (last result index)
    assert_eq!(
        app.search_scroll_offset, 24,
        "Should go to last result offset"
    );
}

#[test]
fn test_search_go_to_bottom_with_empty_results() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results(Vec::new());
    app.search_scroll_offset = 5;

    // Go to bottom with no results - should stay at 0
    app.update(Action::GoToBottom);

    assert_eq!(
        app.search_scroll_offset, 0,
        "Should stay at 0 when no results"
    );
}

#[test]
fn test_search_scroll_with_single_result() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results(vec![serde_json::json!(1)]);
    app.search_scroll_offset = 0;

    // Try to page down with only 1 result
    app.update(Action::PageDown);

    // Should clamp to 0 (only valid offset)
    assert_eq!(
        app.search_scroll_offset, 0,
        "Should stay at 0 with single result"
    );
}

#[test]
fn test_search_go_to_top_from_bottom() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results((0..50).map(|i| serde_json::json!(i)).collect());
    app.search_scroll_offset = 49;

    // Go to top from bottom
    app.update(Action::GoToTop);

    assert_eq!(app.search_scroll_offset, 0, "Should jump to top");
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
