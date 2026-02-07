//! Stress tests for pagination behavior and edge cases.
//!
//! This module tests:
//! - Rapid pagination requests
//! - Pagination at boundaries (offset=0, end of data)
//! - Pagination state consistency
//! - Pagination during loading
//! - Mixed pagination and navigation
//!
//! ## Invariants
//! - Pagination state must remain consistent under all operations
//! - Rapid pagination should not cause state corruption
//! - Boundary conditions must be handled correctly

mod helpers;
use splunk_client::models::SearchJobStatus;
use splunk_tui::{CurrentScreen, action::Action, app::App, app::ConnectionContext};

fn create_mock_jobs(count: usize) -> Vec<SearchJobStatus> {
    (0..count)
        .map(|i| SearchJobStatus {
            sid: format!("sid_{}", i),
            is_done: i % 2 == 0,
            is_finalized: false,
            done_progress: 0.5,
            run_duration: 10.0,
            disk_usage: 1024,
            scan_count: 100,
            event_count: 50,
            result_count: 25,
            cursor_time: None,
            priority: None,
            label: None,
        })
        .collect()
}

#[test]
fn test_rapid_pagination_requests() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(100))));
    app.jobs_state.select(Some(0));

    // Rapid pagination requests
    for i in 0..50 {
        if i % 2 == 0 {
            app.update(Action::PageDown);
        } else {
            app.update(Action::PageUp);
        }
    }

    // Jobs should still be intact
    assert_eq!(app.jobs.as_ref().unwrap().len(), 100);
    // Selection should be valid
    assert!(app.jobs_state.selected().unwrap() < 100);
}

#[test]
fn test_pagination_at_start_boundary() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(20))));
    app.jobs_state.select(Some(0));

    // Try to page up from start
    app.update(Action::PageUp);

    // Should stay at valid position
    let selected = app.jobs_state.selected().unwrap();
    assert!(
        selected < 20,
        "Selection should remain valid at start boundary"
    );
}

#[test]
fn test_pagination_at_end_boundary() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(20))));
    app.jobs_state.select(Some(19)); // Last item

    // Try to navigate past end
    app.update(Action::NavigateDown);

    // Should stay at last item
    assert_eq!(app.jobs_state.selected(), Some(19));
}

#[test]
fn test_go_to_bottom_with_empty_data() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    // No jobs loaded

    // Try to go to bottom
    app.update(Action::GoToBottom);

    // Should not panic
    assert!(app.jobs.is_none());
}

#[test]
fn test_go_to_top_with_empty_data() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    // No jobs loaded

    // Try to go to top
    app.update(Action::GoToTop);

    // Should not panic
    assert!(app.jobs.is_none());
}

#[test]
fn test_page_down_with_few_items() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));
    app.jobs_state.select(Some(0));

    // Page down with fewer than page size items
    app.update(Action::PageDown);

    // Should handle gracefully
    assert!(app.jobs_state.selected().is_some());
}

#[test]
fn test_alternating_pagination_and_navigation() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(50))));
    app.jobs_state.select(Some(0));

    // Mix of pagination and single navigation
    for i in 0..30 {
        match i % 4 {
            0 => app.update(Action::PageDown),
            1 => app.update(Action::NavigateDown),
            2 => app.update(Action::PageUp),
            3 => app.update(Action::NavigateUp),
            _ => unreachable!(),
        }
    }

    // Selection should be valid
    let selected = app.jobs_state.selected().unwrap();
    assert!(selected < 50, "Selection {} should be valid", selected);
}

#[test]
fn test_pagination_state_preserved_across_screen_switch() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(30))));

    // Navigate to middle
    for _ in 0..5 {
        app.update(Action::PageDown);
    }
    let selected_before = app.jobs_state.selected();

    // Switch screen and back
    app.update(Action::NextScreen);

    // Find our way back to Jobs
    for _ in 0..20 {
        app.update(Action::NextScreen);
        if app.current_screen == CurrentScreen::Jobs {
            break;
        }
    }

    // If we're back on Jobs, selection should be preserved
    if app.current_screen == CurrentScreen::Jobs {
        assert_eq!(
            app.jobs_state.selected(),
            selected_before,
            "Pagination state should be preserved"
        );
    }
}

#[test]
fn test_rapid_go_to_top_bottom() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(100))));

    // Rapidly alternate between top and bottom
    for _ in 0..20 {
        app.update(Action::GoToTop);
        assert_eq!(app.jobs_state.selected(), Some(0));

        app.update(Action::GoToBottom);
        assert_eq!(app.jobs_state.selected(), Some(99));
    }
}

#[test]
fn test_pagination_during_loading() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Start loading
    app.update(Action::Loading(true));

    // Try to paginate while loading
    for _ in 0..10 {
        app.update(Action::PageDown);
        app.update(Action::PageUp);
    }

    // Complete loading
    app.update(Action::Loading(false));
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(20))));

    // Should have valid selection
    assert!(app.jobs_state.selected().is_some());
}

#[test]
fn test_pagination_with_single_item() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(1))));
    app.jobs_state.select(Some(0));

    // Try various pagination on single item
    app.update(Action::PageDown);
    app.update(Action::PageUp);
    app.update(Action::NavigateDown);
    app.update(Action::NavigateUp);
    app.update(Action::GoToTop);
    app.update(Action::GoToBottom);

    // Should stay at the only item
    assert_eq!(app.jobs_state.selected(), Some(0));
}

#[test]
fn test_pagination_with_exactly_page_size_items() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    // Create exactly 10 items (typical page size)
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(10))));
    app.jobs_state.select(Some(0));

    // Page down at exact boundary
    app.update(Action::PageDown);

    // Should handle gracefully (might stay at last or wrap)
    let selected = app.jobs_state.selected().unwrap();
    assert!(selected < 10);
}

#[test]
fn test_rapid_selection_reset() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(20))));

    // Rapidly reset and re-select
    for i in 0..20 {
        app.jobs_state.select(Some(i % 20));
        assert_eq!(app.jobs_state.selected(), Some(i % 20));
    }
}

#[test]
fn test_pagination_after_data_refresh() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Initial data
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(30))));
    app.jobs_state.select(Some(15));

    // Navigate
    app.update(Action::PageDown);
    app.update(Action::PageDown);

    // Data refresh (fewer items)
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(10))));

    // Selection should be adjusted to valid range
    let selected = app.jobs_state.selected().unwrap();
    assert!(
        selected < 10,
        "Selection {} should be adjusted to new data range",
        selected
    );
}

#[test]
fn test_extreme_pagination_boundaries() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));

    // Try extreme navigation
    app.jobs_state.select(Some(1000)); // Way out of bounds

    // Navigate should handle gracefully without panic
    app.update(Action::NavigateDown);

    // The app should still be valid - selection may or may not be corrected
    // depending on implementation, but shouldn't crash
    assert!(app.jobs.is_some(), "App should still have jobs data");
}

#[test]
fn test_mixed_pagination_and_filter() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(50))));

    // Paginate
    app.update(Action::PageDown);
    app.update(Action::PageDown);

    // Enter filter mode
    app.update(Action::EnterSearchMode);

    // Add filter text
    app.update(Action::SearchInput('t'));
    app.update(Action::SearchInput('e'));
    app.update(Action::SearchInput('s'));
    app.update(Action::SearchInput('t'));

    // Exit filter mode
    app.update(Action::ClearSearch);

    // Paginate again
    app.update(Action::PageUp);

    // Should be valid
    assert!(app.jobs.is_some());
}

#[test]
fn test_page_down_empty_indexes() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Indexes;
    app.indexes = Some(vec![]);
    app.indexes_state.select(Some(0));

    // Page down on empty list should not panic
    app.update(Action::PageDown);

    // Selection may be None or remain at 0, but should not crash
}

#[test]
fn test_page_down_empty_saved_searches() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::SavedSearches;
    app.saved_searches = Some(vec![]);
    app.saved_searches_state.select(Some(0));

    app.update(Action::PageDown);
}

#[test]
fn test_page_down_empty_internal_logs() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::InternalLogs;
    app.internal_logs = Some(vec![]);
    app.internal_logs_state.select(Some(0));

    app.update(Action::PageDown);
}

#[test]
fn test_page_down_empty_apps() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Apps;
    app.apps = Some(vec![]);
    app.apps_state.select(Some(0));

    app.update(Action::PageDown);
}

#[test]
fn test_page_down_all_navigation_methods_empty_lists() {
    // Test all navigation methods on empty lists for affected screens
    let screens = [
        CurrentScreen::Indexes,
        CurrentScreen::SavedSearches,
        CurrentScreen::InternalLogs,
        CurrentScreen::Apps,
    ];

    for screen in screens {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = screen;

        // Set empty lists based on screen
        match screen {
            CurrentScreen::Indexes => app.indexes = Some(vec![]),
            CurrentScreen::SavedSearches => app.saved_searches = Some(vec![]),
            CurrentScreen::InternalLogs => app.internal_logs = Some(vec![]),
            CurrentScreen::Apps => app.apps = Some(vec![]),
            _ => unreachable!(),
        }

        // All navigation operations should be no-ops, not panics
        app.update(Action::PageDown);
        app.update(Action::PageUp);
        app.update(Action::NavigateDown);
        app.update(Action::NavigateUp);
        app.update(Action::GoToTop);
        app.update(Action::GoToBottom);
    }
}
