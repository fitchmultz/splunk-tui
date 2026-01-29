//! Tests for action redaction and security.
//!
//! This module contains comprehensive tests for the `RedactedAction` wrapper
//! to ensure sensitive data is never logged.

use std::path::PathBuf;
use std::sync::Arc;

use splunk_client::models::{
    App as SplunkApp, ClusterInfo, ClusterPeer, HealthCheckOutput, Index, LogEntry, SavedSearch,
    SearchJobStatus, SplunkHealth, User,
};
use splunk_config::{PersistedState, SearchDefaults};

use crate::ConnectionContext;
use crate::action::format::ExportFormat;
use crate::action::redaction::RedactedAction;
use crate::action::variants::Action;

fn redacted_debug(action: &Action) -> String {
    format!("{:?}", RedactedAction(action))
}

#[test]
fn test_redact_run_search() {
    let action = Action::RunSearch {
        query: "SELECT * FROM users WHERE password='secret'".to_string(),
        search_defaults: SearchDefaults::default(),
    };
    let output = redacted_debug(&action);

    assert!(
        !output.contains("password"),
        "Should not contain sensitive password"
    );
    assert!(!output.contains("secret"), "Should not contain secret word");
    assert!(output.contains("RunSearch"), "Should contain action name");
    assert!(output.contains("43 chars"), "Should show size indicator");
}

#[test]
fn test_redact_copy_to_clipboard() {
    let action = Action::CopyToClipboard("{\"user\":\"alice\",\"token\":\"abc123\"}".to_string());
    let output = redacted_debug(&action);

    assert!(!output.contains("alice"), "Should not contain user name");
    assert!(!output.contains("abc123"), "Should not contain token");
    assert!(
        output.contains("CopyToClipboard"),
        "Should contain action name"
    );
    assert!(output.contains("33 chars"), "Should show size indicator");
}

#[test]
fn test_redact_export_data() {
    let data = serde_json::json!({"results": [{"id": 1, "password": "secret123"}]});
    let path = PathBuf::from("/tmp/export.json");
    let action = Action::ExportData(data.clone(), path, ExportFormat::Json);
    let output = redacted_debug(&action);

    assert!(
        !output.contains("secret123"),
        "Should not contain sensitive data"
    );
    assert!(output.contains("ExportData"), "Should contain action name");
    assert!(output.contains("bytes"), "Should show bytes indicator");
}

#[test]
fn test_redact_notify() {
    let action = Action::Notify(
        crate::ui::ToastLevel::Error,
        "Failed to authenticate: invalid token xyz789".to_string(),
    );
    let output = redacted_debug(&action);

    assert!(!output.contains("xyz789"), "Should not contain token");
    assert!(output.contains("Notify"), "Should contain action name");
    assert!(output.contains("Error"), "Should contain toast level");
    assert!(output.contains("chars"), "Should show size indicator");
}

#[test]
fn test_show_cancel_job_sid() {
    let action = Action::CancelJob("search_job_12345_789".to_string());
    let output = redacted_debug(&action);

    assert!(output.contains("CancelJob"), "Should contain action name");
    assert!(
        output.contains("search_job_12345_789"),
        "Should show SID for debugging"
    );
}

#[test]
fn test_show_delete_job_sid() {
    let action = Action::DeleteJob("search_job_98765_4321".to_string());
    let output = redacted_debug(&action);

    assert!(output.contains("DeleteJob"), "Should contain action name");
    assert!(
        output.contains("search_job_98765_4321"),
        "Should show SID for debugging"
    );
}

#[test]
fn test_show_batch_operation_counts() {
    let sids = vec!["job1".to_string(), "job2".to_string(), "job3".to_string()];
    let action = Action::CancelJobsBatch(sids);
    let output = redacted_debug(&action);

    assert!(
        output.contains("CancelJobsBatch"),
        "Should contain action name"
    );
    assert!(
        output.contains("3 job(s)"),
        "Should show count but not SIDs"
    );
    assert!(!output.contains("job1"), "Should not show individual SIDs");
}

#[test]
fn test_show_search_input() {
    let action = Action::SearchInput('s');
    let output = redacted_debug(&action);

    assert!(output.contains("SearchInput"), "Should contain action name");
    assert!(
        output.contains("'s'"),
        "Should show character for input debugging"
    );
}

#[test]
fn test_non_sensitive_action_shown_fully() {
    let action = Action::Quit;
    let output = redacted_debug(&action);

    assert!(output.contains("Quit"), "Should show simple action fully");
}

#[test]
fn test_unicode_in_payload() {
    let action = Action::CopyToClipboard("日本語テスト 🇯🇵".to_string());
    let output = redacted_debug(&action);

    assert!(
        !output.contains("日本語"),
        "Should not contain Unicode content"
    );
    assert!(output.contains("chars"), "Should show character count");
}

#[test]
fn test_redact_search_started() {
    let action = Action::SearchStarted("SELECT * FROM users WHERE password='secret'".to_string());
    let output = redacted_debug(&action);

    assert!(
        !output.contains("password"),
        "Should not contain sensitive password"
    );
    assert!(!output.contains("secret"), "Should not contain secret word");
    assert!(
        output.contains("SearchStarted"),
        "Should contain action name"
    );
    assert!(output.contains("43 chars"), "Should show size indicator");
}

#[test]
fn test_redact_search_complete_ok() {
    let results = vec![
        serde_json::json!({"password": "secret123", "user": "admin"}),
        serde_json::json!({"token": "abc456", "user": "bob"}),
    ];
    let action = Action::SearchComplete(Ok((results, "search_job_12345".to_string(), Some(100))));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("secret123"),
        "Should not contain sensitive data from results"
    );
    assert!(!output.contains("abc456"), "Should not contain token");
    assert!(
        !output.contains("admin"),
        "Should not contain user names from results"
    );
    assert!(
        output.contains("SearchComplete"),
        "Should contain action name"
    );
    assert!(output.contains("2 results"), "Should show result count");
    assert!(output.contains("sid=search_job_12345"), "Should show SID");
    assert!(
        output.contains("total=Some(100)"),
        "Should show total count"
    );
}

#[test]
fn test_redact_search_complete_err() {
    let action = Action::SearchComplete(Err((
        "Authentication failed for user admin".to_string(),
        crate::error_details::ErrorDetails::from_error_string("auth failed"),
    )));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("Authentication failed"),
        "Should not contain error message"
    );
    assert!(!output.contains("admin"), "Should not contain user name");
    assert!(
        output.contains("SearchComplete"),
        "Should contain action name"
    );
    assert!(output.contains("<error>"), "Should show error indicator");
}

#[test]
fn test_redact_more_search_results_loaded_ok() {
    let results = vec![serde_json::json!({"password": "secret789", "data": "sensitive"})];
    let action = Action::MoreSearchResultsLoaded(Ok((results, 50, Some(200))));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("secret789"),
        "Should not contain sensitive data"
    );
    assert!(
        !output.contains("sensitive"),
        "Should not contain data content"
    );
    assert!(
        output.contains("MoreSearchResultsLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("1 results"), "Should show result count");
    assert!(output.contains("offset=50"), "Should show offset");
    assert!(output.contains("total=Some(200)"), "Should show total");
}

#[test]
fn test_redact_more_search_results_loaded_err() {
    let error = splunk_client::ClientError::ConnectionRefused("test".to_string());
    let action = Action::MoreSearchResultsLoaded(Err(Arc::new(error)));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("Failed to fetch"),
        "Should not contain error message"
    );
    assert!(!output.contains("bob"), "Should not contain user name");
    assert!(
        output.contains("MoreSearchResultsLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("<error>"), "Should show error indicator");
}

#[test]
fn test_redact_indexes_loaded() {
    let indexes = vec![
        Index {
            name: "internal".to_string(),
            max_total_data_size_mb: None,
            current_db_size_mb: 100,
            total_event_count: 1000,
            max_warm_db_count: None,
            max_hot_buckets: None,
            frozen_time_period_in_secs: None,
            cold_db_path: None,
            home_path: None,
            thawed_path: None,
            cold_to_frozen_dir: None,
            primary_index: None,
        },
        Index {
            name: "main".to_string(),
            max_total_data_size_mb: None,
            current_db_size_mb: 200,
            total_event_count: 2000,
            max_warm_db_count: None,
            max_hot_buckets: None,
            frozen_time_period_in_secs: None,
            cold_db_path: None,
            home_path: None,
            thawed_path: None,
            cold_to_frozen_dir: None,
            primary_index: None,
        },
    ];
    let action = Action::IndexesLoaded(Ok(indexes));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("internal"),
        "Should not contain index name"
    );
    assert!(
        !output.contains("/opt/splunk"),
        "Should not contain path data"
    );
    assert!(
        output.contains("IndexesLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("2 items"), "Should show item count");
}

#[test]
fn test_redact_indexes_loaded_err() {
    let error = splunk_client::ClientError::InvalidResponse("test".to_string());
    let action = Action::IndexesLoaded(Err(Arc::new(error)));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("Failed to load"),
        "Should not contain error message"
    );
    assert!(
        output.contains("IndexesLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("<error>"), "Should show error indicator");
}

#[test]
fn test_redact_jobs_loaded() {
    let jobs = vec![
        SearchJobStatus {
            sid: "job1".to_string(),
            is_done: true,
            is_finalized: true,
            done_progress: 1.0,
            run_duration: 1.0,
            cursor_time: None,
            scan_count: 100,
            event_count: 50,
            result_count: 25,
            disk_usage: 1024,
            priority: None,
            label: None,
        },
        SearchJobStatus {
            sid: "job2".to_string(),
            is_done: false,
            is_finalized: false,
            done_progress: 0.5,
            run_duration: 0.5,
            cursor_time: None,
            scan_count: 50,
            event_count: 25,
            result_count: 10,
            disk_usage: 512,
            priority: None,
            label: None,
        },
    ];
    let action = Action::JobsLoaded(Ok(jobs));
    let output = redacted_debug(&action);

    assert!(!output.contains("job1"), "Should not contain job SID");
    assert!(!output.contains("job2"), "Should not contain job SID");
    assert!(output.contains("JobsLoaded"), "Should contain action name");
    assert!(output.contains("2 items"), "Should show item count");
}

#[test]
fn test_redact_saved_searches_loaded() {
    let searches = vec![SavedSearch {
        name: "Admin Activity".to_string(),
        search: "search user=admin".to_string(),
        description: None,
        disabled: false,
    }];
    let action = Action::SavedSearchesLoaded(Ok(searches));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("Admin Activity"),
        "Should not contain search name"
    );
    assert!(
        !output.contains("user=admin"),
        "Should not contain search query"
    );
    assert!(
        output.contains("SavedSearchesLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("1 items"), "Should show item count");
}

#[test]
fn test_redact_internal_logs_loaded() {
    let logs = vec![
        LogEntry {
            time: "2025-01-20T10:30:00.000Z".to_string(),
            index_time: "2025-01-20T10:30:01.000Z".to_string(),
            serial: None,
            level: "INFO".to_string(),
            component: "Auth".to_string(),
            message: "User admin logged in".to_string(),
        },
        LogEntry {
            time: "2025-01-20T10:31:00.000Z".to_string(),
            index_time: "2025-01-20T10:31:01.000Z".to_string(),
            serial: None,
            level: "INFO".to_string(),
            component: "Token".to_string(),
            message: "Token abc123 generated".to_string(),
        },
    ];
    let action = Action::InternalLogsLoaded(Ok(logs));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("admin logged in"),
        "Should not contain log messages"
    );
    assert!(!output.contains("abc123"), "Should not contain token");
    assert!(
        output.contains("InternalLogsLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("2 items"), "Should show item count");
}

#[test]
fn test_redact_apps_loaded() {
    let apps = vec![SplunkApp {
        name: "search".to_string(),
        label: Some("Search & Reporting".to_string()),
        version: None,
        is_configured: None,
        is_visible: None,
        disabled: false,
        description: None,
        author: None,
    }];
    let action = Action::AppsLoaded(Ok(apps));
    let output = redacted_debug(&action);

    assert!(!output.contains("search"), "Should not contain app name");
    assert!(
        !output.contains("Search & Reporting"),
        "Should not contain app label"
    );
    assert!(output.contains("AppsLoaded"), "Should contain action name");
    assert!(output.contains("1 items"), "Should show item count");
}

#[test]
fn test_redact_users_loaded() {
    let users = vec![User {
        name: "admin".to_string(),
        realname: Some("Administrator".to_string()),
        email: None,
        user_type: None,
        default_app: None,
        roles: vec![],
        last_successful_login: None,
    }];
    let action = Action::UsersLoaded(Ok(users));
    let output = redacted_debug(&action);

    assert!(!output.contains("admin"), "Should not contain username");
    assert!(
        !output.contains("Administrator"),
        "Should not contain real name"
    );
    assert!(output.contains("UsersLoaded"), "Should contain action name");
    assert!(output.contains("1 items"), "Should show item count");
}

#[test]
fn test_redact_cluster_peers_loaded() {
    let peers = vec![
        ClusterPeer {
            id: "peer1-id".to_string(),
            label: Some("peer1".to_string()),
            status: "Up".to_string(),
            peer_state: "Active".to_string(),
            site: None,
            guid: "guid1".to_string(),
            host: "internal-host1".to_string(),
            port: 8080,
            replication_count: None,
            replication_status: None,
            bundle_replication_count: None,
            is_captain: None,
        },
        ClusterPeer {
            id: "peer2-id".to_string(),
            label: Some("peer2".to_string()),
            status: "Up".to_string(),
            peer_state: "Active".to_string(),
            site: None,
            guid: "guid2".to_string(),
            host: "internal-host2".to_string(),
            port: 8080,
            replication_count: None,
            replication_status: None,
            bundle_replication_count: None,
            is_captain: None,
        },
    ];
    let action = Action::ClusterPeersLoaded(Ok(peers));
    let output = redacted_debug(&action);

    assert!(!output.contains("peer1"), "Should not contain peer name");
    assert!(
        !output.contains("internal-host1"),
        "Should not contain host"
    );
    assert!(
        output.contains("ClusterPeersLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("2 items"), "Should show item count");
}

#[test]
fn test_redact_cluster_info_loaded() {
    let info = ClusterInfo {
        id: "cluster1-id".to_string(),
        label: Some("cluster1".to_string()),
        mode: "master".to_string(),
        manager_uri: None,
        replication_factor: None,
        search_factor: None,
        status: None,
    };
    let action = Action::ClusterInfoLoaded(Ok(info));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("cluster1"),
        "Should not contain cluster name"
    );
    assert!(
        output.contains("ClusterInfoLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("<data>"), "Should show data indicator");
}

#[test]
fn test_redact_health_loaded() {
    let health = HealthCheckOutput {
        server_info: None,
        splunkd_health: None,
        license_usage: None,
        kvstore_status: None,
        log_parsing_health: None,
    };
    let action = Action::HealthLoaded(Box::new(Ok(health)));
    let output = redacted_debug(&action);

    assert!(
        output.contains("HealthLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("<data>"), "Should show data indicator");
}

#[test]
fn test_redact_health_status_loaded() {
    let health = SplunkHealth {
        health: "yellow".to_string(),
        features: std::collections::HashMap::new(),
    };
    let action = Action::HealthStatusLoaded(Ok(health));
    let output = redacted_debug(&action);

    assert!(!output.contains("yellow"), "Should not contain status");
    assert!(
        output.contains("HealthStatusLoaded"),
        "Should contain action name"
    );
    assert!(output.contains("<data>"), "Should show data indicator");
}

#[test]
fn test_redact_open_profile_selector_with_list() {
    let profiles = vec![
        "production".to_string(),
        "staging".to_string(),
        "admin-profile".to_string(),
    ];
    let action = Action::OpenProfileSelectorWithList(profiles);
    let output = redacted_debug(&action);

    assert!(
        !output.contains("production"),
        "Should not contain profile name"
    );
    assert!(
        !output.contains("admin-profile"),
        "Should not contain admin profile name"
    );
    assert!(
        output.contains("OpenProfileSelectorWithList"),
        "Should contain action name"
    );
    assert!(output.contains("3 profiles"), "Should show profile count");
}

#[test]
fn test_redact_profile_switch_result_ok() {
    let action = Action::ProfileSwitchResult(Ok(ConnectionContext::default()));
    let output = redacted_debug(&action);

    assert!(
        output.contains("ProfileSwitchResult"),
        "Should contain action name"
    );
    assert!(output.contains("Ok"), "Should show Ok");
    assert!(
        !output.contains("ConnectionContext"),
        "Should not contain ConnectionContext details"
    );
}

#[test]
fn test_redact_profile_switch_result_err() {
    let action =
        Action::ProfileSwitchResult(Err("Failed to connect with token abc123".to_string()));
    let output = redacted_debug(&action);

    assert!(
        !output.contains("Failed to connect"),
        "Should not contain error message"
    );
    assert!(!output.contains("abc123"), "Should not contain token");
    assert!(
        output.contains("ProfileSwitchResult"),
        "Should contain action name"
    );
    assert!(output.contains("Err"), "Should show Err");
}

#[test]
fn test_redact_profile_selected() {
    let action = Action::ProfileSelected("production-admin".to_string());
    let output = redacted_debug(&action);

    assert!(
        !output.contains("production-admin"),
        "Should not contain profile name"
    );
    assert!(
        output.contains("ProfileSelected"),
        "Should contain action name"
    );
    assert!(
        output.contains("<redacted>"),
        "Should show redacted indicator"
    );
}

#[test]
fn test_redact_settings_loaded() {
    let state = PersistedState {
        auto_refresh: true,
        sort_column: "sid".to_string(),
        sort_direction: "asc".to_string(),
        last_search_query: Some("password='secret123'".to_string()),
        search_history: vec![
            "search user=admin".to_string(),
            "password='abc456'".to_string(),
        ],
        selected_theme: splunk_config::ColorTheme::Dark,
        search_defaults: SearchDefaults::default(),
        keybind_overrides: splunk_config::KeybindOverrides::default(),
    };
    let action = Action::SettingsLoaded(state);
    let output = redacted_debug(&action);

    assert!(
        !output.contains("secret123"),
        "Should not contain sensitive query data"
    );
    assert!(
        !output.contains("password"),
        "Should not contain password keyword"
    );
    assert!(
        !output.contains("admin"),
        "Should not contain user name from search history"
    );
    assert!(
        output.contains("SettingsLoaded"),
        "Should contain action name"
    );
    assert!(
        output.contains("<redacted>"),
        "Should show redacted indicator"
    );
}

#[test]
fn test_redact_show_error_details() {
    let details = crate::error_details::ErrorDetails::from_error_string(
        "Authentication failed for user admin with password secret123",
    );
    let action = Action::ShowErrorDetails(details);
    let output = redacted_debug(&action);

    assert!(
        !output.contains("Authentication failed"),
        "Should not contain error message"
    );
    assert!(!output.contains("admin"), "Should not contain user name");
    assert!(!output.contains("secret123"), "Should not contain password");
    assert!(
        output.contains("ShowErrorDetails"),
        "Should contain action name"
    );
    assert!(
        output.contains("<redacted>"),
        "Should show redacted indicator"
    );
}

#[test]
fn test_show_error_details_from_current() {
    let action = Action::ShowErrorDetailsFromCurrent;
    let output = redacted_debug(&action);

    assert!(
        output.contains("ShowErrorDetailsFromCurrent"),
        "Should contain action name"
    );
}
