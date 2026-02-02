//! Tests for search input handling.
//!
//! This module tests:
//! - Character input in search query field
//! - Digit handling (should go to input, not trigger navigation)
//!
//! ## Invariants
//! - Digits typed in search screen must go to search input, not trigger navigation

mod helpers;
use helpers::*;
use splunk_tui::{CurrentScreen, app::App, app::ConnectionContext};

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
