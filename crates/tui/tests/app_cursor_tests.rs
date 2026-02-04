//! Tests for cursor movement in search input (RQ-0110).
//!
//! This module tests:
//! - Cursor initial position
//! - Left/right arrow key movement
//! - Home/End key movement
//! - Delete and Backspace behavior
//! - Character insertion at cursor position
//! - History navigation cursor positioning
//!
//! ## Invariants
//! - Cursor must stay within bounds [0, input_length]
//! - Character insertion must happen at cursor position
//! - History navigation must set cursor to end of history item
//!
//! ## Test Organization
//! Tests are grouped by: cursor movement, text editing, history interaction.

mod helpers;
use helpers::*;
use splunk_client::models::SavedSearch;
use splunk_tui::{CurrentScreen, action::Action, app::App, app::ConnectionContext};

#[test]
fn test_cursor_initial_position_at_end_of_input() {
    let app = App::new(None, ConnectionContext::default());
    // Cursor should start at 0 for empty input
    assert_eq!(app.search_input.cursor_position(), 0);
}

#[test]
fn test_cursor_position_with_persisted_query() {
    let persisted = splunk_config::PersistedState {
        last_search_query: Some("index=main".to_string()),
        ..Default::default()
    };
    let app = App::new(Some(persisted), ConnectionContext::default());
    // Cursor should be at end of persisted query
    assert_eq!(app.search_input.cursor_position(), 10); // "index=main".len()
    assert_eq!(app.search_input.value(), "index=main");
}

#[test]
fn test_cursor_left_at_start() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input.set_value("test");
    app.search_input.set_cursor_position(0);

    // Pressing Left at start should stay at 0
    let action = app.handle_input(left_key());
    assert!(action.is_none(), "Left arrow should not return an action");
    assert_eq!(
        app.search_input.cursor_position(),
        0,
        "Cursor should stay at 0"
    );
}

#[test]
fn test_cursor_left_moves_back() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input.set_value("test");
    app.search_input.set_cursor_position(4);

    // Pressing Left should move cursor back
    app.handle_input(left_key());
    assert_eq!(app.search_input.cursor_position(), 3);

    app.handle_input(left_key());
    assert_eq!(app.search_input.cursor_position(), 2);
}

#[test]
fn test_cursor_right_at_end() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input.set_value("test");
    app.search_input.set_cursor_position(4);

    // Pressing Right at end should stay at end
    let action = app.handle_input(right_key());
    assert!(action.is_none(), "Right arrow should not return an action");
    assert_eq!(
        app.search_input.cursor_position(),
        4,
        "Cursor should stay at end"
    );
}

#[test]
fn test_cursor_right_moves_forward() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input.set_value("test");
    app.search_input.set_cursor_position(0);

    // Pressing Right should move cursor forward
    app.handle_input(right_key());
    assert_eq!(app.search_input.cursor_position(), 1);

    app.handle_input(right_key());
    assert_eq!(app.search_input.cursor_position(), 2);
}

#[test]
fn test_home_key_moves_to_start() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input.set_value("test");
    app.search_input.set_cursor_position(4);

    let action = app.handle_input(home_key());
    assert!(action.is_none(), "Home key should not return an action");
    assert_eq!(
        app.search_input.cursor_position(),
        0,
        "Home should move cursor to start"
    );
}

#[test]
fn test_end_key_moves_to_end() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input.set_value("test");
    app.search_input.set_cursor_position(0);

    let action = app.handle_input(end_key());
    assert!(action.is_none(), "End key should not return an action");
    assert_eq!(
        app.search_input.cursor_position(),
        4,
        "End should move cursor to end"
    );
}

#[test]
fn test_delete_removes_at_cursor() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input.set_value("hello");
    app.search_input.set_cursor_position(2); // At 'l' (he|llo)

    let action = app.handle_input(delete_key());
    assert!(action.is_none(), "Delete should not return an action");
    assert_eq!(
        app.search_input.value(),
        "helo",
        "Delete should remove character at cursor"
    );
    assert_eq!(
        app.search_input.cursor_position(),
        2,
        "Cursor should stay at same position"
    );
}

#[test]
fn test_delete_at_end_does_nothing() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input.set_value("hello");
    app.search_input.set_cursor_position(5); // At end

    let action = app.handle_input(delete_key());
    assert!(
        action.is_none(),
        "Delete at end should not return an action"
    );
    assert_eq!(
        app.search_input.value(),
        "hello",
        "Delete at end should not change input"
    );
    assert_eq!(
        app.search_input.cursor_position(),
        5,
        "Cursor should stay at end"
    );
}

#[test]
fn test_backspace_removes_before_cursor() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input.set_value("hello");
    app.search_input.set_cursor_position(2); // After 'he' (he|llo)

    let action = app.handle_input(backspace_key());
    assert!(action.is_none(), "Backspace should not return an action");
    assert_eq!(
        app.search_input.value(),
        "hllo",
        "Backspace should remove character before cursor"
    );
    assert_eq!(
        app.search_input.cursor_position(),
        1,
        "Cursor should move back"
    );
}

#[test]
fn test_backspace_at_start_does_nothing() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input.set_value("hello");
    app.search_input.set_cursor_position(0);

    let action = app.handle_input(backspace_key());
    assert!(
        action.is_none(),
        "Backspace at start should not return an action"
    );
    assert_eq!(
        app.search_input.value(),
        "hello",
        "Backspace at start should not change input"
    );
    assert_eq!(
        app.search_input.cursor_position(),
        0,
        "Cursor should stay at start"
    );
}

#[test]
fn test_char_insertion_at_cursor() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input.set_value("helo");
    app.search_input.set_cursor_position(2); // After 'he'

    let action = app.handle_input(key('l'));
    assert!(action.is_none(), "Char input should not return an action");
    // Inserting 'l' at position 2 in "helo"
    // h e l o
    // 0 1 2 3
    // Inserting at 2: h e l l o
    assert_eq!(
        app.search_input.value(),
        "hello",
        "Char should be inserted at cursor position"
    );
    assert_eq!(
        app.search_input.cursor_position(),
        3,
        "Cursor should move forward"
    );
}

#[test]
fn test_char_insertion_at_end() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;
    app.search_input.set_value("hell");
    app.search_input.set_cursor_position(4);

    app.handle_input(key('o'));
    assert_eq!(
        app.search_input.value(),
        "hello",
        "Char should be appended at end"
    );
    assert_eq!(
        app.search_input.cursor_position(),
        5,
        "Cursor should move to end"
    );
}

#[test]
fn test_history_navigation_sets_cursor_to_end() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::QueryFocused;

    // Add some history
    app.search_history = vec!["index=_internal".to_string(), "index=main".to_string()];

    // Move cursor to middle of current (empty) input
    app.search_input.set_value("test");
    app.search_input.set_cursor_position(2);

    // Press Up to go to history (index 0 is the most recent = "index=_internal")
    app.handle_input(up_key());
    assert_eq!(app.search_input.value(), "index=_internal");
    assert_eq!(
        app.search_input.cursor_position(),
        15,
        "Cursor should be at end after history nav"
    );

    // Press Up again to go to older history (index 1 = "index=main")
    app.handle_input(up_key());
    assert_eq!(app.search_input.value(), "index=main");
    assert_eq!(
        app.search_input.cursor_position(),
        10,
        "Cursor should be at end after history nav"
    );

    // Press Down to go back (to index 0 = "index=_internal")
    app.handle_input(down_key());
    assert_eq!(app.search_input.value(), "index=_internal");
    assert_eq!(
        app.search_input.cursor_position(),
        15,
        "Cursor should be at end after history nav"
    );
}

#[test]
fn test_cursor_movement_only_in_query_focused_mode() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input_mode = splunk_tui::SearchInputMode::ResultsFocused;
    app.search_input.set_value("test");
    app.search_input.set_cursor_position(4);

    // In ResultsFocused mode, Left/Right should not move cursor
    // (they would be handled by global bindings for navigation)
    // We just verify cursor position doesn't change
    let initial_pos = app.search_input.cursor_position();

    // Note: Left/Right in ResultsFocused mode return None from handle_search_input
    // but the action is handled by global bindings. We just verify the cursor
    // state isn't modified.
    app.handle_input(left_key());
    assert_eq!(
        app.search_input.cursor_position(),
        initial_pos,
        "Cursor should not change in ResultsFocused mode"
    );

    app.handle_input(right_key());
    assert_eq!(
        app.search_input.cursor_position(),
        initial_pos,
        "Cursor should not change in ResultsFocused mode"
    );
}

#[test]
fn test_saved_search_selection_sets_cursor_to_end() {
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
    assert_eq!(app.search_input.value(), "index=_internal | stats count");
    assert_eq!(
        app.search_input.cursor_position(),
        29,
        "Cursor should be at end of saved search query"
    );
}
