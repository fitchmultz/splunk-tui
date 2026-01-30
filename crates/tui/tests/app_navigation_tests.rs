//! Tests for screen/tab navigation and list navigation.
//!
//! This module tests:
//! - Tab/Shift+Tab screen cycling
//! - JobInspect exclusion from cycle
//! - Up/down/page navigation
//! - Go to top/bottom navigation
//! - Boundary behavior
//!
//! ## Invariants
//! - Tab on non-Search screens must navigate to next screen
//! - Tab on Search screen in QueryFocused mode must toggle input mode
//! - Tab on Search screen in ResultsFocused mode must navigate
//! - JobInspect must not participate in tab cycling
//!
//! ## Test Organization
//! Tests are grouped by navigation type: screen cycling, list navigation.

mod helpers;
use helpers::*;
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
fn test_navigation_down_at_boundary() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
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
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
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
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));
    app.jobs_state.select(Some(1));

    // Navigate down
    app.update(Action::NavigateDown);

    assert_eq!(app.jobs_state.selected(), Some(2), "Should move to index 2");
}

#[test]
fn test_navigation_up_normal() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));
    app.jobs_state.select(Some(3));

    // Navigate up
    app.update(Action::NavigateUp);

    assert_eq!(app.jobs_state.selected(), Some(2), "Should move to index 2");
}

#[test]
fn test_page_down_navigation() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(25))));
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
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(25))));
    app.jobs_state.select(Some(20));

    // Page up
    app.update(Action::PageUp);

    // Should move to index 10 (20 - 10)
    assert_eq!(app.jobs_state.selected(), Some(10), "Should page up by 10");
}

#[test]
fn test_go_to_top() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(10))));
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
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(10))));
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
fn test_screen_navigation_with_tab() {
    // Tab on non-Search screens navigates to next screen
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Indexes;

    // Navigate to Cluster with Tab
    let action = app.handle_input(tab_key());
    assert!(
        matches!(action, Some(Action::NextScreen)),
        "Should trigger NextScreen"
    );
    app.update(action.unwrap());
    assert_eq!(
        app.current_screen,
        CurrentScreen::Cluster,
        "Should switch to Cluster screen"
    );
}

#[test]
fn test_tab_navigates_to_next_screen() {
    // Tab on non-Search screens navigates to next screen
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Indexes;

    let action = app.handle_input(tab_key());
    assert!(matches!(action, Some(Action::NextScreen)));
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Cluster);
}

#[test]
fn test_tab_on_search_toggles_input_mode() {
    // Tab on Search screen toggles between QueryFocused and ResultsFocused modes
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Initial state: QueryFocused
    assert!(matches!(
        app.search_input_mode,
        splunk_tui::SearchInputMode::QueryFocused
    ));

    // Tab toggles to ResultsFocused (no action returned, just mode change)
    let action = app.handle_input(tab_key());
    assert!(
        action.is_none(),
        "Tab on Search should not return action, just toggle mode"
    );
    assert!(matches!(
        app.search_input_mode,
        splunk_tui::SearchInputMode::ResultsFocused
    ));
}

#[test]
fn test_shift_tab_navigates_to_previous_screen() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Indexes;

    let action = app.handle_input(shift_tab_key());
    assert!(matches!(action, Some(Action::PreviousScreen)));
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Search);
}

#[test]
fn test_tab_cycles_through_screens() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Overview;

    // Tab from Overview should wrap to Search
    let action = app.handle_input(tab_key());
    assert!(matches!(action, Some(Action::NextScreen)));
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Search);

    // On Search screen, Tab toggles input mode instead of navigating
    // Switch to ResultsFocused mode first so Tab can navigate
    app.search_input_mode = splunk_tui::SearchInputMode::ResultsFocused;

    // Tab from Search (in ResultsFocused mode) should go to Indexes
    let action = app.handle_input(tab_key());
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Indexes);
}

#[test]
fn test_shift_tab_cycles_backwards() {
    let mut app = App::new(None, ConnectionContext::default());

    // Start from Indexes (not Search) to avoid input mode complications
    app.current_screen = CurrentScreen::Indexes;

    // Shift+Tab from Indexes should go to Search
    let action = app.handle_input(shift_tab_key());
    assert!(matches!(action, Some(Action::PreviousScreen)));
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Search);

    // On Search screen in QueryFocused mode (default), Shift+Tab toggles input mode
    // Switch to ResultsFocused mode first so Shift+Tab can navigate
    app.search_input_mode = splunk_tui::SearchInputMode::ResultsFocused;

    // Shift+Tab from Search (in ResultsFocused mode) should wrap to Overview
    let action = app.handle_input(shift_tab_key());
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Overview);

    // Shift+Tab from Overview should go to Settings
    let action = app.handle_input(shift_tab_key());
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Settings);

    // Shift+Tab from Settings should go to Configs
    let action = app.handle_input(shift_tab_key());
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Configs);

    // Shift+Tab from Configs should go to Inputs
    let action = app.handle_input(shift_tab_key());
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Inputs);

    // Shift+Tab from Inputs should go to SearchPeers
    let action = app.handle_input(shift_tab_key());
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::SearchPeers);

    // Shift+Tab from SearchPeers should go to Users
    let action = app.handle_input(shift_tab_key());
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Users);
}

#[test]
fn test_navigation_from_all_screens() {
    // Test screens where Tab always navigates (non-Search screens)
    let screens = [
        CurrentScreen::Indexes,
        CurrentScreen::Cluster,
        CurrentScreen::Jobs,
        CurrentScreen::Health,
        CurrentScreen::SavedSearches,
        CurrentScreen::InternalLogs,
        CurrentScreen::Apps,
        CurrentScreen::Users,
        CurrentScreen::SearchPeers,
        CurrentScreen::Inputs,
        CurrentScreen::Configs,
        CurrentScreen::Settings,
    ];

    for screen in screens {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = screen;

        // Tab should work from all non-Search screens
        let action = app.handle_input(tab_key());
        assert!(
            matches!(action, Some(Action::NextScreen)),
            "Tab should work from {:?} screen",
            screen
        );
        app.update(action.unwrap());
        assert_ne!(
            app.current_screen, screen,
            "Screen should change from {:?}",
            screen
        );
    }

    // Test Search screen: Tab toggles input mode in QueryFocused mode
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // In QueryFocused mode (default), Tab toggles input mode
    assert!(matches!(
        app.search_input_mode,
        splunk_tui::SearchInputMode::QueryFocused
    ));
    let action = app.handle_input(tab_key());
    assert!(
        action.is_none(),
        "Tab on Search in QueryFocused mode should toggle input mode, not navigate"
    );
    assert!(
        matches!(
            app.search_input_mode,
            splunk_tui::SearchInputMode::ResultsFocused
        ),
        "Tab should toggle to ResultsFocused mode"
    );

    // In ResultsFocused mode, Tab navigates to next screen
    let action = app.handle_input(tab_key());
    assert!(
        matches!(action, Some(Action::NextScreen)),
        "Tab on Search in ResultsFocused mode should navigate"
    );
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Indexes);
}

#[test]
fn test_job_inspect_excluded_from_cycle() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::JobInspect;

    // Tab from JobInspect should go to Jobs (not cycle)
    let action = app.handle_input(tab_key());
    assert!(matches!(action, Some(Action::NextScreen)));
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Jobs);

    // Shift+Tab from JobInspect should also go to Jobs
    app.current_screen = CurrentScreen::JobInspect;
    let action = app.handle_input(shift_tab_key());
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Jobs);
}
