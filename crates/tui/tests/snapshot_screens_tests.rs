//! Snapshot tests for various screen renderings.

mod helpers;

use helpers::{TuiHarness, create_mock_jobs, create_mock_users};
use splunk_tui::{Popup, PopupType};

#[test]
fn snapshot_indexes_screen_empty() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Indexes;
    harness.app.indexes = None;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_indexes_screen_loading() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Indexes;
    harness.app.indexes = None;
    harness.app.loading = true;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_cluster_screen_empty() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Cluster;
    harness.app.cluster_info = None;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_cluster_screen_loading() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Cluster;
    harness.app.cluster_info = None;
    harness.app.loading = true;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_job_details_screen_with_job() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::JobInspect;
    harness.app.jobs = Some(create_mock_jobs());
    // Manually populate filtered_job_indices since tests don't trigger event handlers
    harness.app.filtered_job_indices = vec![0, 1];
    harness.app.jobs_state.select(Some(0));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_job_details_screen_running_job() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::JobInspect;
    harness.app.jobs = Some(create_mock_jobs());
    // Manually populate filtered_job_indices since tests don't trigger event handlers
    harness.app.filtered_job_indices = vec![0, 1];
    harness.app.jobs_state.select(Some(1)); // Select the running job

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_job_details_screen_no_job() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::JobInspect;
    harness.app.jobs = None;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_job_details_screen_with_help_popup() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::JobInspect;
    harness.app.jobs = Some(create_mock_jobs());
    // Manually populate filtered_job_indices since tests don't trigger event handlers
    harness.app.filtered_job_indices = vec![0, 1];
    harness.app.jobs_state.select(Some(0));
    harness.app.popup = Some(Popup::builder(PopupType::Help).build());

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_internal_logs_screen() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::InternalLogs;
    harness.app.internal_logs = Some(vec![
        splunk_client::models::LogEntry {
            time: "2024-01-15T10:30:00.000Z".to_string(),
            index_time: "2024-01-15T10:30:01.000Z".to_string(),
            serial: Some(1),
            level: "INFO".to_string(),
            component: "Metrics".to_string(),
            message: "some metrics log message".to_string(),
        },
        splunk_client::models::LogEntry {
            time: "2024-01-15T10:29:00.000Z".to_string(),
            index_time: "2024-01-15T10:29:01.000Z".to_string(),
            serial: Some(2),
            level: "ERROR".to_string(),
            component: "DateParser".to_string(),
            message: "failed to parse date".to_string(),
        },
    ]);
    harness.app.internal_logs_state.select(Some(0));

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_users_screen_empty() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Users;
    harness.app.users = None;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_users_screen_loading() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Users;
    harness.app.users = None;
    harness.app.loading = true;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_users_screen_with_data() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Users;
    harness.app.users = Some(create_mock_users());
    harness.app.users_state.select(Some(0));

    insta::assert_snapshot!(harness.render());
}
