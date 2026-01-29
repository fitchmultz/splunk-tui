//! Tests for search input modes and character handling.
//!
//! This module tests:
//! - Search input mode toggling (QueryFocused vs ResultsFocused)
//! - Character insertion in QueryFocused mode
//! - Special key handling (Ctrl shortcuts, Enter, Backspace)
//! - Mode switching behavior
//!
//! ## Invariants
//! - Default mode must be QueryFocused
//! - Tab in QueryFocused mode must toggle to ResultsFocused
//! - Tab in ResultsFocused mode must navigate to next screen
//! - Characters typed in QueryFocused mode must insert into search input
//!
//! ## Test Organization
//! Tests are grouped by: mode switching, character insertion, special keys.

mod helpers;
use helpers::*;
use splunk_tui::{CurrentScreen, action::Action, app::App, app::ConnectionContext};

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
