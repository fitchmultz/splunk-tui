//! Snapshot tests for footer hints rendering.

mod helpers;

use helpers::{TuiHarness, create_mock_index, create_mock_jobs};

#[test]
fn snapshot_footer_hints_search_screen() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input.set_value("index=main");

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_footer_hints_jobs_screen() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.jobs = Some(create_mock_jobs());
    harness.app.filtered_job_indices = vec![0, 1];
    harness.app.jobs_state.select(Some(0));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_footer_hints_indexes_screen() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Indexes;
    harness.app.indexes = Some(vec![create_mock_index()]);
    harness.app.indexes_state.select(Some(0));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_footer_hints_cluster_screen() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Cluster;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_footer_hints_apps_screen() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Apps;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_footer_hints_settings_screen() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Settings;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_footer_hints_narrow_terminal() {
    // Test footer hints truncation on narrow terminal (60 cols)
    let mut harness = TuiHarness::new(60, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.jobs = Some(create_mock_jobs());
    harness.app.filtered_job_indices = vec![0, 1];
    harness.app.jobs_state.select(Some(0));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_footer_hints_job_inspect_screen() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::JobInspect;
    harness.app.jobs = Some(create_mock_jobs());
    harness.app.filtered_job_indices = vec![0, 1];
    harness.app.jobs_state.select(Some(0));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_footer_hints_with_loading() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.loading = true;
    harness.app.progress = 0.65;

    insta::assert_snapshot!(harness.render());
}
