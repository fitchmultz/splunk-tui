//! Snapshot tests for miscellaneous UI states.

mod helpers;

use helpers::{TuiHarness, create_mock_jobs};

#[test]
fn snapshot_error_state() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.toasts.push(splunk_tui::Toast::error(
        "Connection failed: timeout".to_string(),
    ));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_wide_terminal() {
    let mut harness = TuiHarness::new(120, 30);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.jobs = Some(create_mock_jobs());
    // Manually populate filtered_job_indices since tests don't trigger event handlers
    harness.app.filtered_job_indices = vec![0, 1];
    harness.app.jobs_state.select(Some(0));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_narrow_terminal() {
    let mut harness = TuiHarness::new(60, 20);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.jobs = Some(create_mock_jobs());
    // Manually populate filtered_job_indices since tests don't trigger event handlers
    harness.app.filtered_job_indices = vec![0, 1];
    harness.app.jobs_state.select(Some(0));

    insta::assert_snapshot!(harness.render());
}
