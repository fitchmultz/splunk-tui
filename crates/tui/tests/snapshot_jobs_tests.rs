//! Snapshot tests for Jobs screen rendering.

mod helpers;

use helpers::{TuiHarness, create_mock_jobs};
use splunk_client::models::SearchJobStatus;

#[test]
fn snapshot_jobs_screen_with_data() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.jobs = Some(create_mock_jobs());
    // Manually populate filtered_job_indices since tests don't trigger event handlers
    harness.app.filtered_job_indices = vec![0, 1];
    harness.app.jobs_state.select(Some(0));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_jobs_screen_empty() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.jobs = None;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_jobs_screen_loading() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.jobs = None;
    harness.app.loading = true;
    harness.app.progress = 0.5;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_jobs_screen_auto_refresh() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;
    harness.app.jobs = Some(create_mock_jobs());
    // Manually populate filtered_job_indices since tests don't trigger event handlers
    harness.app.filtered_job_indices = vec![0, 1];
    harness.app.auto_refresh = true;
    harness.app.jobs_state.select(Some(1));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_jobs_screen_running_with_progress() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Jobs;

    // Create a job with specific progress to test the "Running (X%)" format
    // Regression test for RQ-0010: Previously used Box::leak which caused memory leaks
    let jobs = vec![SearchJobStatus {
        sid: "admin_search_with_progress".to_string(),
        is_done: false,
        is_finalized: false,
        done_progress: 0.73, // Should render as "Running (73%)"
        run_duration: 15.5,
        disk_usage: 1024,
        scan_count: 1000,
        event_count: 500,
        result_count: 250,
        cursor_time: Some("2024-01-15T10:30:00.000Z".to_string()),
        priority: Some(3),
        label: Some("Test search".to_string()),
    }];

    harness.app.jobs = Some(jobs);
    // Manually populate filtered_job_indices since tests don't trigger event handlers
    harness.app.filtered_job_indices = vec![0];
    harness.app.jobs_state.select(Some(0));

    insta::assert_snapshot!(harness.render());
}
