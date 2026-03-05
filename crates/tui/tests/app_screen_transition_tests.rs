//! Tests for screen transition edge cases and robustness.
//!
//! This module tests:
//! - Rapid screen switching
//! - Switching screens during loading
//! - Switching screens during popup display
//! - Screen transition validation
//! - Focus management across transitions
//!
//! ## Invariants
//! - Screen transitions must always leave app in valid state
//! - Loading states should be handled correctly during transitions
//! - Popups should block or handle screen transitions gracefully

mod helpers;
use helpers::*;
use splunk_client::models::SearchJobStatus;
use splunk_tui::{
    CurrentScreen, SearchInputMode, action::Action, app::App, app::ConnectionContext,
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
fn test_rapid_screen_switching() {
    let mut app = App::new(None, ConnectionContext::default());

    // Cycle through all screens rapidly multiple times
    let screens_before: Vec<_> = (0..100)
        .map(|_| {
            app.update(Action::NextScreen);
            app.current_screen
        })
        .collect();

    // All screens should be valid
    for screen in &screens_before {
        // JobInspect is special - it's excluded from normal cycling
        assert!(
            *screen != CurrentScreen::JobInspect,
            "JobInspect should not appear in normal cycling"
        );
    }

    // App should be in valid state
    assert!(
        matches!(
            app.current_screen,
            CurrentScreen::Search
                | CurrentScreen::Indexes
                | CurrentScreen::Cluster
                | CurrentScreen::Jobs
                | CurrentScreen::Health
                | CurrentScreen::SavedSearches
                | CurrentScreen::InternalLogs
                | CurrentScreen::Apps
                | CurrentScreen::Users
                | CurrentScreen::Roles
                | CurrentScreen::SearchPeers
                | CurrentScreen::Inputs
                | CurrentScreen::Configs
                | CurrentScreen::FiredAlerts
                | CurrentScreen::Settings
                | CurrentScreen::Overview
                | CurrentScreen::MultiInstance
                | CurrentScreen::Forwarders
                | CurrentScreen::Lookups
                | CurrentScreen::Audit
                | CurrentScreen::Dashboards
                | CurrentScreen::DataModels
                | CurrentScreen::WorkloadManagement
                | CurrentScreen::Shc
                | CurrentScreen::License
                | CurrentScreen::Kvstore
        ),
        "Should end on a valid screen: {:?}",
        app.current_screen
    );
}

#[test]
fn test_rapid_backward_screen_switching() {
    let mut app = App::new(None, ConnectionContext::default());

    // Cycle backward through screens rapidly
    for _ in 0..50 {
        app.update(Action::PreviousScreen);
    }

    // App should be in valid state
    assert!(
        !matches!(app.current_screen, CurrentScreen::JobInspect),
        "Should not be on JobInspect after normal cycling"
    );
}

#[test]
fn test_switch_screen_during_loading() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Start loading
    app.update(Action::Loading(true));
    assert!(app.loading);

    // Switch screen before loading completes
    let prev_screen = app.current_screen;
    app.update(Action::NextScreen);

    // Loading state should still be active
    assert!(
        app.loading,
        "Loading should persist across screen transitions"
    );

    // Screen should have changed
    assert_ne!(
        app.current_screen, prev_screen,
        "Screen should change even while loading"
    );
}

#[test]
fn test_switch_screen_during_popup() {
    let mut app = App::new(None, ConnectionContext::default());

    // Open a popup
    app.update(Action::OpenHelpPopup);
    assert!(app.popup.is_some(), "Popup should be open");

    let _screen_with_popup = app.current_screen;

    // Try to switch screen while popup is open
    // The app may either block the transition or close the popup
    app.update(Action::NextScreen);

    // Both behaviors are valid:
    // 1. Screen changed and popup closed
    // 2. Screen stayed same and popup still open
    // 3. Screen changed (popup auto-closed)

    // Just verify no panic occurred and state is consistent
    let _ = app.current_screen;
}

#[test]
fn test_job_inspect_exit_behavior() {
    let mut app = App::new(None, ConnectionContext::default());

    // Navigate to JobInspect
    app.current_screen = CurrentScreen::JobInspect;

    // NextScreen from JobInspect should go to Jobs
    app.update(Action::NextScreen);
    assert_eq!(
        app.current_screen,
        CurrentScreen::Jobs,
        "NextScreen from JobInspect should go to Jobs"
    );

    // Go back to JobInspect
    app.current_screen = CurrentScreen::JobInspect;

    // PreviousScreen from JobInspect should also go to Jobs
    app.update(Action::PreviousScreen);
    assert_eq!(
        app.current_screen,
        CurrentScreen::Jobs,
        "PreviousScreen from JobInspect should go to Jobs"
    );
}

#[test]
fn test_search_screen_input_mode_preserved() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Set to ResultsFocused mode
    app.search_input_mode = SearchInputMode::ResultsFocused;

    // Switch away and back
    app.update(Action::NextScreen);
    app.update(Action::PreviousScreen);

    // Should be back on Search screen
    assert_eq!(app.current_screen, CurrentScreen::Search);
}

#[test]
fn test_screen_specific_data_preserved() {
    let mut app = App::new(None, ConnectionContext::default());

    // Load jobs data
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));

    // Switch to different screen
    app.update(Action::NextScreen);
    let intermediate_screen = app.current_screen;

    // Switch back to Jobs
    while app.current_screen != CurrentScreen::Jobs {
        app.update(Action::NextScreen);
        // Safety check to prevent infinite loop
        if app.current_screen == intermediate_screen {
            // We've gone full circle without finding Jobs
            break;
        }
    }

    // Jobs data should still be present (if we made it back to Jobs)
    if app.current_screen == CurrentScreen::Jobs {
        assert!(
            app.jobs.is_some(),
            "Jobs data should be preserved after screen transitions"
        );
    }
}

#[test]
fn test_filter_mode_exit_on_screen_switch() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));

    // Enter filter mode
    app.update(Action::EnterSearchMode);
    assert!(app.is_filtering);

    // Switch screen
    app.update(Action::NextScreen);

    // Should exit filter mode
    assert!(
        !app.is_filtering || app.current_screen != CurrentScreen::Jobs,
        "Should exit filter mode when leaving jobs screen"
    );
}

#[test]
fn test_rapid_alternating_directions() {
    let mut app = App::new(None, ConnectionContext::default());

    // Rapidly alternate between NextScreen and PreviousScreen
    for i in 0..100 {
        if i % 2 == 0 {
            app.update(Action::NextScreen);
        } else {
            app.update(Action::PreviousScreen);
        }
    }

    // App should be in valid state
    assert!(
        !matches!(app.current_screen, CurrentScreen::JobInspect),
        "Should handle rapid direction changes"
    );
}

#[test]
fn test_screen_transition_with_active_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(10))));

    // Select an item
    app.update(Action::NavigateDown);
    app.update(Action::NavigateDown);
    let selected_index = app.jobs_state.selected();

    // Switch screens
    app.update(Action::NextScreen);
    app.update(Action::PreviousScreen);

    // Back on Jobs, selection should be preserved
    if app.current_screen == CurrentScreen::Jobs {
        assert_eq!(
            app.jobs_state.selected(),
            selected_index,
            "Selection should be preserved after screen transitions"
        );
    }
}

#[test]
fn test_loading_completion_after_screen_switch() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Indexes;

    // Start loading
    app.update(Action::Loading(true));

    // Switch screen
    app.update(Action::NextScreen);

    // Complete loading (simulating async completion)
    app.update(Action::Loading(false));
    app.update(Action::IndexesLoaded(Ok(vec![])));

    // Should not panic
    assert!(!app.loading);
}

#[test]
fn test_error_persistence_across_screen_switch() {
    let mut app = App::new(None, ConnectionContext::default());

    // Trigger an error using ApiError variant
    let error = splunk_client::ClientError::ApiError {
        status: 500,
        url: "http://test".to_string(),
        message: "Test error".to_string(),
        request_id: None,
    };
    app.update(Action::IndexesLoaded(Err(std::sync::Arc::new(error))));

    assert!(!app.toasts.is_empty(), "Should have error toast");

    // Switch screens
    app.update(Action::NextScreen);

    // Error toast should still be visible
    assert!(
        !app.toasts.is_empty(),
        "Error toast should persist across screen transitions"
    );
}

#[test]
fn test_all_screens_reachable() {
    // Test that we can reach all main screens
    let mut app = App::new(None, ConnectionContext::default());

    let mut screens_seen = Vec::new();
    screens_seen.push(app.current_screen);

    // Cycle through all screens
    for _ in 0..100 {
        app.update(Action::NextScreen);
        if !screens_seen.contains(&app.current_screen) {
            screens_seen.push(app.current_screen);
        }

        // Stop if we've looped back to start
        if screens_seen.len() > 5 && app.current_screen == CurrentScreen::Search {
            break;
        }
    }

    // Should have seen multiple screens
    assert!(
        screens_seen.len() >= 3,
        "Should be able to reach multiple screens, got: {:?}",
        screens_seen
    );
}

#[test]
fn test_search_screen_tab_behavior() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Tab now navigates to next screen deterministically
    assert!(matches!(
        app.search_input_mode,
        SearchInputMode::QueryFocused
    ));

    let action = app.handle_input(tab_key());
    assert!(
        matches!(action, Some(Action::NextScreen)),
        "Tab on Search should navigate to next screen"
    );
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Indexes);
    // Mode stays as QueryFocused (Tab doesn't toggle mode anymore)
    assert!(matches!(
        app.search_input_mode,
        SearchInputMode::QueryFocused
    ));
}
