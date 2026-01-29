//! Tests for quit action handling.
//!
//! This module tests:
//! - Keyboard quit ('q' key)
//! - Mouse quit (footer click)
//! - Quit behavior on different screens
//! - Quit behavior in Search screen input modes
//!
//! ## Invariants
//! - 'q' on non-Search screens must always quit
//! - 'q' on Search screen in QueryFocused mode must insert 'q' character
//! - 'q' on Search screen in ResultsFocused mode must quit
//!
//! ## Test Organization
//! Tests are grouped by quit method: keyboard, mouse.

mod helpers;
use helpers::*;
use ratatui::prelude::Rect;
use splunk_tui::{CurrentScreen, action::Action, app::App, app::ConnectionContext};

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
