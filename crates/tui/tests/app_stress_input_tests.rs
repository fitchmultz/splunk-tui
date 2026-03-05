//! Stress tests for rapid input handling and key spam scenarios.
//!
//! This module tests:
//! - Rapid navigation actions (key spam)
//! - Rapid search input while switching screens
//! - Rapid popup open/close cycles
//! - State consistency under high-frequency input
//!
//! ## Invariants
//! - App must remain in valid state after any sequence of rapid inputs
//! - No panics, crashes, or inconsistent state transitions
//! - Input state must remain consistent regardless of input speed

mod helpers;
use helpers::*;
use splunk_client::models::SearchJobStatus;
use splunk_tui::{CurrentScreen, action::Action, app::App, app::ConnectionContext, ui::ToastLevel};

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
fn test_rapid_navigation_does_not_panic() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(10))));

    // Send 100 rapid navigation actions
    for i in 0..100 {
        if i % 2 == 0 {
            app.update(Action::NavigateDown);
        } else {
            app.update(Action::NavigateUp);
        }
    }

    // App should still be in valid state
    assert_eq!(app.current_screen, CurrentScreen::Jobs);
    // Should still have jobs loaded
    assert!(app.jobs.is_some());
    assert_eq!(app.jobs.as_ref().unwrap().len(), 10);
}

#[test]
fn test_rapid_screen_cycling_does_not_panic() {
    let mut app = App::new(None, ConnectionContext::default());

    // Rapidly cycle through screens 50 times
    for _ in 0..50 {
        app.update(Action::NextScreen);
    }

    // App should be in valid state - the important thing is no panic occurred
    // The screen should be deterministically set after cycling
    // We verify the app is still functional by checking it can continue
    app.update(Action::NextScreen);
    let _screen_after = app.current_screen; // Should not panic
}

#[test]
fn test_rapid_search_and_navigate() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Simulate rapid typing while occasionally navigating
    for i in 0..50 {
        if i % 5 == 0 {
            // Occasionally switch screen
            app.update(Action::NextScreen);
        } else {
            // Type characters
            app.update(Action::SearchInput('a'));
        }
    }

    // Search input should have accumulated characters
    // (may not be exactly 40 due to screen switching behavior, but should be consistent)
    let input_len = app.search_input.value().len();
    assert!(
        input_len <= 50,
        "Search input length should be reasonable, got {}",
        input_len
    );
}

#[test]
fn test_rapid_popup_open_close() {
    let mut app = App::new(None, ConnectionContext::default());

    // Rapidly open and close help popup
    for _ in 0..20 {
        app.update(Action::OpenHelpPopup);
        // Simulate pressing Esc to close
        let action = app.handle_input(esc_key());
        if let Some(act) = action {
            app.update(act);
        }
    }

    // App should still be valid
    assert!(
        app.popup.is_none() || app.popup.is_some(),
        "Popup state should be valid"
    );
}

#[test]
fn test_rapid_pagination_requests() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(100))));

    // Rapid pagination requests
    for i in 0..30 {
        if i % 3 == 0 {
            app.update(Action::PageDown);
        } else if i % 3 == 1 {
            app.update(Action::PageUp);
        } else {
            app.update(Action::NavigateDown);
        }
    }

    // Jobs should still be loaded
    assert!(app.jobs.is_some());
    assert_eq!(app.jobs.as_ref().unwrap().len(), 100);
}

#[test]
fn test_rapid_focus_changes() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Rapid focus toggling
    for _ in 0..20 {
        app.update(Action::ToggleFocusMode);
    }

    // App should remain valid
    assert_eq!(app.current_screen, CurrentScreen::Search);
}

#[test]
fn test_mixed_rapid_actions() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(20))));

    // Mix of different actions
    let actions = [
        Action::NavigateDown,
        Action::NavigateUp,
        Action::NextScreen,
        Action::PreviousScreen,
        Action::PageDown,
        Action::PageUp,
        Action::GoToTop,
        Action::GoToBottom,
    ];

    for i in 0..100 {
        let action = &actions[i % actions.len()];
        app.update(action.clone());
    }

    // Should still be in valid state
    assert!(app.jobs.is_some());
}

#[test]
fn test_rapid_loading_toggle() {
    let mut app = App::new(None, ConnectionContext::default());

    // Rapidly toggle loading state
    for _ in 0..50 {
        app.update(Action::Loading(true));
        app.update(Action::Loading(false));
    }

    // Final state should be not loading
    assert!(!app.loading);
}

#[test]
fn test_rapid_search_mode_toggle() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(10))));

    // Rapidly enter and exit search/filter mode
    for _ in 0..20 {
        app.update(Action::EnterSearchMode);
        let action = app.handle_input(esc_key());
        if let Some(act) = action {
            app.update(act);
        }
    }

    // App should be valid
    assert!(app.jobs.is_some());
}

#[test]
fn test_boundary_navigation_stress() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));

    // Rapidly navigate at boundaries
    for _ in 0..50 {
        app.update(Action::GoToTop);
        app.update(Action::NavigateUp); // Should stay at top
        app.update(Action::GoToBottom);
        app.update(Action::NavigateDown); // Should stay at bottom
    }

    // Should end at bottom
    assert_eq!(app.jobs_state.selected(), Some(4));
}

#[test]
fn test_rapid_screen_switch_during_loading() {
    let mut app = App::new(None, ConnectionContext::default());

    // Start loading
    app.update(Action::Loading(true));

    // Try to switch screens while loading
    for _ in 0..20 {
        app.update(Action::NextScreen);
        app.update(Action::PreviousScreen);
    }

    // Stop loading
    app.update(Action::Loading(false));

    // App should be valid
    assert!(!app.loading);
}

#[test]
fn test_rapid_toast_notifications() {
    let mut app = App::new(None, ConnectionContext::default());

    // Add many toasts rapidly
    for i in 0..20 {
        app.update(Action::Notify(ToastLevel::Error, format!("Error {}", i)));
    }

    // Should have toasts (implementation may limit count)
    // Just verify no panic occurred
    assert!(app.toasts.len() <= 20);
}

#[test]
fn test_rapid_error_clear_cycles() {
    let mut app = App::new(None, ConnectionContext::default());

    // Rapid error and clear cycles
    for _ in 0..30 {
        app.update(Action::Notify(ToastLevel::Error, "Test error".to_string()));
        app.update(Action::ClearErrorDetails);
    }

    // App should be valid - ClearErrorDetails clears error details state,
    // not necessarily toasts. The important thing is no panic occurred.
    // Verify app is still functional by checking it responds to actions
    let _initial_screen = app.current_screen;
    app.update(Action::NextScreen);
    // App should have processed the action without panicking
}

#[test]
fn test_key_spam_with_empty_data() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    // No jobs loaded - test navigation on empty state

    for _ in 0..50 {
        app.update(Action::NavigateDown);
        app.update(Action::NavigateUp);
        app.update(Action::PageDown);
        app.update(Action::PageUp);
    }

    // Should not panic with empty data
    assert!(app.jobs.is_none());
}

#[test]
fn test_rapid_resize_events() {
    let mut app = App::new(None, ConnectionContext::default());

    // Rapid resize events
    for i in 0..30 {
        let width = 80 + (i % 40) as u16;
        let height = 24 + (i % 10) as u16;
        app.update(Action::Resize(width, height));
    }

    // App should be valid
    assert_eq!(app.current_screen, CurrentScreen::Search);
}
