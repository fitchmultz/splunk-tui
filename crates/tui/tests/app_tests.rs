//! Unit tests for App state machine and input handling.
//!
//! Tests cover:
//! - Popup state transitions (cancel/confirm)
//! - Selection preservation during data updates
//! - Auto-refresh suppression during popups
//! - Navigation boundary behavior

mod helpers;
use helpers::*;
use splunk_client::models::SearchJobStatus;
use splunk_tui::{CurrentScreen, Popup, PopupType, Toast, ToastLevel, action::Action, app::App};
use std::time::Duration;

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
fn test_popup_cancel_flow() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(3));
    app.jobs_state.select(Some(1));

    // Open cancel popup by pressing 'c'
    let action = app.handle_input(key('c'));
    assert!(action.is_none(), "Opening popup should not return action");
    assert!(app.popup.is_some(), "Popup should be open");
    assert!(
        matches!(
            app.popup.as_ref().map(|p| &p.kind),
            Some(PopupType::ConfirmCancel(_))
        ),
        "Should be ConfirmCancel popup"
    );

    // Press 'n' to cancel
    let action = app.handle_input(key('n'));
    assert!(action.is_none(), "Canceling popup should not return action");
    assert!(app.popup.is_none(), "Popup should be closed");

    // Verify selection is preserved
    assert_eq!(
        app.jobs_state.selected(),
        Some(1),
        "Selection should be preserved"
    );
}

#[test]
fn test_popup_cancel_with_escape() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(3));
    app.jobs_state.select(Some(1));

    // Open cancel popup
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Press Esc to cancel
    let action = app.handle_input(esc_key());
    assert!(
        action.is_none(),
        "Canceling with Esc should not return action"
    );
    assert!(app.popup.is_none(), "Popup should be closed");
}

#[test]
fn test_popup_confirm_cancel_action() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(3));
    app.jobs_state.select(Some(1));
    let expected_sid = "sid_1".to_string();

    // Open cancel popup
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Press 'y' to confirm
    let action = app.handle_input(key('y'));
    assert!(action.is_some(), "Confirming should return action");
    assert!(matches!(action, Some(Action::CancelJob(sid)) if sid == expected_sid));
    assert!(app.popup.is_none(), "Popup should be closed after confirm");
}

#[test]
fn test_popup_confirm_with_enter() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(3));
    app.jobs_state.select(Some(1));
    let expected_sid = "sid_1".to_string();

    // Open cancel popup
    app.handle_input(key('c'));

    // Press Enter to confirm
    let action = app.handle_input(enter_key());
    assert!(
        action.is_some(),
        "Confirming with Enter should return action"
    );
    assert!(matches!(action, Some(Action::CancelJob(sid)) if sid == expected_sid));
}

#[test]
fn test_popup_delete_confirm_action() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(3));
    app.jobs_state.select(Some(2));
    let expected_sid = "sid_2".to_string();

    // Open delete popup by pressing 'd'
    app.handle_input(key('d'));
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::ConfirmDelete(_))
    ));

    // Press 'y' to confirm
    let action = app.handle_input(key('y'));
    assert!(action.is_some(), "Confirming delete should return action");
    assert!(matches!(action, Some(Action::DeleteJob(sid)) if sid == expected_sid));
}

#[test]
fn test_jobs_loaded_preserves_selection() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(10));
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
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(5));
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
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.auto_refresh = true;

    // Without popup, tick should return LoadJobs action
    let action = app.handle_tick();
    assert!(
        matches!(action, Some(Action::LoadJobs)),
        "Tick should return LoadJobs when no popup"
    );

    // Open a popup
    app.popup = Some(Popup::builder(PopupType::Help).build());

    // With popup, tick should return None
    let action = app.handle_tick();
    assert!(action.is_none(), "Tick should be suppressed during popup");
}

#[test]
fn test_navigation_down_at_boundary() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(3));
    app.jobs_state.select(Some(2)); // Already at last item

    // Try to navigate down from last item
    app.update(Action::NavigateDown);

    // Should stay at last item (index 2)
    assert_eq!(
        app.jobs_state.selected(),
        Some(2),
        "Should stay at last item"
    );
}

#[test]
fn test_navigation_up_at_boundary() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(3));
    app.jobs_state.select(Some(0)); // Already at first item

    // Try to navigate up from first item
    app.update(Action::NavigateUp);

    // Should stay at first item (index 0)
    assert_eq!(
        app.jobs_state.selected(),
        Some(0),
        "Should stay at first item"
    );
}

#[test]
fn test_navigation_down_normal() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(5));
    app.jobs_state.select(Some(1));

    // Navigate down
    app.update(Action::NavigateDown);

    assert_eq!(app.jobs_state.selected(), Some(2), "Should move to index 2");
}

#[test]
fn test_navigation_up_normal() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(5));
    app.jobs_state.select(Some(3));

    // Navigate up
    app.update(Action::NavigateUp);

    assert_eq!(app.jobs_state.selected(), Some(2), "Should move to index 2");
}

#[test]
fn test_help_popup_open_close() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Search;

    // Open help popup
    let action = app.handle_input(key('?'));
    assert!(action.is_none(), "Opening help should not return action");
    assert!(
        matches!(app.popup.as_ref().map(|p| &p.kind), Some(PopupType::Help)),
        "Should open Help popup"
    );

    // Close with Esc
    let action = app.handle_input(esc_key());
    assert!(action.is_none(), "Closing help should not return action");
    assert!(app.popup.is_none(), "Popup should be closed");

    // Reopen with '?'
    app.handle_input(key('?'));
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::Help)
    ));

    // Close with 'q'
    let action = app.handle_input(key('q'));
    assert!(
        action.is_none(),
        "Closing help with 'q' should not return action"
    );
    assert!(app.popup.is_none(), "Popup should be closed");
}

#[test]
fn test_page_down_navigation() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(25));
    app.jobs_state.select(Some(5));

    // Page down
    app.update(Action::PageDown);

    // Should move to index 15 (5 + 10)
    assert_eq!(
        app.jobs_state.selected(),
        Some(15),
        "Should page down by 10"
    );
}

#[test]
fn test_page_up_navigation() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(25));
    app.jobs_state.select(Some(20));

    // Page up
    app.update(Action::PageUp);

    // Should move to index 10 (20 - 10)
    assert_eq!(app.jobs_state.selected(), Some(10), "Should page up by 10");
}

#[test]
fn test_go_to_top() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(10));
    app.jobs_state.select(Some(7));

    // Go to top
    app.update(Action::GoToTop);

    assert_eq!(
        app.jobs_state.selected(),
        Some(0),
        "Should go to top (index 0)"
    );
}

#[test]
fn test_go_to_bottom() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(10));
    app.jobs_state.select(Some(2));

    // Go to bottom
    app.update(Action::GoToBottom);

    assert_eq!(
        app.jobs_state.selected(),
        Some(9),
        "Should go to bottom (last index)"
    );
}

#[test]
fn test_toggle_auto_refresh() {
    let mut app = App::new(None);
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
fn test_screen_navigation_with_number_keys() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Search;

    // Navigate to Jobs with '4'
    let action = app.handle_input(key('4'));
    assert!(
        matches!(action, Some(Action::LoadJobs)),
        "Should trigger LoadJobs"
    );
    assert_eq!(
        app.current_screen,
        CurrentScreen::Jobs,
        "Should switch to Jobs screen"
    );
}

#[test]
fn test_quit_action() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;

    // Press 'q' to quit
    let action = app.handle_input(key('q'));
    assert!(
        matches!(action, Some(Action::Quit)),
        "Should return Quit action"
    );
}

#[test]
fn test_refresh_jobs_action() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;

    // Press 'r' to refresh
    let action = app.handle_input(key('r'));
    assert!(
        matches!(action, Some(Action::LoadJobs)),
        "Should return LoadJobs action"
    );
}

#[test]
fn test_notify_adds_toast() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;

    // Add a toast notification
    app.update(Action::Notify(ToastLevel::Error, "Test error".to_string()));

    assert_eq!(app.toasts.len(), 1, "Should have one toast");
    assert_eq!(
        app.toasts[0].message, "Test error",
        "Toast message should match"
    );
    assert_eq!(
        app.toasts[0].level,
        ToastLevel::Error,
        "Toast level should be Error"
    );
}

#[test]
fn test_tick_prunes_expired_toasts() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;

    // Add a toast
    app.toasts.push(Toast::error("Test".to_string()));

    // Manually expire it
    app.toasts[0].ttl = Duration::from_millis(1);
    std::thread::sleep(Duration::from_millis(10));

    // Tick should prune expired toasts
    app.update(Action::Tick);

    assert!(app.toasts.is_empty(), "Expired toasts should be pruned");
}

#[test]
fn test_progress_update() {
    let mut app = App::new(None);

    // Update progress
    app.update(Action::Progress(0.75));

    assert_eq!(app.progress, 0.75, "Progress should be updated");
}

#[test]
fn test_indexes_navigation() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Indexes;
    app.indexes = Some(vec![
        splunk_client::models::Index {
            name: "index1".to_string(),
            total_event_count: 100,
            current_db_size_mb: 10,
            max_total_data_size_mb: None,
            max_warm_db_count: None,
            max_hot_buckets: None,
            frozen_time_period_in_secs: None,
            cold_db_path: None,
            home_path: None,
            thawed_path: None,
            cold_to_frozen_dir: None,
            primary_index: None,
        },
        splunk_client::models::Index {
            name: "index2".to_string(),
            total_event_count: 200,
            current_db_size_mb: 20,
            max_total_data_size_mb: None,
            max_warm_db_count: None,
            max_hot_buckets: None,
            frozen_time_period_in_secs: None,
            cold_db_path: None,
            home_path: None,
            thawed_path: None,
            cold_to_frozen_dir: None,
            primary_index: None,
        },
    ]);
    app.indexes_state.select(Some(0));

    // Navigate down
    app.update(Action::NavigateDown);
    assert_eq!(
        app.indexes_state.selected(),
        Some(1),
        "Should move to index 1"
    );

    // Navigate up
    app.update(Action::NavigateUp);
    assert_eq!(
        app.indexes_state.selected(),
        Some(0),
        "Should move to index 0"
    );
}

#[test]
fn test_job_inspection_flow() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = Some(create_mock_jobs(3));
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
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;
    app.jobs = None; // No jobs loaded
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
fn test_job_inspect_help_popup() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::JobInspect;

    // Open help popup with '?'
    let action = app.handle_input(key('?'));
    assert!(action.is_none(), "Opening help should not return action");
    assert!(
        matches!(app.popup.as_ref().map(|p| &p.kind), Some(PopupType::Help)),
        "Should open Help popup"
    );

    // Close with Esc
    let action = app.handle_input(esc_key());
    assert!(action.is_none(), "Closing help should not return action");
    assert!(app.popup.is_none(), "Popup should be closed");
}

#[test]
fn test_jobs_filter_persistence() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;

    // Enter filter mode with '/'
    let action = app.handle_input(key('/'));
    assert!(
        action.is_none(),
        "Entering filter mode should not return action"
    );
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
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;

    // Set an existing filter
    app.search_filter = Some("existing".to_string());

    // Enter filter mode
    app.handle_input(key('/'));
    assert!(app.is_filtering);

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
fn test_jobs_filter_cancel_with_escape() {
    let mut app = App::new(None);
    app.current_screen = CurrentScreen::Jobs;

    // Set an existing filter
    app.search_filter = Some("existing".to_string());

    // Enter filter mode
    app.handle_input(key('/'));
    assert!(app.is_filtering);

    // Type some text
    app.handle_input(key('f'));
    app.handle_input(key('o'));
    app.handle_input(key('o'));
    assert_eq!(app.filter_input, "foo");

    // Press Esc to cancel without applying
    let action = app.handle_input(esc_key());

    // Esc should return ClearSearch
    assert!(
        matches!(action, Some(Action::ClearSearch)),
        "Esc should return ClearSearch"
    );
    assert!(!app.is_filtering, "Should exit filter mode");
    assert!(
        app.filter_input.is_empty(),
        "Filter input should be cleared"
    );
    // The existing filter should be cleared (current behavior)
    // This is because ClearSearch sets search_filter to None
}
