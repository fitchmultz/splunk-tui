//! Tests for search input, history, pagination, and results scrolling.
//!
//! This module tests:
//! - Search input handling (digits, characters)
//! - Search history navigation and deduplication
//! - Search results scrolling (page up/down, line scrolling)
//! - Search completion and pagination state
//! - Results appending and pagination triggers
//!
//! ## Invariants
//! - Digits typed in search screen must go to search input, not trigger navigation
//! - History navigation must preserve saved input when going back
//! - Pagination must trigger when near end of results and more exist
//!
//! ## Test Organization
//! Tests are grouped by: input handling, history, scrolling, pagination.

mod helpers;
use helpers::*;
use splunk_tui::{CurrentScreen, ToastLevel, action::Action, app::App, app::ConnectionContext};
use std::sync::Arc;

fn create_mock_search_results(count: usize) -> Vec<serde_json::Value> {
    (0..count)
        .map(|i| {
            serde_json::json!({
                "_time": format!("2024-01-15T10:{:02}:00.000Z", i),
                "level": "INFO",
                "message": format!("Test message {}", i),
            })
        })
        .collect()
}

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

#[test]
fn test_search_complete_sets_pagination_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Simulate search completion with total count
    let results = create_mock_search_results(50);
    let sid = "test_sid_123".to_string();
    let total = Some(200);

    app.update(Action::SearchComplete(Ok((
        results.clone(),
        sid.clone(),
        total,
    ))));

    // Verify basic results are set
    assert_eq!(app.search_results.len(), 50);
    assert_eq!(app.search_sid.as_ref(), Some(&sid));

    // Verify pagination state is set correctly
    assert_eq!(app.search_results_total_count, Some(200));
    assert!(
        app.search_has_more_results,
        "Should have more results when loaded < total"
    );
    assert!(
        !app.loading,
        "Loading should be false after search complete"
    );
}

#[test]
fn test_search_complete_with_no_total() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Simulate search completion without total count (API doesn't always provide it)
    let results = create_mock_search_results(10);
    let sid = "test_sid_456".to_string();

    app.update(Action::SearchComplete(Ok((
        results.clone(),
        sid.clone(),
        None,
    ))));

    assert_eq!(app.search_results.len(), 10);
    assert_eq!(app.search_results_total_count, None);
    // When total is None and results < page_size (100), assume no more
    assert!(
        !app.search_has_more_results,
        "Should not have more when total is None and results < page_size"
    );
}

#[test]
fn test_search_complete_when_total_is_none_with_full_page() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Simulate search completion with total = None but full page (exactly page_size)
    let results = create_mock_search_results(100); // Exactly page_size
    let sid = "test_sid_total_none_full".to_string();

    app.update(Action::SearchComplete(Ok((
        results.clone(),
        sid.clone(),
        None,
    ))));

    assert_eq!(app.search_results.len(), 100);
    assert_eq!(app.search_results_total_count, None);
    // When total is None and results == page_size, assume more may exist
    assert!(
        app.search_has_more_results,
        "Should have more when total is None and results == page_size"
    );
}

#[test]
fn test_search_complete_when_all_results_loaded() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Simulate search completion where loaded == total (all results)
    let results = create_mock_search_results(100);
    let sid = "test_sid_789".to_string();
    let total = Some(100);

    app.update(Action::SearchComplete(Ok((
        results.clone(),
        sid.clone(),
        total,
    ))));

    assert_eq!(app.search_results.len(), 100);
    assert_eq!(app.search_results_total_count, Some(100));
    assert!(
        !app.search_has_more_results,
        "Should not have more results when loaded == total"
    );
}

#[test]
fn test_append_search_results_increases_results() {
    let mut app = App::new(None, ConnectionContext::default());

    // Initial state: 100 results loaded, 500 total
    app.search_results = create_mock_search_results(100);
    app.search_sid = Some("test_sid".to_string());
    app.search_results_total_count = Some(500);
    app.search_has_more_results = true;

    // Append 100 more results
    let more_results = create_mock_search_results(100);
    app.update(Action::MoreSearchResultsLoaded(Ok((
        more_results,
        100,
        Some(500),
    ))));

    assert_eq!(app.search_results.len(), 200);
    assert_eq!(app.search_results_total_count, Some(500));
    assert!(
        app.search_has_more_results,
        "Should still have more results"
    );
}

#[test]
fn test_append_search_results_reaches_total() {
    let mut app = App::new(None, ConnectionContext::default());

    // Initial state: 400 results loaded, 500 total
    app.search_results = create_mock_search_results(400);
    app.search_sid = Some("test_sid".to_string());
    app.search_results_total_count = Some(500);
    app.search_has_more_results = true;

    // Append final 100 results
    let more_results = create_mock_search_results(100);
    app.update(Action::MoreSearchResultsLoaded(Ok((
        more_results,
        400,
        Some(500),
    ))));

    assert_eq!(app.search_results.len(), 500);
    assert_eq!(app.search_results_total_count, Some(500));
    assert!(
        !app.search_has_more_results,
        "Should not have more results when reaching total"
    );
}

#[test]
fn test_maybe_fetch_more_results_returns_action_when_needed() {
    let mut app = App::new(None, ConnectionContext::default());

    // Setup: 100 results loaded, 1000 total, scroll at position 90 (within threshold)
    app.search_results = create_mock_search_results(100);
    app.search_sid = Some("test_sid".to_string());
    app.search_results_total_count = Some(1000);
    app.search_has_more_results = true;
    app.search_scroll_offset = 90;
    app.loading = false;

    let action = app.maybe_fetch_more_results();

    assert!(
        action.is_some(),
        "Should return LoadMoreSearchResults action when near end"
    );
    if let Some(Action::LoadMoreSearchResults { sid, offset, count }) = action {
        assert_eq!(sid, "test_sid");
        assert_eq!(offset, 100);
        assert_eq!(count, 100); // default page size
    } else {
        panic!("Expected LoadMoreSearchResults action");
    }
}

#[test]
fn test_maybe_fetch_more_results_returns_none_when_not_near_end() {
    let mut app = App::new(None, ConnectionContext::default());

    // Setup: 100 results loaded, scroll at position 50 (not within threshold)
    app.search_results = create_mock_search_results(100);
    app.search_sid = Some("test_sid".to_string());
    app.search_results_total_count = Some(1000);
    app.search_has_more_results = true;
    app.search_scroll_offset = 50;
    app.loading = false;

    let action = app.maybe_fetch_more_results();

    assert!(
        action.is_none(),
        "Should not return action when not near end of results"
    );
}

#[test]
fn test_maybe_fetch_more_results_returns_none_when_no_more_results() {
    let mut app = App::new(None, ConnectionContext::default());

    // Setup: All results loaded (search_has_more_results = false)
    app.search_results = create_mock_search_results(100);
    app.search_sid = Some("test_sid".to_string());
    app.search_results_total_count = Some(100);
    app.search_has_more_results = false;
    app.search_scroll_offset = 95;
    app.loading = false;

    let action = app.maybe_fetch_more_results();

    assert!(
        action.is_none(),
        "Should not return action when no more results available"
    );
}

#[test]
fn test_maybe_fetch_more_results_returns_none_when_already_loading() {
    let mut app = App::new(None, ConnectionContext::default());

    // Setup: loading = true prevents duplicate fetches
    app.search_results = create_mock_search_results(100);
    app.search_sid = Some("test_sid".to_string());
    app.search_results_total_count = Some(1000);
    app.search_has_more_results = true;
    app.search_scroll_offset = 95;
    app.loading = true; // Already loading

    let action = app.maybe_fetch_more_results();

    assert!(
        action.is_none(),
        "Should not return action when already loading"
    );
}

#[test]
fn test_maybe_fetch_more_results_returns_none_when_no_sid() {
    let mut app = App::new(None, ConnectionContext::default());

    // Setup: no search SID (no active search)
    app.search_results = create_mock_search_results(100);
    app.search_sid = None; // No SID
    app.search_results_total_count = Some(1000);
    app.search_has_more_results = true;
    app.search_scroll_offset = 95;
    app.loading = false;

    let action = app.maybe_fetch_more_results();

    assert!(action.is_none(), "Should not return action when no SID");
}

#[test]
fn test_more_search_results_loaded_error_handling() {
    let mut app = App::new(None, ConnectionContext::default());

    // Setup initial state
    app.search_results = create_mock_search_results(50);
    app.search_sid = Some("test_sid".to_string());
    app.search_results_total_count = Some(500);
    app.search_has_more_results = true;
    app.loading = true;

    // Simulate error loading more results
    let error = splunk_client::ClientError::Timeout(std::time::Duration::from_secs(30));
    app.update(Action::MoreSearchResultsLoaded(Err(Arc::new(error))));

    // Results should be unchanged
    assert_eq!(app.search_results.len(), 50);
    assert_eq!(app.search_results_total_count, Some(500));

    // Loading should be cleared
    assert!(!app.loading);

    // Error toast should be added
    assert!(!app.toasts.is_empty(), "Should have error toast");
    let toast = &app.toasts[0];
    assert_eq!(toast.level, ToastLevel::Error);
    assert!(
        toast.message.contains("Failed to load more results"),
        "Toast should mention loading failure"
    );
}

#[test]
fn test_append_search_results_when_total_is_none() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Setup: 100 results loaded, total is None
    app.search_results = create_mock_search_results(100);
    app.search_sid = Some("test_sid".to_string());
    app.search_results_total_count = None;
    app.search_has_more_results = true;

    // Append a full page (100 results)
    let more_results = create_mock_search_results(100);
    app.update(Action::MoreSearchResultsLoaded(Ok((
        more_results,
        100,
        None, // total is None
    ))));

    assert_eq!(app.search_results.len(), 200);
    assert_eq!(app.search_results_total_count, None);
    assert!(
        app.search_has_more_results,
        "Should have more when total is None and page was full"
    );
}

#[test]
fn test_append_search_results_when_total_is_none_partial_page() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Setup: 100 results loaded, total is None
    app.search_results = create_mock_search_results(100);
    app.search_sid = Some("test_sid".to_string());
    app.search_results_total_count = None;
    app.search_has_more_results = true;

    // Append a partial page (50 results, less than page_size)
    let more_results = create_mock_search_results(50);
    app.update(Action::MoreSearchResultsLoaded(Ok((
        more_results,
        100,
        None, // total is None
    ))));

    assert_eq!(app.search_results.len(), 150);
    assert_eq!(app.search_results_total_count, None);
    assert!(
        !app.search_has_more_results,
        "Should not have more when total is None and page was partial"
    );
}

#[test]
fn test_pagination_trigger_at_threshold() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    app.search_sid = Some("test_sid".to_string());
    app.search_results_page_size = 50;
    app.search_has_more_results = true;
    app.loading = false;

    let results: Vec<serde_json::Value> = (0..50).map(|i| serde_json::json!({"id": i})).collect();
    app.append_search_results(results, Some(200));

    app.search_scroll_offset = 40;

    let action = app.maybe_fetch_more_results();
    assert!(
        action.is_some(),
        "Should trigger LoadMoreSearchResults when within threshold"
    );

    if let Some(Action::LoadMoreSearchResults { sid, offset, count }) = action {
        assert_eq!(sid, "test_sid");
        assert_eq!(offset, 50);
        assert_eq!(count, 50);
    } else {
        panic!("Expected LoadMoreSearchResults action");
    }
}

#[test]
fn test_pagination_no_trigger_before_threshold() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    app.search_sid = Some("test_sid".to_string());
    app.search_results_page_size = 50;
    app.search_has_more_results = true;
    app.loading = false;

    let results: Vec<serde_json::Value> = (0..50).map(|i| serde_json::json!({"id": i})).collect();
    app.append_search_results(results, Some(200));

    app.search_scroll_offset = 30;

    let action = app.maybe_fetch_more_results();
    assert!(
        action.is_none(),
        "Should NOT trigger LoadMoreSearchResults before threshold"
    );
}

#[test]
fn test_pagination_no_trigger_when_all_loaded() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    app.search_sid = Some("test_sid".to_string());
    app.search_results_page_size = 50;
    app.search_has_more_results = false;
    app.loading = false;

    let results: Vec<serde_json::Value> = (0..50).map(|i| serde_json::json!({"id": i})).collect();
    app.append_search_results(results, Some(50));

    app.search_scroll_offset = 40;

    let action = app.maybe_fetch_more_results();
    assert!(
        action.is_none(),
        "Should NOT trigger LoadMoreSearchResults when all results loaded"
    );
}

#[test]
fn test_pagination_no_trigger_while_loading() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    app.search_sid = Some("test_sid".to_string());
    app.search_results_page_size = 50;
    app.search_has_more_results = true;
    app.loading = true;

    let results: Vec<serde_json::Value> = (0..50).map(|i| serde_json::json!({"id": i})).collect();
    app.append_search_results(results, Some(200));

    app.search_scroll_offset = 40;

    let action = app.maybe_fetch_more_results();
    assert!(
        action.is_none(),
        "Should NOT trigger LoadMoreSearchResults while loading"
    );
}
