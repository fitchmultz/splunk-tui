//! Purpose: Style-aware visual regression tests for core TUI surfaces.
//! Responsibilities: Capture styled buffer snapshots and assert semantic color/modifier contracts.
//! Scope: Deterministic offscreen rendering only (no network or async side effects).
//! Usage: Run via `cargo test -p splunk-tui --test snapshot_styled_tests` or `make tui-visual`.
//! Invariants/Assumptions: Terminal size and app state are fixed for stable snapshots.

mod helpers;

use helpers::{TuiHarness, assert_text_has_fg, assert_text_has_modifier, create_mock_jobs, key};
use ratatui::style::Modifier;
use splunk_tui::{CurrentScreen, SearchInputMode};

#[test]
fn snapshot_styled_search_screen_semantics() {
    let mut harness = TuiHarness::new(160, 24);
    harness.app.current_screen = CurrentScreen::Search;
    harness.app.search_input.set_value("index=main | head 5");
    harness.app.search_status = "Ready".to_string();
    harness.app.set_onboarding_checklist_enabled(false);

    let buffer = harness.render_buffer();
    assert_text_has_fg(&buffer, "Splunk TUI", harness.app.theme.title);
    assert_text_has_modifier(&buffer, "Splunk TUI", Modifier::BOLD);
    assert_text_has_fg(&buffer, "Ctrl+Q:Quit", harness.app.theme.error);

    insta::assert_snapshot!(harness.render_styled());
}

#[test]
fn snapshot_styled_jobs_screen_selection_highlight() {
    let mut harness = TuiHarness::new(100, 24);
    harness.app.current_screen = CurrentScreen::Jobs;
    harness.app.jobs = Some(create_mock_jobs());
    harness.app.jobs_state.select(Some(0));

    let buffer = harness.render_buffer();
    assert_text_has_fg(&buffer, "?:Help", harness.app.theme.success);
    assert_text_has_fg(&buffer, "q:Quit", harness.app.theme.error);

    insta::assert_snapshot!(harness.render_styled());
}

#[test]
fn snapshot_styled_help_popup_semantics() {
    let mut harness = TuiHarness::new(120, 24);
    harness.app.current_screen = CurrentScreen::Search;
    harness.app.search_input_mode = SearchInputMode::ResultsFocused;
    let _ = harness.render_buffer();

    harness.step_key(key('?'));
    assert!(
        harness.app.popup.is_some(),
        "Expected help popup to be open"
    );

    insta::assert_snapshot!(harness.render_styled());
}
