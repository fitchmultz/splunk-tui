//! Unit tests for App state machine and input handling.
//!
//! Tests cover:
//! - Popup state transitions (cancel/confirm)
//! - Selection preservation during data updates
//! - Auto-refresh suppression during popups
//! - Navigation boundary behavior

mod helpers;
use crossterm::event::KeyCode;
use helpers::*;
use ratatui::prelude::Rect;
use splunk_client::models::{App as SplunkApp, Index, SavedSearch, SearchJobStatus, User};
use splunk_tui::{
    CurrentScreen, Popup, PopupType, Toast, ToastLevel, action::Action, action::ExportFormat,
    app::App, app::ConnectionContext,
};
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

// ============================================================================
// KeyEventKind Filtering Tests (RQ-0107 fix)
// ============================================================================

// NOTE: The filtering for KeyEventKind happens in main.rs at the input task level.
// These tests verify that the helper functions create events with the correct kind.
// The app.handle_input() method does NOT check key.kind - it only looks at
// key.code and key.modifiers, which is why filtering must happen earlier in the pipeline.

#[test]
fn test_release_event_helper_creates_correct_kind() {
    let release = release_key('a');

    assert_eq!(release.kind, crossterm::event::KeyEventKind::Release);
    assert_eq!(release.code, KeyCode::Char('a'));
}

#[test]
fn test_repeat_event_helper_creates_correct_kind() {
    let repeat = repeat_key('b');

    assert_eq!(repeat.kind, crossterm::event::KeyEventKind::Repeat);
    assert_eq!(repeat.code, KeyCode::Char('b'));
}

#[allow(dead_code)]
#[test]
fn test_settings_screen_navigation() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Settings;

    // Test navigation with Tab - should wrap to Search
    let action = app.handle_input(tab_key());
    assert!(
        matches!(action, Some(Action::NextScreen)),
        "Tab from Settings should return NextScreen and wrap to Search"
    );

    app.update(action.unwrap());
    // Verify screen switched to Search (wraps around)
    assert_eq!(app.current_screen, CurrentScreen::Search);
}

#[test]
fn test_auto_refresh_toggle() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Settings;
    let initial = app.auto_refresh;

    // Toggle auto-refresh
    app.handle_input(key('a'));

    assert_ne!(app.auto_refresh, initial);
    assert_eq!(app.toasts.len(), 1, "Toast should be added");
}

#[test]
fn test_theme_cycle_from_settings() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Settings;

    let initial = app.color_theme;

    let action = app.handle_input(key('t'));
    assert!(matches!(action, Some(Action::CycleTheme)));

    app.update(action.unwrap());
    assert_ne!(app.color_theme, initial, "Theme should change immediately");

    // Persisted state should include theme
    let persisted = app.get_persisted_state();
    assert_eq!(persisted.selected_theme, app.color_theme);

    // New app should initialize from persisted state
    let app2 = App::new(Some(persisted), ConnectionContext::default());
    assert_eq!(app2.color_theme, app.color_theme);
}

#[test]
fn test_sort_column_cycle() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Settings;
    let initial = app.sort_state.column;

    // Cycle sort column 5 times should return to initial
    for _ in 0..5 {
        app.handle_input(key('s'));
    }
    assert_eq!(app.sort_state.column, initial);
}

#[test]
fn test_clear_search_history() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Settings;
    app.search_history = vec!["query1".to_string(), "query2".to_string()];

    // Clear history
    app.handle_input(key('c'));

    assert!(app.search_history.is_empty(), "History should be cleared");
    assert_eq!(app.toasts.len(), 1, "Toast should be added");
}

#[test]
fn test_popup_cancel_flow() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
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
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
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
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
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
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
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
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
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
fn test_help_popup_open_close() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Switch to ResultsFocused mode first (help only works in this mode on Search screen)
    app.search_input_mode = splunk_tui::SearchInputMode::ResultsFocused;

    // Open help popup
    let action = app.handle_input(key('?'));
    assert!(
        matches!(action, Some(Action::OpenHelpPopup)),
        "Opening help should return OpenHelpPopup action"
    );
    app.update(action.unwrap());
    assert!(
        matches!(app.popup.as_ref().map(|p| &p.kind), Some(PopupType::Help)),
        "Should open Help popup"
    );

    // Close with Esc
    let action = app.handle_input(esc_key());
    assert!(action.is_none(), "Closing help should not return action");
    assert!(app.popup.is_none(), "Popup should be closed");

    // Reopen with '?' (still in ResultsFocused mode)
    let action = app.handle_input(key('?'));
    assert!(matches!(action, Some(Action::OpenHelpPopup)));
    app.update(action.unwrap());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::Help)
    ));

    // Close with 'q' (still in ResultsFocused mode)
    let action = app.handle_input(key('q'));
    assert!(
        action.is_none(),
        "Closing help with 'q' should not return action"
    );
    assert!(app.popup.is_none(), "Popup should be closed");
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
    app.current_screen = CurrentScreen::Settings;

    // Tab from Settings should wrap to Search
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

    // Shift+Tab from Search (in ResultsFocused mode) should wrap to Settings
    let action = app.handle_input(shift_tab_key());
    app.update(action.unwrap());
    assert_eq!(app.current_screen, CurrentScreen::Settings);

    // Shift+Tab from Settings should go to Users
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

#[test]
fn test_quit_action() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Press 'q' to quit
    let action = app.handle_input(key('q'));
    assert!(
        matches!(action, Some(Action::Quit)),
        "Should return Quit action"
    );
}

#[test]
fn test_quit_keyboard_triggers_action() {
    // Test screens where 'q' always quits (non-Search screens)
    let screens = [
        CurrentScreen::Jobs,
        CurrentScreen::Indexes,
        CurrentScreen::Cluster,
        CurrentScreen::Health,
        CurrentScreen::SavedSearches,
        CurrentScreen::InternalLogs,
        CurrentScreen::Apps,
        CurrentScreen::Users,
        CurrentScreen::Settings,
    ];

    for screen in screens {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = screen;

        let action = app.handle_input(key('q'));
        assert!(
            matches!(action, Some(Action::Quit)),
            "Pressing 'q' on {:?} screen should return Quit action",
            screen
        );
    }

    // Test Search screen: 'q' quits only in ResultsFocused mode
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // In QueryFocused mode (default), 'q' should insert into query
    assert!(matches!(
        app.search_input_mode,
        splunk_tui::SearchInputMode::QueryFocused
    ));
    let action = app.handle_input(key('q'));
    assert!(
        action.is_none(),
        "Pressing 'q' on Search screen in QueryFocused mode should insert into query"
    );
    assert_eq!(
        app.search_input, "q",
        "'q' should be inserted into search input"
    );

    // In ResultsFocused mode, 'q' should quit
    app.search_input_mode = splunk_tui::SearchInputMode::ResultsFocused;
    let action = app.handle_input(key('q'));
    assert!(
        matches!(action, Some(Action::Quit)),
        "Pressing 'q' on Search screen in ResultsFocused mode should return Quit action"
    );
}

#[test]
fn test_quit_mouse_footer_click() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 100, 24);
    app.loading = false;

    // Use FooterLayout to calculate the correct quit button position
    use splunk_tui::app::footer_layout::FooterLayout;
    let layout = FooterLayout::calculate(false, 0.0, app.current_screen, app.last_area.width);

    // Click in the middle of the quit button (accounting for border)
    let click_col = layout.quit_start + 1 + 3; // +1 for border, +3 for middle of quit button
    let action = app.handle_mouse(mouse_click(click_col, 22));
    assert!(
        matches!(action, Some(Action::Quit)),
        "Clicking 'Quit' in footer should return Quit action (col={})",
        click_col
    );
}

#[test]
fn test_quit_mouse_footer_click_with_loading() {
    let mut app = App::new(None, ConnectionContext::default());
    app.last_area = Rect::new(0, 0, 100, 24);
    app.loading = true;
    app.progress = 1.0; // Set progress so loading text renders

    // Use FooterLayout to calculate the correct quit button position
    use splunk_tui::app::footer_layout::FooterLayout;
    let layout = FooterLayout::calculate(true, 1.0, app.current_screen, app.last_area.width);

    // Click in the middle of the quit button (accounting for border)
    let click_col = layout.quit_start + 1 + 3; // +1 for border, +3 for middle of quit button
    let action = app.handle_mouse(mouse_click(click_col, 22));
    assert!(
        matches!(action, Some(Action::Quit)),
        "Clicking 'Quit' in footer with loading offset should return Quit action (col={})",
        click_col
    );
}

#[test]
fn test_refresh_jobs_action() {
    let mut app = App::new(None, ConnectionContext::default());
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
    let mut app = App::new(None, ConnectionContext::default());
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
    let mut app = App::new(None, ConnectionContext::default());
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
    let mut app = App::new(None, ConnectionContext::default());

    // Update progress
    app.update(Action::Progress(0.75));

    assert_eq!(app.progress, 0.75, "Progress should be updated");
}

#[test]
fn test_indexes_navigation() {
    let mut app = App::new(None, ConnectionContext::default());
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
fn test_job_inspect_help_popup() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::JobInspect;

    // Open help popup with '?'
    let action = app.handle_input(key('?'));
    assert!(matches!(action, Some(Action::OpenHelpPopup)));
    app.update(action.unwrap());
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

// Regression tests for RQ-0021: loading state should be cleared on error

#[test]
fn test_cancel_job_error_clears_loading() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Simulate the sequence of actions when CancelJob fails:
    // 1. Loading(true) is sent before the async operation
    app.update(Action::Loading(true));
    assert!(app.loading, "Loading should be true after Loading(true)");

    // 2. Error notification is sent when operation fails
    app.update(Action::Notify(
        ToastLevel::Error,
        "Failed to cancel job: connection error".to_string(),
    ));

    // 3. Loading(false) is sent to clear the loading state
    app.update(Action::Loading(false));

    assert!(!app.loading, "Loading should be false after Loading(false)");
    assert_eq!(app.toasts.len(), 1, "Should have error toast");
    assert_eq!(
        app.toasts[0].level,
        ToastLevel::Error,
        "Toast should be Error level"
    );
}

#[test]
fn test_delete_job_error_clears_loading() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Simulate the sequence of actions when DeleteJob fails:
    // 1. Loading(true) is sent before the async operation
    app.update(Action::Loading(true));
    assert!(app.loading, "Loading should be true after Loading(true)");

    // 2. Error notification is sent when operation fails
    app.update(Action::Notify(
        ToastLevel::Error,
        "Failed to delete job: not found".to_string(),
    ));

    // 3. Loading(false) is sent to clear the loading state
    app.update(Action::Loading(false));

    assert!(!app.loading, "Loading should be false after Loading(false)");
    assert_eq!(app.toasts.len(), 1, "Should have error toast");
    assert_eq!(
        app.toasts[0].level,
        ToastLevel::Error,
        "Toast should be Error level"
    );
}

// Tests for search results scrolling (RQ-0026 fix)

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

// ============================================================================
// Clipboard (Ctrl+C) Tests
// ============================================================================

#[test]
fn test_ctrl_c_copies_search_query_when_no_results() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input = "index=_internal | head 5".to_string();
    app.search_results.clear();

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "index=_internal | head 5"),
        "Ctrl+C should emit CopyToClipboard(query)"
    );
}

#[test]
fn test_ctrl_c_copies_current_search_result_when_results_exist() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    let v = serde_json::json!({"foo":"bar","n":1});
    app.set_search_results(vec![v.clone()]);
    app.search_scroll_offset = 0;

    let expected = serde_json::to_string_pretty(&v).unwrap();
    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == expected),
        "Ctrl+C should emit CopyToClipboard(pretty_json)"
    );
}

#[test]
fn test_ctrl_c_copies_selected_job_sid() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "sid_1"),
        "Ctrl+C should copy selected job SID"
    );
}

#[test]
fn test_ctrl_c_copies_selected_index_name() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Indexes;
    app.indexes = Some(vec![Index {
        name: "main".to_string(),
        total_event_count: 1,
        current_db_size_mb: 1,
        max_total_data_size_mb: None,
        max_warm_db_count: None,
        max_hot_buckets: None,
        frozen_time_period_in_secs: None,
        cold_db_path: None,
        home_path: None,
        thawed_path: None,
        cold_to_frozen_dir: None,
        primary_index: None,
    }]);
    app.indexes_state.select(Some(0));

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "main"),
        "Ctrl+C should copy selected index name"
    );
}

#[test]
fn test_ctrl_c_copies_selected_saved_search_name() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::SavedSearches;
    app.saved_searches = Some(vec![SavedSearch {
        name: "Errors Last 24 Hours".to_string(),
        search: "index=_internal error".to_string(),
        description: None,
        disabled: false,
    }]);
    app.saved_searches_state.select(Some(0));

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "Errors Last 24 Hours"),
        "Ctrl+C should copy selected saved search name"
    );
}

#[test]
fn test_ctrl_c_copies_selected_app_name() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Apps;
    app.apps = Some(vec![SplunkApp {
        name: "search".to_string(),
        label: Some("Search".to_string()),
        version: Some("1.0.0".to_string()),
        is_configured: None,
        is_visible: None,
        disabled: false,
        description: None,
        author: None,
    }]);
    app.apps_state.select(Some(0));

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "search"),
        "Ctrl+C should copy selected app name"
    );
}

#[test]
fn test_ctrl_c_copies_selected_username() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Users;
    app.users = Some(vec![User {
        name: "admin".to_string(),
        realname: Some("Administrator".to_string()),
        email: None,
        user_type: None,
        default_app: None,
        roles: vec!["admin".to_string()],
        last_successful_login: None,
    }]);
    app.users_state.select(Some(0));

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "admin"),
        "Ctrl+C should copy selected username"
    );
}

#[test]
fn test_ctrl_c_copies_selected_log_message() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::InternalLogs;
    app.internal_logs = Some(vec![splunk_client::models::LogEntry {
        time: "2024-01-01 12:00:00".to_string(),
        index_time: String::new(),
        serial: None,
        level: "ERROR".to_string(),
        component: "Test".to_string(),
        message: "Something went wrong".to_string(),
    }]);
    app.internal_logs_state.select(Some(0));

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "Something went wrong"),
        "Ctrl+C should copy selected log message"
    );
}

#[test]
fn test_ctrl_c_copies_cluster_id() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Cluster;
    app.cluster_info = Some(splunk_client::models::ClusterInfo {
        id: "cluster-123".to_string(),
        label: None,
        mode: "master".to_string(),
        manager_uri: None,
        replication_factor: None,
        search_factor: None,
        status: None,
    });

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "cluster-123"),
        "Ctrl+C should copy cluster ID"
    );
}

#[test]
fn test_ctrl_c_copies_health_status() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Health;
    app.health_info = Some(splunk_client::models::HealthCheckOutput {
        server_info: None,
        splunkd_health: Some(splunk_client::models::SplunkHealth {
            health: "green".to_string(),
            features: std::collections::HashMap::new(),
        }),
        license_usage: None,
        kvstore_status: None,
        log_parsing_health: None,
    });

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "green"),
        "Ctrl+C should copy health status"
    );
}

#[test]
fn test_copy_to_clipboard_action_success_emits_info_toast_and_records_text() {
    let guard = splunk_tui::app::clipboard::install_recording_clipboard();

    let mut app = App::new(None, ConnectionContext::default());
    app.update(Action::CopyToClipboard("hello world".to_string()));

    assert!(
        guard.copied_text().as_deref() == Some("hello world"),
        "Recording clipboard should capture copied content"
    );
    assert!(!app.toasts.is_empty(), "Should emit a toast on success");
    assert_eq!(app.toasts.last().unwrap().level, ToastLevel::Info);
    assert!(
        app.toasts.last().unwrap().message.starts_with("Copied:"),
        "Success toast should begin with 'Copied:'"
    );
}

#[test]
fn test_copy_to_clipboard_action_failure_emits_error_toast() {
    let _guard = splunk_tui::app::clipboard::install_failing_clipboard("boom");

    let mut app = App::new(None, ConnectionContext::default());
    app.update(Action::CopyToClipboard("hello".to_string()));

    assert!(!app.toasts.is_empty(), "Should emit a toast on failure");
    assert_eq!(app.toasts.last().unwrap().level, ToastLevel::Error);
    assert!(
        app.toasts
            .last()
            .unwrap()
            .message
            .contains("Clipboard error: boom"),
        "Error toast should include clipboard error message"
    );
}

#[test]
fn test_typing_e_in_search_query_does_not_trigger_export() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Ensure export would be available if Ctrl+E were pressed.
    app.set_search_results(vec![serde_json::json!({"foo": "bar"})]);

    // Plain 'e' should type into the query, not open the export popup.
    app.search_input = "s".to_string();
    app.search_cursor_position = 1; // Cursor at end
    app.handle_input(key('e'));

    assert_eq!(app.search_input, "se", "Should append 'e' to query input");
    assert!(
        app.popup.is_none(),
        "Should not open export popup on plain 'e'"
    );
}

#[test]
fn test_export_search_popup_flow() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results(vec![serde_json::json!({"foo": "bar"})]);

    // Press Ctrl+e to open export popup
    app.handle_input(ctrl_key('e'));
    assert!(app.popup.is_some());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::ExportSearch)
    ));
    assert_eq!(app.export_input, "results.json");
    assert_eq!(app.export_format, ExportFormat::Json);

    // Press Tab to toggle format
    app.handle_input(tab_key());
    assert_eq!(app.export_format, ExportFormat::Csv);
    assert_eq!(app.export_input, "results.csv");

    // Toggle back to Json
    app.handle_input(tab_key());
    assert_eq!(app.export_format, ExportFormat::Json);
    assert_eq!(app.export_input, "results.json");

    // Toggle back to Csv for further testing
    app.handle_input(tab_key());
    assert_eq!(app.export_format, ExportFormat::Csv);
    assert_eq!(app.export_input, "results.csv");

    // Backspace and type new filename
    for _ in 0..12 {
        app.handle_input(backspace_key());
    }
    app.handle_input(key('d'));
    app.handle_input(key('a'));
    app.handle_input(key('t'));
    app.handle_input(key('a'));
    app.handle_input(key('.'));
    app.handle_input(key('c'));
    app.handle_input(key('s'));
    app.handle_input(key('v'));
    assert_eq!(app.export_input, "data.csv");

    // Press Enter to confirm export
    let action = app.handle_input(enter_key());
    assert!(action.is_some());
    if let Some(Action::ExportData(data, path, format)) = action {
        assert!(data.is_array());
        assert_eq!(path.to_str().unwrap(), "data.csv");
        assert_eq!(format, ExportFormat::Csv);
    } else {
        panic!("Should return ExportData action");
    }
    assert!(app.popup.is_none());
}

#[test]
fn test_export_search_disabled_when_no_results() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_results = Vec::new();

    // Press Ctrl+e - should not open popup
    app.handle_input(ctrl_key('e'));
    assert!(app.popup.is_none());
}

#[test]
fn test_export_search_cancel_with_esc() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.set_search_results(vec![serde_json::json!({"foo": "bar"})]);

    app.handle_input(ctrl_key('e'));
    assert!(app.popup.is_some());

    app.handle_input(esc_key());
    assert!(app.popup.is_none());
}

// Tests for multi-selection feature (RQ-0050)

#[test]
fn test_spacebar_toggles_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));

    let sid = "sid_1";

    // Select job with spacebar
    let action = app.handle_input(key(' '));
    assert!(action.is_none(), "Spacebar should not return action");
    assert!(
        app.selected_jobs.contains(sid),
        "Job should be selected after pressing spacebar"
    );

    // Toggle off with spacebar
    let action = app.handle_input(key(' '));
    assert!(action.is_none(), "Spacebar should not return action");
    assert!(
        !app.selected_jobs.contains(sid),
        "Job should be deselected after pressing spacebar again"
    );
}

#[test]
fn test_multiple_jobs_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));

    // Select first job
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    assert!(app.selected_jobs.contains("sid_0"));

    // Select third job
    app.jobs_state.select(Some(2));
    app.handle_input(key(' '));
    assert!(app.selected_jobs.contains("sid_2"));

    // Select fifth job
    app.jobs_state.select(Some(4));
    app.handle_input(key(' '));
    assert!(app.selected_jobs.contains("sid_4"));

    // Verify all three jobs are selected
    assert_eq!(
        app.selected_jobs.len(),
        3,
        "Should have exactly 3 jobs selected"
    );
    assert!(app.selected_jobs.contains("sid_0"));
    assert!(app.selected_jobs.contains("sid_2"));
    assert!(app.selected_jobs.contains("sid_4"));
}

#[test]
fn test_batch_cancel_popup_with_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));

    // Select multiple jobs
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(2));
    app.handle_input(key(' '));

    // Press 'c' to open batch cancel popup
    let action = app.handle_input(key('c'));
    assert!(action.is_none(), "Opening popup should not return action");
    assert!(app.popup.is_some(), "Popup should be open");

    // Verify it's a batch cancel popup with 2 jobs
    assert!(
        matches!(
            app.popup.as_ref().map(|p| &p.kind),
            Some(PopupType::ConfirmCancelBatch(sids)) if sids.len() == 2
        ),
        "Should be ConfirmCancelBatch with 2 SIDs"
    );
}

#[test]
fn test_batch_delete_popup_with_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(5))));

    // Select multiple jobs
    app.jobs_state.select(Some(1));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(3));
    app.handle_input(key(' '));

    // Press 'd' to open batch delete popup
    let action = app.handle_input(key('d'));
    assert!(action.is_none(), "Opening popup should not return action");
    assert!(app.popup.is_some(), "Popup should be open");

    // Verify it's a batch delete popup with 2 jobs
    assert!(
        matches!(
            app.popup.as_ref().map(|p| &p.kind),
            Some(PopupType::ConfirmDeleteBatch(sids)) if sids.len() == 2
        ),
        "Should be ConfirmDeleteBatch with 2 SIDs"
    );
}

#[test]
fn test_batch_cancel_action_generated() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));

    // Select two jobs
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(1));
    app.handle_input(key(' '));

    // Open batch cancel popup
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Press 'y' to confirm
    let action = app.handle_input(key('y'));
    assert!(action.is_some(), "Confirming should return action");

    // Verify it's CancelJobsBatch with correct SIDs
    assert!(
        matches!(
            action,
            Some(Action::CancelJobsBatch(sids)) if sids.len() == 2
        ),
        "Should be CancelJobsBatch with 2 SIDs"
    );
    assert!(app.popup.is_none(), "Popup should be closed after confirm");
}

#[test]
fn test_batch_delete_action_generated() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));

    // Select two jobs
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(2));
    app.handle_input(key(' '));

    // Open batch delete popup
    app.handle_input(key('d'));
    assert!(app.popup.is_some());

    // Press 'y' to confirm
    let action = app.handle_input(key('y'));
    assert!(action.is_some(), "Confirming should return action");

    // Verify it's DeleteJobsBatch with correct SIDs
    assert!(
        matches!(
            action,
            Some(Action::DeleteJobsBatch(sids)) if sids.len() == 2
        ),
        "Should be DeleteJobsBatch with 2 SIDs"
    );
    assert!(app.popup.is_none(), "Popup should be closed after confirm");
}

#[test]
fn test_single_cancel_with_no_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(1));

    // No jobs selected, pressing 'c' should open single cancel popup
    let action = app.handle_input(key('c'));
    assert!(action.is_none(), "Opening popup should not return action");

    // Verify it's a single cancel popup (not batch)
    assert!(
        matches!(
            app.popup.as_ref().map(|p| &p.kind),
            Some(PopupType::ConfirmCancel(sid)) if sid == "sid_1"
        ),
        "Should be ConfirmCancel popup for single job"
    );
}

#[test]
fn test_single_delete_with_no_selection() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));
    app.jobs_state.select(Some(2));

    // No jobs selected, pressing 'd' should open single delete popup
    let action = app.handle_input(key('d'));
    assert!(action.is_none(), "Opening popup should not return action");

    // Verify it's a single delete popup (not batch)
    assert!(
        matches!(
            app.popup.as_ref().map(|p| &p.kind),
            Some(PopupType::ConfirmDelete(sid)) if sid == "sid_2"
        ),
        "Should be ConfirmDelete popup for single job"
    );
}

#[test]
fn test_selection_cleared_after_job_operation() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));

    // Select two jobs
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(1));
    app.handle_input(key(' '));

    assert_eq!(app.selected_jobs.len(), 2, "Should have 2 jobs selected");

    // Simulate job operation complete
    app.update(Action::JobOperationComplete(
        "Operation complete".to_string(),
    ));

    // Selection should be cleared
    assert!(
        app.selected_jobs.is_empty(),
        "Selection should be cleared after JobOperationComplete"
    );
    assert_eq!(
        app.search_status, "Operation complete",
        "Status message should be updated"
    );
    assert!(!app.loading, "Loading should be cleared");
}

#[test]
fn test_selection_persists_across_jobs_loaded() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    // Load initial jobs and select some
    let jobs1 = create_mock_jobs(5);
    app.update(Action::JobsLoaded(Ok(jobs1)));
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(2));
    app.handle_input(key(' '));

    let selected_sids = app.selected_jobs.clone();
    assert_eq!(selected_sids.len(), 2);

    // Simulate refresh with new job list (same SIDs)
    let jobs2 = create_mock_jobs(5);
    app.update(Action::JobsLoaded(Ok(jobs2)));

    // Selection should still be present (tracked by SID)
    assert_eq!(
        app.selected_jobs.len(),
        2,
        "Selection should persist across JobsLoaded"
    );
    assert_eq!(
        app.selected_jobs, selected_sids,
        "Same SIDs should still be selected"
    );
}

#[test]
fn test_batch_popup_cancel_with_n() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));

    // Select job
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));

    // Open batch cancel popup
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Press 'n' to cancel
    let action = app.handle_input(key('n'));
    assert!(action.is_none(), "Canceling popup should not return action");
    assert!(app.popup.is_none(), "Popup should be closed");

    // Selection should still be present
    assert_eq!(app.selected_jobs.len(), 1);
}

#[test]
fn test_batch_popup_cancel_with_esc() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));

    // Select jobs
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(1));
    app.handle_input(key(' '));

    // Open batch delete popup
    app.handle_input(key('d'));
    assert!(app.popup.is_some());

    // Press Esc to cancel
    let action = app.handle_input(esc_key());
    assert!(action.is_none(), "Canceling popup should not return action");
    assert!(app.popup.is_none(), "Popup should be closed");

    // Selection should still be present
    assert_eq!(app.selected_jobs.len(), 2);
}

#[test]
fn test_batch_confirm_with_enter() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;
    app.update(Action::JobsLoaded(Ok(create_mock_jobs(3))));

    // Select jobs
    app.jobs_state.select(Some(0));
    app.handle_input(key(' '));
    app.jobs_state.select(Some(2));
    app.handle_input(key(' '));

    // Open batch cancel popup
    app.handle_input(key('c'));
    assert!(app.popup.is_some());

    // Press Enter to confirm
    let action = app.handle_input(enter_key());
    assert!(
        action.is_some(),
        "Confirming with Enter should return action"
    );
    assert!(
        matches!(action, Some(Action::CancelJobsBatch(_))),
        "Should be CancelJobsBatch action"
    );
    assert!(app.popup.is_none(), "Popup should be closed after confirm");
}

// ============================================================================
// Performance Tests
// ============================================================================

#[test]
fn test_search_rendering_with_large_dataset() {
    use ratatui::{Terminal, backend::TestBackend};
    use std::time::Instant;

    let dataset_sizes = [10, 100, 1000, 10000];

    for size in dataset_sizes {
        let mut app = App::new(None, ConnectionContext::default());
        app.current_screen = CurrentScreen::Search;

        let results: Vec<serde_json::Value> = (0..size)
            .map(|i| {
                serde_json::json!({
                    "_time": format!("2024-01-15T10:30:{:02}.000Z", i % 60),
                    "level": "INFO",
                    "message": format!("Event number {}", i),
                })
            })
            .collect();

        app.set_search_results(results);
        app.search_scroll_offset = 0;

        let backend = TestBackend::new(80, 24);
        let mut terminal = Terminal::new(backend).expect("Failed to create terminal");

        let start = Instant::now();
        terminal.draw(|f| app.render(f)).expect("Failed to render");
        let duration = start.elapsed();

        let max_expected_ms = 10;
        assert!(
            duration.as_millis() < max_expected_ms,
            "Rendering {} results took {:?}, expected < {:?}ms",
            size,
            duration,
            max_expected_ms
        );
    }
}

// ============================================================================
// Pagination Tests
// ============================================================================
// Pagination Tests
// ============================================================================

/// Helper to create mock search result JSON values
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
    app.update(Action::MoreSearchResultsLoaded(Err(
        "Connection timeout".to_string()
    )));

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

// ============================================================================
// Progress Callback Bridge Tests (RQ-0128)
// ============================================================================

#[test]
fn test_progress_callback_bridge_sends_action() {
    use splunk_tui::action::progress_callback_to_action_sender;
    use tokio::sync::mpsc::unbounded_channel;

    let (tx, mut rx) = unbounded_channel::<Action>();
    let mut callback = progress_callback_to_action_sender(tx);

    // Call the callback with a progress value
    callback(0.5);

    // Verify the action was sent
    let action = rx.try_recv().expect("Should receive action");
    assert!(
        matches!(action, Action::Progress(0.5)),
        "Should receive Progress action with value 0.5"
    );
}

#[test]
fn test_progress_callback_bridge_clamps_to_valid_range() {
    use splunk_tui::action::progress_callback_to_action_sender;
    use tokio::sync::mpsc::unbounded_channel;

    let (tx, mut rx) = unbounded_channel::<Action>();
    let mut callback = progress_callback_to_action_sender(tx);

    // Test values outside [0.0, 1.0] range
    callback(-0.5);
    let action = rx.try_recv().expect("Should receive action");
    assert!(
        matches!(action, Action::Progress(0.0)),
        "Negative progress should be clamped to 0.0"
    );

    callback(1.5);
    let action = rx.try_recv().expect("Should receive action");
    assert!(
        matches!(action, Action::Progress(1.0)),
        "Progress > 1.0 should be clamped to 1.0"
    );
}

#[test]
fn test_progress_callback_bridge_preserves_valid_values() {
    use splunk_tui::action::progress_callback_to_action_sender;
    use tokio::sync::mpsc::unbounded_channel;

    let (tx, mut rx) = unbounded_channel::<Action>();
    let mut callback = progress_callback_to_action_sender(tx);

    // Test boundary values
    callback(0.0);
    let action = rx.try_recv().expect("Should receive action");
    assert!(
        matches!(action, Action::Progress(0.0)),
        "Progress 0.0 should be preserved"
    );

    callback(1.0);
    let action = rx.try_recv().expect("Should receive action");
    assert!(
        matches!(action, Action::Progress(1.0)),
        "Progress 1.0 should be preserved"
    );

    callback(0.75);
    let action = rx.try_recv().expect("Should receive action");
    assert!(
        matches!(action, Action::Progress(0.75)),
        "Progress 0.75 should be preserved"
    );
}

// ============================================================================
// Error Context Helper Tests (RQ-0128)
// ============================================================================

#[test]
fn test_search_error_message_timeout() {
    use splunk_tui::error_details::search_error_message;

    let error = splunk_client::ClientError::Timeout(std::time::Duration::from_secs(300));
    let message = search_error_message(&error);
    assert_eq!(
        message, "Search timeout",
        "Timeout should map to 'Search timeout'"
    );
}

#[test]
fn test_search_error_message_auth_failed() {
    use splunk_tui::error_details::search_error_message;

    let error = splunk_client::ClientError::AuthFailed("Invalid credentials".to_string());
    let message = search_error_message(&error);
    assert_eq!(
        message, "Authentication failed",
        "AuthFailed should map to 'Authentication failed'"
    );
}

#[test]
fn test_search_error_message_session_expired() {
    use splunk_tui::error_details::search_error_message;

    let error = splunk_client::ClientError::SessionExpired;
    let message = search_error_message(&error);
    assert_eq!(
        message, "Session expired",
        "SessionExpired should map to 'Session expired'"
    );
}

#[test]
fn test_search_error_message_rate_limited() {
    use splunk_tui::error_details::search_error_message;

    let error = splunk_client::ClientError::RateLimited(Some(std::time::Duration::from_secs(60)));
    let message = search_error_message(&error);
    assert_eq!(
        message, "Rate limited",
        "RateLimited should map to 'Rate limited'"
    );
}

#[test]
fn test_search_error_message_connection_refused() {
    use splunk_tui::error_details::search_error_message;

    let error = splunk_client::ClientError::ConnectionRefused("localhost:8089".to_string());
    let message = search_error_message(&error);
    assert_eq!(
        message, "Connection refused",
        "ConnectionRefused should map to 'Connection refused'"
    );
}

#[test]
fn test_build_search_error_details_includes_all_context() {
    use splunk_tui::error_details::build_search_error_details;

    let error = splunk_client::ClientError::Timeout(std::time::Duration::from_secs(300));
    let details = build_search_error_details(
        &error,
        "index=_internal | head 10".to_string(),
        "search_with_progress".to_string(),
        Some("test_sid_123".to_string()),
    );

    assert_eq!(
        details.context.get("query"),
        Some(&"index=_internal | head 10".to_string()),
        "Should include query in context"
    );
    assert_eq!(
        details.context.get("operation"),
        Some(&"search_with_progress".to_string()),
        "Should include operation in context"
    );
    assert_eq!(
        details.context.get("sid"),
        Some(&"test_sid_123".to_string()),
        "Should include SID in context"
    );
}

#[test]
fn test_build_search_error_details_with_rate_limited() {
    use splunk_tui::error_details::build_search_error_details;

    // RateLimited takes Option<Duration>
    let error = splunk_client::ClientError::RateLimited(Some(std::time::Duration::from_secs(60)));
    let details = build_search_error_details(
        &error,
        "search *".to_string(),
        "search_with_progress".to_string(),
        Some("test_sid_456".to_string()),
    );

    assert_eq!(
        details.context.get("query"),
        Some(&"search *".to_string()),
        "Should include query in context"
    );
    assert_eq!(
        details.context.get("operation"),
        Some(&"search_with_progress".to_string()),
        "Should include operation in context"
    );
}

#[test]
fn test_build_search_error_details_without_sid() {
    use splunk_tui::error_details::build_search_error_details;

    let error = splunk_client::ClientError::AuthFailed("Invalid token".to_string());
    let details = build_search_error_details(
        &error,
        "search *".to_string(),
        "create_search_job".to_string(),
        None,
    );

    assert_eq!(
        details.context.get("query"),
        Some(&"search *".to_string()),
        "Should include query in context"
    );
    assert_eq!(
        details.context.get("operation"),
        Some(&"create_search_job".to_string()),
        "Should include operation in context"
    );
    assert!(
        !details.context.contains_key("sid"),
        "Should not include SID when None"
    );
}

// ============================================================================
// App Enable/Disable Tests (RQ-0135)
// ============================================================================

#[test]
fn test_apps_enable_opens_confirmation_popup() {
    let mut app = App::new(None, ConnectionContext::default());

    // Set up apps list with a disabled app
    app.apps = Some(vec![SplunkApp {
        name: "test-app".to_string(),
        label: Some("Test App".to_string()),
        version: Some("1.0.0".to_string()),
        disabled: true,
        description: None,
        author: None,
        is_configured: Some(true),
        is_visible: Some(true),
    }]);
    app.apps_state.select(Some(0));
    app.current_screen = CurrentScreen::Apps;

    // Press 'e' to enable
    let action = app.handle_input(key('e'));

    assert!(action.is_none());
    assert!(app.popup.is_some());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::ConfirmEnableApp(name)) if name == "test-app"
    ));
}

#[test]
fn test_apps_disable_opens_confirmation_popup() {
    let mut app = App::new(None, ConnectionContext::default());

    // Set up apps list with an enabled app
    app.apps = Some(vec![SplunkApp {
        name: "test-app".to_string(),
        label: Some("Test App".to_string()),
        version: Some("1.0.0".to_string()),
        disabled: false,
        description: None,
        author: None,
        is_configured: Some(true),
        is_visible: Some(true),
    }]);
    app.apps_state.select(Some(0));
    app.current_screen = CurrentScreen::Apps;

    // Press 'd' to disable
    let action = app.handle_input(key('d'));

    assert!(action.is_none());
    assert!(app.popup.is_some());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(PopupType::ConfirmDisableApp(name)) if name == "test-app"
    ));
}

#[test]
fn test_apps_enable_already_enabled_shows_info() {
    let mut app = App::new(None, ConnectionContext::default());

    // Set up apps list with an already enabled app
    app.apps = Some(vec![SplunkApp {
        name: "test-app".to_string(),
        label: Some("Test App".to_string()),
        version: Some("1.0.0".to_string()),
        disabled: false,
        description: None,
        author: None,
        is_configured: Some(true),
        is_visible: Some(true),
    }]);
    app.apps_state.select(Some(0));
    app.current_screen = CurrentScreen::Apps;

    // Press 'e' to enable (but it's already enabled)
    let action = app.handle_input(key('e'));

    assert!(matches!(
        action,
        Some(Action::Notify(ToastLevel::Info, msg)) if msg.contains("already enabled")
    ));
    assert!(app.popup.is_none());
}

#[test]
fn test_apps_disable_already_disabled_shows_info() {
    let mut app = App::new(None, ConnectionContext::default());

    // Set up apps list with an already disabled app
    app.apps = Some(vec![SplunkApp {
        name: "test-app".to_string(),
        label: Some("Test App".to_string()),
        version: Some("1.0.0".to_string()),
        disabled: true,
        description: None,
        author: None,
        is_configured: Some(true),
        is_visible: Some(true),
    }]);
    app.apps_state.select(Some(0));
    app.current_screen = CurrentScreen::Apps;

    // Press 'd' to disable (but it's already disabled)
    let action = app.handle_input(key('d'));

    assert!(matches!(
        action,
        Some(Action::Notify(ToastLevel::Info, msg)) if msg.contains("already disabled")
    ));
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_confirm_enable_app() {
    let mut app = App::new(None, ConnectionContext::default());
    app.popup = Some(Popup::builder(PopupType::ConfirmEnableApp("test-app".to_string())).build());

    // Confirm with 'y'
    let action = app.handle_popup_input(key('y'));
    assert!(matches!(action, Some(Action::EnableApp(name)) if name == "test-app"));
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_confirm_disable_app() {
    let mut app = App::new(None, ConnectionContext::default());
    app.popup = Some(Popup::builder(PopupType::ConfirmDisableApp("test-app".to_string())).build());

    // Confirm with Enter
    let action = app.handle_popup_input(enter_key());
    assert!(matches!(action, Some(Action::DisableApp(name)) if name == "test-app"));
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_cancel_enable_app() {
    let mut app = App::new(None, ConnectionContext::default());
    app.popup = Some(Popup::builder(PopupType::ConfirmEnableApp("test-app".to_string())).build());

    // Cancel with 'n'
    let action = app.handle_popup_input(key('n'));
    assert!(action.is_none());
    assert!(app.popup.is_none());
}

#[test]
fn test_popup_cancel_disable_app() {
    let mut app = App::new(None, ConnectionContext::default());
    app.popup = Some(Popup::builder(PopupType::ConfirmDisableApp("test-app".to_string())).build());

    // Cancel with Esc
    let action = app.handle_popup_input(esc_key());
    assert!(action.is_none());
    assert!(app.popup.is_none());
}

// ============================================================================
// Search Input Mode Tests (RQ-0101 fix)
// ============================================================================

#[test]
fn test_search_input_mode_default_is_query_focused() {
    let app = App::new(None, ConnectionContext::default());
    assert_eq!(app.current_screen, CurrentScreen::Search);
    assert!(
        matches!(
            app.search_input_mode,
            splunk_tui::SearchInputMode::QueryFocused
        ),
        "Default search input mode should be QueryFocused"
    );
}

#[test]
fn test_search_input_mode_toggles_with_tab() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Initial state: QueryFocused
    assert!(matches!(
        app.search_input_mode,
        splunk_tui::SearchInputMode::QueryFocused
    ));

    // Tab toggles to ResultsFocused (bypasses global NextScreen binding in QueryFocused mode)
    app.handle_input(tab_key());
    assert!(
        matches!(
            app.search_input_mode,
            splunk_tui::SearchInputMode::ResultsFocused
        ),
        "Tab should toggle to ResultsFocused mode"
    );

    // In ResultsFocused mode, Tab triggers NextScreen action (does not toggle back)
    let action = app.handle_input(tab_key());
    assert!(
        matches!(action, Some(Action::NextScreen)),
        "Tab in ResultsFocused mode should return NextScreen action"
    );
    // Mode stays as ResultsFocused
    assert!(
        matches!(
            app.search_input_mode,
            splunk_tui::SearchInputMode::ResultsFocused
        ),
        "Mode should remain ResultsFocused after Tab in that mode"
    );
}

#[test]
fn test_search_input_mode_esc_switches_to_query_focused() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Start in ResultsFocused mode
    app.search_input_mode = splunk_tui::SearchInputMode::ResultsFocused;

    // Esc switches back to QueryFocused
    app.handle_input(esc_key());
    assert!(
        matches!(
            app.search_input_mode,
            splunk_tui::SearchInputMode::QueryFocused
        ),
        "Esc should switch back to QueryFocused mode"
    );
}

#[test]
fn test_search_query_focused_inserts_q_char() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;

    // Type 'q' - should insert into query, NOT quit
    let action = app.handle_input(key('q'));
    assert!(
        action.is_none(),
        "'q' in QueryFocused mode should not return an action"
    );
    assert_eq!(
        app.search_input, "q",
        "'q' should be inserted into search input"
    );
}

#[test]
fn test_search_query_focused_inserts_question_mark() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;

    // Type '?' - should insert into query, NOT open help
    let action = app.handle_input(key('?'));
    assert!(
        action.is_none(),
        "'?' in QueryFocused mode should not return an action"
    );
    assert_eq!(
        app.search_input, "?",
        "'?' should be inserted into search input"
    );
}

#[test]
fn test_search_query_focused_inserts_digits() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;

    // Type digits - should insert into query
    app.handle_input(key('1'));
    app.handle_input(key('2'));
    app.handle_input(key('3'));
    assert_eq!(
        app.search_input, "123",
        "Digits should be inserted into search input"
    );
}

#[test]
fn test_search_query_focused_inserts_e_char() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;

    // Type 'e' - should insert into query
    let action = app.handle_input(key('e'));
    assert!(
        action.is_none(),
        "'e' in QueryFocused mode should not return an action"
    );
    assert_eq!(
        app.search_input, "e",
        "'e' should be inserted into search input"
    );
}

#[test]
fn test_search_results_focused_allows_quit() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::ResultsFocused;

    // 'q' in ResultsFocused mode should return Quit action
    let action = app.handle_input(key('q'));
    assert!(
        matches!(action, Some(Action::Quit)),
        "'q' in ResultsFocused mode should return Quit action"
    );
}

#[test]
fn test_search_results_focused_allows_help() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::ResultsFocused;

    // '?' in ResultsFocused mode should return OpenHelpPopup action
    let action = app.handle_input(key('?'));
    assert!(
        matches!(action, Some(Action::OpenHelpPopup)),
        "'?' in ResultsFocused mode should return OpenHelpPopup action"
    );
}

#[test]
fn test_search_query_focused_allows_ctrl_shortcuts() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "index=main".to_string();

    // Ctrl+ shortcuts should still work in QueryFocused mode
    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(_))),
        "Ctrl+c in QueryFocused mode should return CopyToClipboard action"
    );
}

#[test]
fn test_search_query_focused_allows_special_keys() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;

    // Enter should still run search
    app.search_input = "index=main".to_string();
    let action = app.handle_input(enter_key());
    assert!(
        matches!(action, Some(Action::RunSearch { .. })),
        "Enter in QueryFocused mode should return RunSearch action"
    );
}

#[test]
fn test_search_query_focused_allows_backspace() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;

    // Type some text first
    app.handle_input(key('t'));
    app.handle_input(key('e'));
    app.handle_input(key('s'));
    app.handle_input(key('t'));
    assert_eq!(app.search_input, "test");

    // Backspace should remove last character
    let action = app.handle_input(backspace_key());
    assert!(action.is_none(), "Backspace should not return an action");
    assert_eq!(
        app.search_input, "tes",
        "Backspace should remove last character"
    );
}

#[test]
fn test_search_run_switches_to_results_focused() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "index=main".to_string();

    // Running search should switch to ResultsFocused mode
    let action = app.handle_input(enter_key());
    assert!(matches!(action, Some(Action::RunSearch { .. })));

    // Apply the action (which would normally be done in the main loop)
    app.update(action.unwrap());

    assert!(
        matches!(
            app.search_input_mode,
            splunk_tui::SearchInputMode::ResultsFocused
        ),
        "Running search should switch to ResultsFocused mode"
    );
}

// ============================================================================
// Cursor Movement Tests (RQ-0110)
// ============================================================================

#[test]
fn test_cursor_initial_position_at_end_of_input() {
    let app = App::new(None, ConnectionContext::default());
    // Cursor should start at 0 for empty input
    assert_eq!(app.search_cursor_position, 0);
}

#[test]
fn test_cursor_position_with_persisted_query() {
    let persisted = splunk_config::PersistedState {
        last_search_query: Some("index=main".to_string()),
        ..Default::default()
    };
    let app = App::new(Some(persisted), ConnectionContext::default());
    // Cursor should be at end of persisted query
    assert_eq!(app.search_cursor_position, 10); // "index=main".len()
    assert_eq!(app.search_input, "index=main");
}

#[test]
fn test_cursor_left_at_start() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "test".to_string();
    app.search_cursor_position = 0;

    // Pressing Left at start should stay at 0
    let action = app.handle_input(left_key());
    assert!(action.is_none(), "Left arrow should not return an action");
    assert_eq!(app.search_cursor_position, 0, "Cursor should stay at 0");
}

#[test]
fn test_cursor_left_moves_back() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "test".to_string();
    app.search_cursor_position = 4;

    // Pressing Left should move cursor back
    app.handle_input(left_key());
    assert_eq!(app.search_cursor_position, 3);

    app.handle_input(left_key());
    assert_eq!(app.search_cursor_position, 2);
}

#[test]
fn test_cursor_right_at_end() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "test".to_string();
    app.search_cursor_position = 4;

    // Pressing Right at end should stay at end
    let action = app.handle_input(right_key());
    assert!(action.is_none(), "Right arrow should not return an action");
    assert_eq!(app.search_cursor_position, 4, "Cursor should stay at end");
}

#[test]
fn test_cursor_right_moves_forward() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "test".to_string();
    app.search_cursor_position = 0;

    // Pressing Right should move cursor forward
    app.handle_input(right_key());
    assert_eq!(app.search_cursor_position, 1);

    app.handle_input(right_key());
    assert_eq!(app.search_cursor_position, 2);
}

#[test]
fn test_home_key_moves_to_start() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "test".to_string();
    app.search_cursor_position = 4;

    let action = app.handle_input(home_key());
    assert!(action.is_none(), "Home key should not return an action");
    assert_eq!(
        app.search_cursor_position, 0,
        "Home should move cursor to start"
    );
}

#[test]
fn test_end_key_moves_to_end() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "test".to_string();
    app.search_cursor_position = 0;

    let action = app.handle_input(end_key());
    assert!(action.is_none(), "End key should not return an action");
    assert_eq!(
        app.search_cursor_position, 4,
        "End should move cursor to end"
    );
}

#[test]
fn test_delete_removes_at_cursor() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "hello".to_string();
    app.search_cursor_position = 2; // At 'l' (he|llo)

    let action = app.handle_input(delete_key());
    assert!(action.is_none(), "Delete should not return an action");
    assert_eq!(
        app.search_input, "helo",
        "Delete should remove character at cursor"
    );
    assert_eq!(
        app.search_cursor_position, 2,
        "Cursor should stay at same position"
    );
}

#[test]
fn test_delete_at_end_does_nothing() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "hello".to_string();
    app.search_cursor_position = 5; // At end

    let action = app.handle_input(delete_key());
    assert!(
        action.is_none(),
        "Delete at end should not return an action"
    );
    assert_eq!(
        app.search_input, "hello",
        "Delete at end should not change input"
    );
    assert_eq!(app.search_cursor_position, 5, "Cursor should stay at end");
}

#[test]
fn test_backspace_removes_before_cursor() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "hello".to_string();
    app.search_cursor_position = 2; // After 'he' (he|llo)

    let action = app.handle_input(backspace_key());
    assert!(action.is_none(), "Backspace should not return an action");
    assert_eq!(
        app.search_input, "hllo",
        "Backspace should remove character before cursor"
    );
    assert_eq!(app.search_cursor_position, 1, "Cursor should move back");
}

#[test]
fn test_backspace_at_start_does_nothing() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "hello".to_string();
    app.search_cursor_position = 0;

    let action = app.handle_input(backspace_key());
    assert!(
        action.is_none(),
        "Backspace at start should not return an action"
    );
    assert_eq!(
        app.search_input, "hello",
        "Backspace at start should not change input"
    );
    assert_eq!(app.search_cursor_position, 0, "Cursor should stay at start");
}

#[test]
fn test_char_insertion_at_cursor() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "helo".to_string();
    app.search_cursor_position = 2; // After 'he'

    let action = app.handle_input(key('l'));
    assert!(action.is_none(), "Char input should not return an action");
    // Inserting 'l' at position 2 in "helo"
    // h e l o
    // 0 1 2 3
    // Inserting at 2: h e l l o
    assert_eq!(
        app.search_input, "hello",
        "Char should be inserted at cursor position"
    );
    assert_eq!(app.search_cursor_position, 3, "Cursor should move forward");
}

#[test]
fn test_char_insertion_at_end() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input = "hell".to_string();
    app.search_cursor_position = 4;

    app.handle_input(key('o'));
    assert_eq!(app.search_input, "hello", "Char should be appended at end");
    assert_eq!(app.search_cursor_position, 5, "Cursor should move to end");
}

#[test]
fn test_history_navigation_sets_cursor_to_end() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;

    // Add some history
    app.search_history = vec!["index=_internal".to_string(), "index=main".to_string()];

    // Move cursor to middle of current (empty) input
    app.search_input = "test".to_string();
    app.search_cursor_position = 2;

    // Press Up to go to history (index 0 is the most recent = "index=_internal")
    app.handle_input(up_key());
    assert_eq!(app.search_input, "index=_internal");
    assert_eq!(
        app.search_cursor_position, 15,
        "Cursor should be at end after history nav"
    );

    // Press Up again to go to older history (index 1 = "index=main")
    app.handle_input(up_key());
    assert_eq!(app.search_input, "index=main");
    assert_eq!(
        app.search_cursor_position, 10,
        "Cursor should be at end after history nav"
    );

    // Press Down to go back (to index 0 = "index=_internal")
    app.handle_input(down_key());
    assert_eq!(app.search_input, "index=_internal");
    assert_eq!(
        app.search_cursor_position, 15,
        "Cursor should be at end after history nav"
    );
}

#[test]
fn test_cursor_movement_only_in_query_focused_mode() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::ResultsFocused;
    app.search_input = "test".to_string();
    app.search_cursor_position = 4;

    // In ResultsFocused mode, Left/Right should not move cursor
    // (they would be handled by global bindings for navigation)
    // We just verify cursor position doesn't change
    let initial_pos = app.search_cursor_position;

    // Note: Left/Right in ResultsFocused mode return None from handle_search_input
    // but the action is handled by global bindings. We just verify the cursor
    // state isn't modified.
    app.handle_input(left_key());
    assert_eq!(
        app.search_cursor_position, initial_pos,
        "Cursor should not change in ResultsFocused mode"
    );

    app.handle_input(right_key());
    assert_eq!(
        app.search_cursor_position, initial_pos,
        "Cursor should not change in ResultsFocused mode"
    );
}

#[test]
fn test_saved_search_selection_sets_cursor_to_end() {
    use splunk_client::models::SavedSearch;

    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::SavedSearches;
    app.saved_searches = Some(vec![SavedSearch {
        name: "Test Search".to_string(),
        search: "index=_internal | stats count".to_string(),
        description: None,
        disabled: false,
    }]);
    app.saved_searches_state.select(Some(0));

    // Press Enter to select saved search
    let action = app.handle_input(enter_key());
    assert!(matches!(action, Some(Action::RunSearch { .. })));

    // Verify cursor is at end of selected query
    assert_eq!(app.search_input, "index=_internal | stats count");
    assert_eq!(
        app.search_cursor_position, 29,
        "Cursor should be at end of saved search query"
    );
}

// ============================================================================
// Search Status Messaging Tests (RQ-0111)
// ============================================================================

#[test]
fn test_search_complete_uses_running_query_not_current_input() {
    let mut app = App::new(None, ConnectionContext::default());

    // User types and submits a query
    app.search_input = "index=main | stats count".to_string();
    app.update(Action::SearchStarted(
        "index=main | stats count".to_string(),
    ));

    // While search is running, user modifies the input
    app.search_input = "different query".to_string();

    // Search completes
    app.update(Action::SearchComplete(Ok((
        vec![],
        "sid123".to_string(),
        Some(0),
    ))));

    // Status should reference the submitted query, not the current input
    assert!(app.search_status.contains("index=main | stats count"));
    assert!(!app.search_status.contains("different query"));
}

#[test]
fn test_running_query_cleared_on_completion() {
    let mut app = App::new(None, ConnectionContext::default());

    app.update(Action::SearchStarted("index=main".to_string()));
    assert_eq!(app.running_query, Some("index=main".to_string()));

    app.update(Action::SearchComplete(Ok((
        vec![],
        "sid123".to_string(),
        Some(0),
    ))));

    // running_query should be cleared after completion
    assert_eq!(app.running_query, None);
}

#[test]
fn test_running_query_cleared_on_error() {
    let mut app = App::new(None, ConnectionContext::default());

    app.update(Action::SearchStarted("index=main".to_string()));
    assert_eq!(app.running_query, Some("index=main".to_string()));

    app.update(Action::SearchComplete(Err((
        "Search failed".to_string(),
        helpers::error_details_from_string("test error"),
    ))));

    // running_query should be cleared on error
    assert_eq!(app.running_query, None);
}

#[test]
fn test_search_complete_fallback_to_search_input() {
    let mut app = App::new(None, ConnectionContext::default());

    // No SearchStarted action was sent (e.g., old code path or edge case)
    app.search_input = "fallback query".to_string();

    app.update(Action::SearchComplete(Ok((
        vec![],
        "sid123".to_string(),
        Some(0),
    ))));

    // Should fall back to search_input
    assert!(app.search_status.contains("fallback query"));
}
