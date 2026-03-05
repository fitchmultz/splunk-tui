//! Purpose: Deterministic interaction-to-render visual behavior tests.
//! Responsibilities: Drive key input sequences and verify state transitions plus semantic visual styling.
//! Scope: Keyboard-driven local state transitions (no async side effects or network).
//! Usage: Run via `cargo test -p splunk-tui --test interaction_render_tests` or `make tui-visual`.
//! Invariants/Assumptions: Tests start from stable initial app state and fixed terminal size.

mod helpers;

use helpers::{TuiHarness, assert_text_has_fg, assert_text_has_modifier, esc_key, key, tab_key};
use ratatui::style::Modifier;
use splunk_tui::{CurrentScreen, PopupType, SearchInputMode};

#[test]
fn interaction_tab_cycles_screen_and_preserves_header_title_style() {
    let mut harness = TuiHarness::new(120, 24);
    harness.app.current_screen = CurrentScreen::Search;

    let initial_buffer = harness.render_buffer();
    assert_text_has_fg(&initial_buffer, "Splunk TUI", harness.app.theme.title);
    assert_text_has_modifier(&initial_buffer, "Splunk TUI", Modifier::BOLD);

    harness.step_key(tab_key());
    assert_eq!(harness.app.current_screen, CurrentScreen::Indexes);

    let indexed_buffer = harness.render_buffer();
    assert_text_has_fg(&indexed_buffer, "Splunk TUI", harness.app.theme.title);
    assert_text_has_modifier(&indexed_buffer, "Splunk TUI", Modifier::BOLD);
}

#[test]
fn interaction_help_popup_open_close_roundtrip_keeps_footer_semantics() {
    let mut harness = TuiHarness::new(120, 24);
    harness.app.current_screen = CurrentScreen::Search;
    harness.app.search_input_mode = SearchInputMode::ResultsFocused;

    let _ = harness.render_buffer();

    harness.step_key(key('?'));
    assert!(
        matches!(
            harness.app.popup.as_ref().map(|popup| &popup.kind),
            Some(PopupType::Help)
        ),
        "Expected Help popup after '?'"
    );

    let popup_buffer = harness.render_buffer();
    assert_text_has_fg(&popup_buffer, "q:Quit", harness.app.theme.error);

    harness.step_key(esc_key());
    assert!(
        harness.app.popup.is_none(),
        "Expected popup to close on Esc"
    );

    let post_close_buffer = harness.render_buffer();
    assert_text_has_fg(&post_close_buffer, "?:Help", harness.app.theme.success);
    assert_text_has_fg(&post_close_buffer, "q:Quit", harness.app.theme.error);
}
