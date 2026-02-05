//! Tests for search results pagination.
//!
//! This module tests:
//! - Search completion state management
//! - Results appending
//! - Pagination trigger logic
//! - Error handling during pagination
//!
//! ## Invariants
//! - Pagination must trigger when near end of results and more exist
//! - Loading state must prevent duplicate fetch requests

mod helpers;
use splunk_tui::{CurrentScreen, ToastLevel, action::Action, app::App, app::ConnectionContext};
use std::sync::Arc;

// Local helper for creating mock search results
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
    // When total is None and results < page_size (1000), assume no more
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
    // Note: search_results_page_size is now synced with SearchDefaults::default().max_results (1000)
    let results = create_mock_search_results(1000); // Exactly page_size
    let sid = "test_sid_total_none_full".to_string();

    app.update(Action::SearchComplete(Ok((
        results.clone(),
        sid.clone(),
        None,
    ))));

    assert_eq!(app.search_results.len(), 1000);
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

    // Setup: 1000 results loaded, 2000 total, scroll at position 990 (within threshold)
    // Note: search_results_page_size is now synced with SearchDefaults::default().max_results (1000)
    app.search_results = create_mock_search_results(1000);
    app.search_sid = Some("test_sid".to_string());
    app.search_results_total_count = Some(2000);
    app.search_has_more_results = true;
    app.search_scroll_offset = 990;
    app.loading = false;

    let action = app.maybe_fetch_more_results();

    assert!(
        action.is_some(),
        "Should return LoadMoreSearchResults action when near end"
    );
    if let Some(Action::LoadMoreSearchResults { sid, offset, count }) = action {
        assert_eq!(sid, "test_sid");
        assert_eq!(offset, 1000);
        assert_eq!(count, 1000); // default page size (synced with SearchDefaults.max_results)
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

    // Setup: 1000 results loaded, total is None
    // Note: search_results_page_size is now synced with SearchDefaults::default().max_results (1000)
    app.search_results = create_mock_search_results(1000);
    app.search_sid = Some("test_sid".to_string());
    app.search_results_total_count = None;
    app.search_has_more_results = true;

    // Append a full page (1000 results)
    let more_results = create_mock_search_results(1000);
    app.update(Action::MoreSearchResultsLoaded(Ok((
        more_results,
        1000,
        None, // total is None
    ))));

    assert_eq!(app.search_results.len(), 2000);
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
