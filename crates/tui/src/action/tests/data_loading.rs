//! Tests for data loading action redaction.

use std::sync::Arc;

use splunk_client::models::{App as SplunkApp, Index, LogEntry, SavedSearch, User};

use crate::action::tests::redacted_debug;
use crate::action::variants::Action;

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
