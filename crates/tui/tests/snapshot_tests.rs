//! Snapshot tests for UI rendering.
//!
//! Visual regression tests using insta to capture and verify TUI rendering.
//! Tests cover:
//! - Jobs screen with mock data
//! - Each popup variant (Help, ConfirmCancel, ConfirmDelete)
//! - Empty states (no jobs, no results)

mod helpers;

use ratatui::{Terminal, backend::TestBackend};
use splunk_client::models::{SearchJobStatus, User};
use splunk_tui::{App, Popup, PopupType};

/// Test harness for TUI rendering with a mock terminal.
struct TuiHarness {
    pub app: App,
    pub terminal: Terminal<TestBackend>,
}

impl TuiHarness {
    /// Create a new test harness with the given terminal dimensions.
    fn new(width: u16, height: u16) -> Self {
        let backend = TestBackend::new(width, height);
        let terminal = Terminal::new(backend).expect("Failed to create terminal");
        let app = App::new(None);
        Self { app, terminal }
    }

    /// Render the current app state and return the buffer contents.
    fn render(&mut self) -> String {
        self.terminal
            .draw(|f| self.app.render(f))
            .expect("Failed to render");
        buffer_to_string(self.terminal.backend().buffer())
    }
}

/// Convert a ratatui Buffer to a string for snapshot testing.
fn buffer_to_string(buffer: &ratatui::buffer::Buffer) -> String {
    let area = buffer.area();
    let mut output = String::new();

    for y in area.top()..area.bottom() {
        for x in area.left()..area.right() {
            let cell = &buffer[(x, y)];
            output.push(cell.symbol().chars().next().unwrap_or(' '));
        }
        if y < area.bottom() - 1 {
            output.push('\n');
        }
    }

    output
}

/// Create mock user data for testing.
fn create_mock_users() -> Vec<User> {
    vec![
        User {
            name: "admin".to_string(),
            realname: Some("System Administrator".to_string()),
            email: Some("admin@example.com".to_string()),
            user_type: Some("Splunk".to_string()),
            default_app: Some("launcher".to_string()),
            roles: vec!["admin".to_string(), "can_delete".to_string()],
            last_successful_login: Some(1736956200), // 2024-01-15 10:30:00 UTC
        },
        User {
            name: "power_user".to_string(),
            realname: Some("Power User".to_string()),
            email: Some("power@example.com".to_string()),
            user_type: Some("Splunk".to_string()),
            default_app: Some("search".to_string()),
            roles: vec!["power".to_string()],
            last_successful_login: Some(1736870400), // 2024-01-14 10:00:00 UTC
        },
        User {
            name: "user_no_roles".to_string(),
            realname: Some("Limited User".to_string()),
            email: None,
            user_type: None,
            default_app: None,
            roles: vec![],
            last_successful_login: None,
        },
    ]
}

/// Create mock job data for testing.
fn create_mock_jobs() -> Vec<SearchJobStatus> {
    vec![
        SearchJobStatus {
            sid: "scheduler_admin_search_1234567890".to_string(),
            is_done: true,
            is_finalized: false,
            done_progress: 1.0,
            run_duration: 5.23,
            disk_usage: 2048,
            scan_count: 1500,
            event_count: 500,
            result_count: 100,
            cursor_time: Some("2024-01-15T10:30:00.000Z".to_string()),
            priority: Some(5),
            label: Some("Scheduled search".to_string()),
        },
        SearchJobStatus {
            sid: "admin_search_9876543210".to_string(),
            is_done: false,
            is_finalized: false,
            done_progress: 0.65,
            run_duration: 12.45,
            disk_usage: 5120,
            scan_count: 5000,
            event_count: 2000,
            result_count: 450,
            cursor_time: Some("2024-01-15T10:29:00.000Z".to_string()),
            priority: Some(3),
            label: Some("Ad-hoc search".to_string()),
        },
    ]
}

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

#[test]
fn snapshot_help_popup() {
    let mut harness = TuiHarness::new(80, 24);
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
fn snapshot_search_screen_initial() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input = "index=main".to_string();

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_search_screen_loading() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input = "index=main | stats count".to_string();
    harness.app.search_status = "Running search...".to_string();
    harness.app.loading = true;
    harness.app.progress = 0.45;

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_search_screen_with_results() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input = "index=main ERROR".to_string();
    harness.app.search_status = "Search complete: index=main ERROR".to_string();
    harness.app.set_search_results(vec![
        serde_json::json!({"_time": "2024-01-15T10:30:00.000Z", "level": "ERROR", "message": "Connection failed"}),
        serde_json::json!({"_time": "2024-01-15T10:29:00.000Z", "level": "ERROR", "message": "Timeout error"}),
    ]);
    harness.app.search_sid = Some("search_12345".to_string());

    insta::assert_snapshot!(harness.render());
}

#[test]
fn snapshot_search_screen_empty() {
    let mut harness = TuiHarness::new(80, 24);
    harness.app.current_screen = splunk_tui::CurrentScreen::Search;
    harness.app.search_input.clear();
    harness.app.set_search_results(Vec::new());

    insta::assert_snapshot!(harness.render());
}

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
            level: "INFO".to_string(),
            component: "Metrics".to_string(),
            message: "some metrics log message".to_string(),
        },
        splunk_client::models::LogEntry {
            time: "2024-01-15T10:29:00.000Z".to_string(),
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
