//! Snapshot tests for popup rendering.

mod helpers;

use helpers::{TuiHarness, create_mock_index, create_mock_jobs};
use splunk_tui::app::state::{CurrentScreen, SearchInputMode};
use splunk_tui::{Popup, PopupType};

#[test]
fn snapshot_help_popup() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.popup = Some(Popup::builder(PopupType::Help).build());

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_help_popup_small_terminal() {
    // Test with narrow terminal (40x20) to verify scroll behavior
    let mut harness = TuiHarness::new(40, 20);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.popup = Some(Popup::builder(PopupType::Help).build());

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_confirm_cancel_popup() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.jobs = Some(create_mock_jobs());
    // Manually populate filtered_job_indices since tests don't trigger event handlers
    harness.app.filtered_job_indices = vec![0, 1];
    harness.app.jobs_state.select(Some(0));
    harness.app.popup = Some(
        Popup::builder(PopupType::ConfirmCancel(
            "scheduler_admin_search_1234567890".to_string(),
        ))
        .build(),
    );

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_confirm_delete_popup() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.jobs = Some(create_mock_jobs());
    // Manually populate filtered_job_indices since tests don't trigger event handlers
    harness.app.filtered_job_indices = vec![0, 1];
    harness.app.jobs_state.select(Some(1));
    harness.app.popup = Some(
        Popup::builder(PopupType::ConfirmDelete(
            "admin_search_9876543210".to_string(),
        ))
        .build(),
    );

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_index_details_popup_empty() {
    // Test with no index selected (edge case)
    let mut harness = TuiHarness::new(120, 30);
    harness.app.current_screen = splunk_tui::CurrentScreen::Indexes;
    harness.app.popup = Some(Popup::builder(PopupType::IndexDetails).build());
    // No indexes set, should show "No index selected" message

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_index_details_popup_populated() {
    // Test with full index data
    let mut harness = TuiHarness::new(120, 30);
    harness.app.current_screen = splunk_tui::CurrentScreen::Indexes;
    harness.app.indexes = Some(vec![create_mock_index()]);
    harness.app.indexes_state.select(Some(0));
    harness.app.popup = Some(Popup::builder(PopupType::IndexDetails).build());

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_index_details_popup_narrow() {
    // Test with narrow terminal (80x24)
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Indexes;
    harness.app.indexes = Some(vec![create_mock_index()]);
    harness.app.indexes_state.select(Some(0));
    harness.app.popup = Some(Popup::builder(PopupType::IndexDetails).build());

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_help_popup_search_screen_results_focused() {
    // Test contextual help on Search screen in ResultsFocused mode
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Search;
    harness.app.search_input_mode = SearchInputMode::ResultsFocused;
    harness.app.popup = Some(Popup::builder(PopupType::Help).build_with_context(&harness.app));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_help_popup_search_screen_query_focused() {
    // Test contextual help on Search screen in QueryFocused mode
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Search;
    harness.app.search_input_mode = SearchInputMode::QueryFocused;
    harness.app.popup = Some(Popup::builder(PopupType::Help).build_with_context(&harness.app));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_help_popup_jobs_screen() {
    // Test contextual help on Jobs screen
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Jobs;
    harness.app.popup = Some(Popup::builder(PopupType::Help).build_with_context(&harness.app));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_help_popup_indexes_screen() {
    // Test contextual help on Indexes screen
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Indexes;
    harness.app.popup = Some(Popup::builder(PopupType::Help).build_with_context(&harness.app));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_help_popup_cluster_screen() {
    // Test contextual help on Cluster screen
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Cluster;
    harness.app.popup = Some(Popup::builder(PopupType::Help).build_with_context(&harness.app));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_help_popup_macros_screen() {
    // Test contextual help on Macros screen
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = CurrentScreen::Macros;
    harness.app.popup = Some(Popup::builder(PopupType::Help).build_with_context(&harness.app));

    insta::assert_snapshot!(harness.render());
}
