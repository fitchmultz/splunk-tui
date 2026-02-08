//! Tests for copy to clipboard and Ctrl+C handlers across all screens.
//!
//! This module tests:
//! - Ctrl+C on Search screen (query vs results)
//! - Ctrl+C on Jobs screen (selected job SID)
//! - Ctrl+C on Indexes screen (selected index name)
//! - Ctrl+C on SavedSearches screen (selected search name)
//! - Ctrl+C on Apps screen (selected app name)
//! - Ctrl+C on Users screen (selected username)
//! - Ctrl+C on InternalLogs screen (selected log message)
//! - Ctrl+C on Cluster screen (cluster ID)
//! - Ctrl+C on Health screen (health status)
//! - Copy action success/failure handling
//!
//! ## Invariants
//! - Ctrl+C must copy contextually relevant data for each screen
//! - Copy success must emit info toast
//! - Copy failure must emit error toast
//!
//! ## Test Organization
//! Tests are grouped by screen.

mod helpers;
use helpers::*;
use splunk_client::models::{App as SplunkApp, Index, LogEntry, SavedSearch, User};
use splunk_tui::{CurrentScreen, ToastLevel, action::Action, app::App, app::ConnectionContext};
use std::collections::HashMap;

#[test]
fn test_ctrl_c_copies_search_query_when_no_results() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;
    app.search_input.set_value("index=_internal | head 5");
    app.search_results.clear();

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "index=_internal | head 5"),
        "Ctrl+C should emit CopyToClipboard(query)"
    );
}

#[test]
fn test_ctrl_c_copies_current_search_result_when_results_exist() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    let v = serde_json::json!({"foo":"bar","n":1});
    app.set_search_results(vec![v.clone()]);
    app.search_scroll_offset = 0;

    let expected = serde_json::to_string_pretty(&v).unwrap();
    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == expected),
        "Ctrl+C should emit CopyToClipboard(pretty_json)"
    );
}

#[test]
fn test_ctrl_c_copies_selected_job_sid() {
    use splunk_client::models::SearchJobStatus;

    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Jobs;

    let jobs: Vec<SearchJobStatus> = (0..3)
        .map(|i| SearchJobStatus {
            sid: format!("sid_{}", i),
            is_done: i % 2 == 0,
            is_finalized: false,
            done_progress: 0.5,
            run_duration: 10.0,
            disk_usage: 1024,
            scan_count: 100,
            event_count: 50,
            result_count: 25,
            cursor_time: None,
            priority: None,
            label: None,
        })
        .collect();

    app.update(Action::JobsLoaded(Ok(jobs)));
    app.jobs_state.select(Some(1));

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "sid_1"),
        "Ctrl+C should copy selected job SID"
    );
}

#[test]
fn test_ctrl_c_copies_selected_index_name() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Indexes;
    app.indexes = Some(vec![Index {
        name: "main".to_string(),
        total_event_count: 1,
        current_db_size_mb: 1,
        max_total_data_size_mb: None,
        max_warm_db_count: None,
        max_hot_buckets: None,
        frozen_time_period_in_secs: None,
        cold_db_path: None,
        home_path: None,
        thawed_path: None,
        cold_to_frozen_dir: None,
        primary_index: None,
    }]);
    app.indexes_state.select(Some(0));

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "main"),
        "Ctrl+C should copy selected index name"
    );
}

#[test]
fn test_ctrl_c_copies_selected_saved_search_name() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::SavedSearches;
    app.saved_searches = Some(vec![SavedSearch {
        name: "Errors Last 24 Hours".to_string(),
        search: "index=_internal error".to_string(),
        description: None,
        disabled: false,
    }]);
    app.saved_searches_state.select(Some(0));

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "Errors Last 24 Hours"),
        "Ctrl+C should copy selected saved search name"
    );
}

#[test]
fn test_ctrl_c_copies_selected_app_name() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Apps;
    app.apps = Some(vec![SplunkApp {
        name: "search".to_string(),
        label: Some("Search".to_string()),
        version: Some("1.0.0".to_string()),
        is_configured: None,
        is_visible: None,
        disabled: false,
        description: None,
        author: None,
    }]);
    app.apps_state.select(Some(0));

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "search"),
        "Ctrl+C should copy selected app name"
    );
}

#[test]
fn test_ctrl_c_copies_selected_username() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Users;
    app.users = Some(vec![User {
        name: "admin".to_string(),
        realname: Some("Administrator".to_string()),
        email: None,
        user_type: None,
        default_app: None,
        roles: vec!["admin".to_string()],
        last_successful_login: None,
    }]);
    app.users_state.select(Some(0));

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "admin"),
        "Ctrl+C should copy selected username"
    );
}

#[test]
fn test_ctrl_c_copies_selected_log_message() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::InternalLogs;
    app.internal_logs = Some(vec![LogEntry {
        time: "2024-01-01 12:00:00".to_string(),
        index_time: String::new(),
        serial: None,
        level: splunk_client::models::LogLevel::Error,
        component: "Test".to_string(),
        message: "Something went wrong".to_string(),
    }]);
    app.internal_logs_state.select(Some(0));

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "Something went wrong"),
        "Ctrl+C should copy selected log message"
    );
}

#[test]
fn test_ctrl_c_copies_cluster_id() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Cluster;
    app.cluster_info = Some(splunk_client::models::ClusterInfo {
        id: "cluster-123".to_string(),
        label: None,
        mode: splunk_client::models::ClusterMode::Manager,
        manager_uri: None,
        replication_factor: None,
        search_factor: None,
        status: None,
        maintenance_mode: None,
    });

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "cluster-123"),
        "Ctrl+C should copy cluster ID"
    );
}

#[test]
fn test_ctrl_c_copies_health_status() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Health;
    app.health_info = Some(splunk_client::models::HealthCheckOutput {
        server_info: None,
        splunkd_health: Some(splunk_client::models::SplunkHealth {
            health: splunk_client::models::HealthStatus::Green,
            features: HashMap::new(),
        }),
        license_usage: None,
        kvstore_status: None,
        log_parsing_health: None,
    });

    let action = app.handle_input(ctrl_key('c'));
    assert!(
        matches!(action, Some(Action::CopyToClipboard(s)) if s == "green"),
        "Ctrl+C should copy health status"
    );
}

#[test]
fn test_copy_to_clipboard_action_success_emits_info_toast_and_records_text() {
    let guard = splunk_tui::app::clipboard::install_recording_clipboard();

    let mut app = App::new(None, ConnectionContext::default());
    app.update(Action::CopyToClipboard("hello world".to_string()));

    assert!(
        guard.copied_text().as_deref() == Some("hello world"),
        "Recording clipboard should capture copied content"
    );
    assert!(!app.toasts.is_empty(), "Should emit a toast on success");
    assert_eq!(app.toasts.last().unwrap().level, ToastLevel::Info);
    assert!(
        app.toasts.last().unwrap().message.starts_with("Copied:"),
        "Success toast should begin with 'Copied:'"
    );
}

#[test]
fn test_copy_to_clipboard_action_failure_emits_error_toast() {
    let _guard = splunk_tui::app::clipboard::install_failing_clipboard("boom");

    let mut app = App::new(None, ConnectionContext::default());
    app.update(Action::CopyToClipboard("hello".to_string()));

    assert!(!app.toasts.is_empty(), "Should emit a toast on failure");
    assert_eq!(app.toasts.last().unwrap().level, ToastLevel::Error);
    assert!(
        app.toasts
            .last()
            .unwrap()
            .message
            .contains("Clipboard error: boom"),
        "Error toast should include clipboard error message"
    );
}

#[test]
fn test_typing_e_in_search_query_does_not_trigger_export() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Search;

    // Ensure export would be available if Ctrl+E were pressed.
    app.set_search_results(vec![serde_json::json!({"foo": "bar"})]);

    // Plain 'e' should type into the query, not open the export popup.
    app.search_input.set_value("s");
    app.search_input.set_cursor_position(1); // Cursor at end
    app.handle_input(key('e'));

    assert_eq!(
        app.search_input.value(),
        "se",
        "Should append 'e' to query input"
    );
    assert!(
        app.popup.is_none(),
        "Should not open export popup on plain 'e'"
    );
}
