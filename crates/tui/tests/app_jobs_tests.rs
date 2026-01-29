//! Tests for jobs loading, selection, filtering, and auto-refresh.
//!
//! This module tests:
//! - Jobs loading and selection preservation
//! - Auto-refresh toggle and tick suppression
//! - Sort column cycling
//! - Job inspection flow
//! - Jobs filter persistence and clear behavior
//! - Filtered job selection (RQ-0009 fix)
//!
//! ## Invariants
//! - Selection must be clamped to valid indices when jobs are reloaded
//! - Tick should be suppressed during popups
//! - Filter must persist across navigation
//!
//! ## Test Organization
//! Tests are grouped by functionality: loading, filtering, selection.

mod helpers;
use helpers::*;
use splunk_client::models::SearchJobStatus;
use splunk_tui::{
    CurrentScreen, Popup, PopupType, action::Action, app::App, app::ConnectionContext,
};

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
fn test_jobs_loaded_preserves_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(10))));
    app.jobs_state.select(Some(7));

    // Simulate loading new jobs with fewer items
    let new_jobs = create_mock_jobs(5);
    app.update(Action::JobsLoaded(Ok(new_jobs)));

    // Selection should be clamped to max valid index
    assert_eq!(
        app.jobs_state.selected(),
        Some(4),
        "Selection should be clamped to 4 (len - 1)"
    );
    assert!(app.jobs.is_some(), "Jobs should still be loaded");
}

#[test]
fn test_jobs_loaded_with_empty_list() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));
    app.jobs_state.select(Some(2));

    // Simulate loading empty jobs
    let new_jobs = create_mock_jobs(0);
    app.update(Action::JobsLoaded(Ok(new_jobs)));

    // Selection should be set to 0 even though list is empty
    assert_eq!(
        app.jobs_state.selected(),
        Some(0),
        "Selection should be 0 for empty list"
    );
}

#[test]
fn test_tick_suppressed_during_popup() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.auto_refresh = true;

    // Without popup, tick should return LoadJobs action
    let action = app.handle_tick();
    assert!(
        matches!(action, Some(Action::LoadJobs { .. })),
        "Tick should return LoadJobs when no popup"
    );

    // Open a popup
    app.popup = Some(Popup::builder(PopupType::Help).build());

    // With popup, tick should return None
    let action = app.handle_tick();
    assert!(action.is_none(), "Tick should be suppressed during popup");
}

#[test]
fn test_toggle_auto_refresh() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Auto-refresh should start as false
    assert!(!app.auto_refresh, "Auto-refresh should be false initially");

    // Press 'a' to toggle
    app.handle_input(key('a'));
    assert!(app.auto_refresh, "Auto-refresh should be true after toggle");

    // Press 'a' again to toggle back
    app.handle_input(key('a'));
    assert!(
        !app.auto_refresh,
        "Auto-refresh should be false after second toggle"
    );
}

#[test]
fn test_refresh_jobs_action() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Press 'r' to refresh (now returns LoadMoreJobs which gets converted)
    let action = app.handle_input(key('r'));
    assert!(
        matches!(action, Some(Action::LoadMoreJobs)),
        "Should return LoadMoreJobs action"
    );
}

#[test]
fn test_job_inspection_flow() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));

    // Press Enter to inspect job
    let action = app.handle_input(enter_key());
    assert!(
        matches!(action, Some(Action::InspectJob)),
        "Should return InspectJob action"
    );

    // Apply the action to transition screens
    app.update(action.unwrap());

    assert_eq!(
        app.current_screen,
        CurrentScreen::JobInspect,
        "Should transition to JobInspect screen"
    );

    // Press Esc to exit inspect mode
    let action = app.handle_input(esc_key());
    assert!(
        matches!(action, Some(Action::ExitInspectMode)),
        "Should return ExitInspectMode action"
    );

    // Apply the action to transition back
    app.update(action.unwrap());

    assert_eq!(
        app.current_screen,
        CurrentScreen::Jobs,
        "Should return to Jobs screen"
    );

    // Selection should be preserved
    assert_eq!(
        app.jobs_state.selected(),
        Some(1),
        "Selection should be preserved after returning from inspect"
    );
}

#[test]
fn test_job_inspection_without_jobs() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(vec![]))); // No jobs loaded
    app.jobs_state.select(Some(0));

    // Press Enter with no jobs loaded
    let action = app.handle_input(enter_key());
    assert!(
        matches!(action, Some(Action::InspectJob)),
        "Should still return InspectJob action"
    );

    // Apply the action - should NOT transition since no jobs are loaded
    app.update(action.unwrap());

    assert_eq!(
        app.current_screen,
        CurrentScreen::Jobs,
        "Should stay on Jobs screen when no jobs loaded"
    );
}

#[test]
fn test_jobs_filter_persistence() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Enter filter mode with '/'
    let action = app.handle_input(key('/'));
    // Process the EnterSearchMode action
    if let Some(a) = action {
        app.update(a);
    }
    assert!(app.is_filtering, "Should be in filter mode");
    assert!(
        app.filter_input.is_empty(),
        "Filter input should start empty"
    );
    assert!(
        app.search_filter.is_none(),
        "No filter should be applied yet"
    );

    // Type filter text "foo"
    app.handle_input(key('f'));
    app.handle_input(key('o'));
    app.handle_input(key('o'));
    assert_eq!(app.filter_input, "foo", "Filter input should be 'foo'");

    // Press Enter to apply filter
    let action = app.handle_input(enter_key());

    // Should NOT return ClearSearch (which would wipe the filter)
    assert!(
        action.is_none(),
        "Applying filter should not return ClearSearch action"
    );

    // Verify final state
    assert!(!app.is_filtering, "Should exit filter mode");
    assert_eq!(
        app.search_filter,
        Some("foo".to_string()),
        "Filter should persist after Enter"
    );
    assert!(
        app.filter_input.is_empty(),
        "Filter input should be cleared after apply"
    );
}

#[test]
fn test_jobs_filter_clear_with_empty_input() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Set an existing filter
    app.search_filter = Some("existing".to_string());

    // Enter filter mode
    let action = app.handle_input(key('/'));
    if let Some(a) = action {
        app.update(a);
    }
    assert!(app.is_filtering);
    // filter_input is pre-populated with existing filter
    assert_eq!(app.filter_input, "existing");

    // Clear the pre-populated input to simulate empty input
    app.filter_input.clear();

    // Press Enter without typing anything (empty input)
    let action = app.handle_input(enter_key());

    // Empty input should return ClearSearch to clear the filter
    assert!(
        matches!(action, Some(Action::ClearSearch)),
        "Empty input should return ClearSearch"
    );
    assert!(!app.is_filtering, "Should exit filter mode");
}

#[test]
fn test_jobs_filter_cancel_with_escape_restores_previous_filter() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Set an existing filter
    app.search_filter = Some("existing".to_string());

    // Enter filter mode
    let action = app.handle_input(key('/'));
    if let Some(a) = action {
        app.update(a);
    }
    assert!(app.is_filtering);
    assert_eq!(app.filter_before_edit, Some("existing".to_string()));
    assert_eq!(app.filter_input, "existing"); // Pre-populated with existing filter

    // Type some new text (replacing the pre-populated text)
    app.filter_input.clear();
    app.handle_input(key('f'));
    app.handle_input(key('o'));
    app.handle_input(key('o'));
    assert_eq!(app.filter_input, "foo");

    // Press Esc to cancel without applying
    let action = app.handle_input(esc_key());

    // Esc should NOT return ClearSearch - it should restore the previous filter
    assert!(
        action.is_none(),
        "Esc while editing should restore previous filter, not return ClearSearch"
    );
    assert!(!app.is_filtering, "Should exit filter mode");
    assert!(
        app.filter_input.is_empty(),
        "Filter input should be cleared"
    );
    // The existing filter should be restored (new behavior)
    assert_eq!(
        app.search_filter,
        Some("existing".to_string()),
        "Previous filter should be restored after cancel"
    );
    assert!(
        app.filter_before_edit.is_none(),
        "filter_before_edit should be cleared after cancel"
    );
}

#[test]
fn test_jobs_filter_esc_clears_when_no_previous_filter() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // No existing filter - enter filter mode fresh
    let action = app.handle_input(key('/'));
    if let Some(a) = action {
        app.update(a);
    }
    assert!(app.is_filtering);
    assert_eq!(app.filter_before_edit, None); // No previous filter to save

    // Type some text
    app.handle_input(key('t'));
    app.handle_input(key('e'));
    app.handle_input(key('s'));
    app.handle_input(key('t'));
    assert_eq!(app.filter_input, "test");

    // Press Esc to cancel
    let action = app.handle_input(esc_key());

    // Since there's no previous filter, Esc should return ClearSearch
    assert!(
        matches!(action, Some(Action::ClearSearch)),
        "Esc should return ClearSearch when no previous filter exists"
    );
    assert!(!app.is_filtering, "Should exit filter mode");
    assert_eq!(
        app.search_filter, None,
        "Filter should remain None when canceling with no previous filter"
    );
}

#[test]
fn test_jobs_filter_enter_commits_edit() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Set an existing filter
    app.search_filter = Some("old_filter".to_string());

    // Enter filter mode
    let action = app.handle_input(key('/'));
    if let Some(a) = action {
        app.update(a);
    }
    assert!(app.is_filtering);
    assert_eq!(app.filter_before_edit, Some("old_filter".to_string()));

    // Type new filter text (clear pre-populated first)
    app.filter_input.clear();
    app.handle_input(key('n'));
    app.handle_input(key('e'));
    app.handle_input(key('w'));
    assert_eq!(app.filter_input, "new");

    // Press Enter to commit
    let action = app.handle_input(enter_key());

    assert!(
        action.is_none(),
        "Enter with non-empty input should not return an action"
    );
    assert!(!app.is_filtering, "Should exit filter mode");
    assert_eq!(
        app.search_filter,
        Some("new".to_string()),
        "New filter should be applied"
    );
    assert!(
        app.filter_before_edit.is_none(),
        "filter_before_edit should be cleared after commit"
    );
    assert!(
        app.filter_input.is_empty(),
        "filter_input should be cleared after commit"
    );
}

#[test]
fn test_jobs_filter_enter_with_empty_input_clears() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Set an existing filter
    app.search_filter = Some("existing".to_string());

    // Enter filter mode
    let action = app.handle_input(key('/'));
    if let Some(a) = action {
        app.update(a);
    }
    assert!(app.is_filtering);

    // Clear the pre-populated input (simulating user deleting all text)
    app.filter_input.clear();

    // Press Enter with empty input
    let action = app.handle_input(enter_key());

    assert!(
        matches!(action, Some(Action::ClearSearch)),
        "Enter with empty input should return ClearSearch"
    );
    assert!(!app.is_filtering, "Should exit filter mode");
}

#[test]
fn test_clear_filter_rebuilds_indices() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Create jobs
    let jobs = create_mock_jobs(10);
    app.update(Action::JobsLoaded(Ok(jobs)));

    // Apply filter
    let action = app.handle_input(key('/'));
    if let Some(a) = action {
        app.update(a);
    }
    app.handle_input(key('s'));
    app.handle_input(key('i'));
    app.handle_input(key('d'));
    app.handle_input(key('_'));
    app.handle_input(key('0'));
    app.handle_input(enter_key());

    let filtered_len = app.filtered_job_indices.len();
    assert!(
        filtered_len < 10,
        "Filtered list should be shorter than full list"
    );

    // Clear filter by entering filter mode and pressing Enter with empty input
    let action = app.handle_input(key('/')); // Enter filter mode
    if let Some(a) = action {
        app.update(a);
    }
    // Clear the pre-populated input and press Enter to clear filter
    app.filter_input.clear();
    let action = app.handle_input(enter_key()); // Press Enter with empty input to clear
    if let Some(a) = action {
        app.update(a);
    }

    // filtered_job_indices should now contain all indices
    assert_eq!(
        app.filtered_job_indices.len(),
        10,
        "After clearing filter, all jobs should be visible"
    );
}

#[test]
fn test_sort_changes_rebuild_indices() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Create jobs with different durations
    let jobs = vec![
        SearchJobStatus {
            sid: "job_1".to_string(),
            is_done: true,
            is_finalized: false,
            done_progress: 1.0,
            run_duration: 100.0,
            disk_usage: 1024,
            scan_count: 100,
            event_count: 50,
            result_count: 25,
            cursor_time: None,
            priority: None,
            label: None,
        },
        SearchJobStatus {
            sid: "job_2".to_string(),
            is_done: false,
            is_finalized: false,
            done_progress: 0.5,
            run_duration: 10.0,
            disk_usage: 2048,
            scan_count: 200,
            event_count: 100,
            result_count: 50,
            cursor_time: None,
            priority: None,
            label: None,
        },
    ];
    app.update(Action::JobsLoaded(Ok(jobs)));

    // Initial sort by sid (ascending) - job_1 comes before job_2
    assert_eq!(
        app.filtered_job_indices[0], 0,
        "First item should be job_1 (sid asc)"
    );
    assert_eq!(
        app.filtered_job_indices[1], 1,
        "Second item should be job_2 (sid asc)"
    );

    // Cycle sort column
    let action = app.handle_input(key('s'));
    if let Some(a) = action {
        app.update(a);
    }

    // Indices should be rebuilt with new sort order
    assert_eq!(
        app.filtered_job_indices.len(),
        2,
        "Should still have 2 items"
    );
}

// Tests for filtered job selection (RQ-0009 fix)

#[test]
fn test_filtered_job_selection_inspect() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Create jobs with distinct SIDs for easy identification
    let jobs = vec![
        SearchJobStatus {
            sid: "aaa_job".to_string(),
            is_done: true,
            is_finalized: false,
            done_progress: 1.0,
            run_duration: 10.0,
            disk_usage: 1024,
            scan_count: 100,
            event_count: 50,
            result_count: 25,
            cursor_time: None,
            priority: None,
            label: None,
        },
        SearchJobStatus {
            sid: "bbb_job".to_string(),
            is_done: false,
            is_finalized: false,
            done_progress: 0.5,
            run_duration: 20.0,
            disk_usage: 2048,
            scan_count: 200,
            event_count: 100,
            result_count: 50,
            cursor_time: None,
            priority: None,
            label: None,
        },
        SearchJobStatus {
            sid: "ccc_job".to_string(),
            is_done: false,
            is_finalized: false,
            done_progress: 0.3,
            run_duration: 30.0,
            disk_usage: 3072,
            scan_count: 300,
            event_count: 150,
            result_count: 75,
            cursor_time: None,
            priority: None,
            label: None,
        },
    ];
    app.update(Action::JobsLoaded(Ok(jobs)));

    // Apply a filter that matches only "bbb_job" by entering filter mode
    let action = app.handle_input(key('/'));
    if let Some(a) = action {
        app.update(a);
    }
    app.handle_input(key('b'));
    app.handle_input(key('b'));
    app.handle_input(key('b'));
    app.handle_input(enter_key());

    // Select the first (and only) item in the filtered list
    app.jobs_state.select(Some(0));

    // Verify get_selected_job returns the correct job
    let selected = app.get_selected_job();
    assert!(selected.is_some(), "Should have a selected job");
    assert_eq!(
        selected.unwrap().sid,
        "bbb_job",
        "Should select bbb_job (the only matching job)"
    );
}

#[test]
fn test_filtered_job_selection_cancel() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Create jobs with specific SIDs
    let jobs = vec![
        SearchJobStatus {
            sid: "first_job".to_string(),
            is_done: true,
            is_finalized: false,
            done_progress: 1.0,
            run_duration: 10.0,
            disk_usage: 1024,
            scan_count: 100,
            event_count: 50,
            result_count: 25,
            cursor_time: None,
            priority: None,
            label: None,
        },
        SearchJobStatus {
            sid: "target_job".to_string(),
            is_done: false,
            is_finalized: false,
            done_progress: 0.5,
            run_duration: 20.0,
            disk_usage: 2048,
            scan_count: 200,
            event_count: 100,
            result_count: 50,
            cursor_time: None,
            priority: None,
            label: None,
        },
        SearchJobStatus {
            sid: "last_job".to_string(),
            is_done: false,
            is_finalized: false,
            done_progress: 0.3,
            run_duration: 30.0,
            disk_usage: 3072,
            scan_count: 300,
            event_count: 150,
            result_count: 75,
            cursor_time: None,
            priority: None,
            label: None,
        },
    ];
    app.update(Action::JobsLoaded(Ok(jobs)));

    // Apply filter matching only "target_job" using filter mode
    let action = app.handle_input(key('/'));
    if let Some(a) = action {
        app.update(a);
    }
    app.handle_input(key('t'));
    app.handle_input(key('a'));
    app.handle_input(key('r'));
    app.handle_input(key('g'));
    app.handle_input(key('e'));
    app.handle_input(key('t'));
    app.handle_input(enter_key());

    // Select the first item in filtered view (which is target_job)
    app.jobs_state.select(Some(0));

    // Open cancel popup
    app.handle_input(key('c'));

    // Verify the popup shows the correct job SID
    assert!(app.popup.is_some(), "Popup should be open");
    if let Some(Popup {
        kind: PopupType::ConfirmCancel(sid),
        ..
    }) = &app.popup
    {
        assert_eq!(
            sid, "target_job",
            "Cancel popup should show target_job, not first_job"
        );
    } else {
        panic!("Should have ConfirmCancel popup");
    }
}

#[test]
fn test_filtered_job_selection_delete() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Create jobs
    let jobs = vec![
        SearchJobStatus {
            sid: "keep_this".to_string(),
            is_done: true,
            is_finalized: false,
            done_progress: 1.0,
            run_duration: 10.0,
            disk_usage: 1024,
            scan_count: 100,
            event_count: 50,
            result_count: 25,
            cursor_time: None,
            priority: None,
            label: None,
        },
        SearchJobStatus {
            sid: "delete_this".to_string(),
            is_done: false,
            is_finalized: false,
            done_progress: 0.5,
            run_duration: 20.0,
            disk_usage: 2048,
            scan_count: 200,
            event_count: 100,
            result_count: 50,
            cursor_time: None,
            priority: None,
            label: None,
        },
    ];
    app.update(Action::JobsLoaded(Ok(jobs)));

    // Apply filter using filter mode
    let action = app.handle_input(key('/'));
    if let Some(a) = action {
        app.update(a);
    }
    app.handle_input(key('d'));
    app.handle_input(key('e'));
    app.handle_input(key('l'));
    app.handle_input(key('e'));
    app.handle_input(key('t'));
    app.handle_input(key('e'));
    app.handle_input(enter_key());

    app.jobs_state.select(Some(0));

    // Open delete popup
    app.handle_input(key('d'));

    // Verify the popup shows the correct job SID
    assert!(app.popup.is_some(), "Popup should be open");
    if let Some(Popup {
        kind: PopupType::ConfirmDelete(sid),
        ..
    }) = &app.popup
    {
        assert_eq!(
            sid, "delete_this",
            "Delete popup should show delete_this, not keep_this"
        );
    } else {
        panic!("Should have ConfirmDelete popup");
    }
}

#[test]
fn test_filtered_navigation_respects_bounds() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Create 10 jobs
    let jobs = create_mock_jobs(10);
    app.update(Action::JobsLoaded(Ok(jobs)));

    // Apply filter that matches only 3 jobs (sid_0, sid_1, sid_2)
    let action = app.handle_input(key('/'));
    if let Some(a) = action {
        app.update(a);
    }
    app.handle_input(key('s'));
    app.handle_input(key('i'));
    app.handle_input(key('d'));
    app.handle_input(key('_'));
    app.handle_input(enter_key());

    // Should have filtered indices
    assert!(
        app.filtered_job_indices.len() >= 3,
        "Should have at least 3 filtered jobs"
    );

    // Select first item
    app.jobs_state.select(Some(0));

    // Navigate down - should respect filtered bounds
    app.update(Action::NavigateDown);
    let selected = app.jobs_state.selected().unwrap();
    assert!(
        selected < app.filtered_job_indices.len(),
        "Selection should be within filtered bounds"
    );
}
