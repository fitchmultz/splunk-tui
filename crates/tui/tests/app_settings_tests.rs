//! Tests for Settings screen functionality.
//!
//! This module tests:
//! - Auto-refresh toggle
//! - Theme cycling
//! - Sort column cycling
//! - Search history clearing
//!
//! ## Invariants
//! - Settings changes must immediately update the app state
//! - Persisted state must reflect theme changes
//!
//! ## Test Organization
//! Tests are grouped by setting type.

mod helpers;
use helpers::*;
use splunk_tui::{CurrentScreen, action::Action, app::App, app::ConnectionContext};

#[allow(dead_code)]
#[test]
fn test_settings_screen_navigation() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Settings;

    // Test navigation with Tab - should go to Overview
    let action = app.handle_input(tab_key());
    assert!(
        matches!(action, Some(Action::NextScreen)),
        "Tab from Settings should return NextScreen and go to Overview"
    );

    app.update(action.unwrap());
    // Verify screen switched to Overview
    assert_eq!(app.current_screen, CurrentScreen::Overview);

    // Tab from Overview should wrap to Search
    let action = app.handle_input(tab_key());
    app.update(action.unwrap());
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
