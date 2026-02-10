//! Tests for profile-related action redaction.

use splunk_config::{PersistedState, SearchDefaults};

use crate::ConnectionContext;
use crate::action::tests::redacted_debug;
use crate::action::variants::Action;

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
    use splunk_client::ClientError;
    use std::sync::Arc;

    // Use ClientError instead of plain String for unified error classification
    let error = Arc::new(ClientError::ConnectionRefused("localhost:8089".to_string()));
    let action = Action::ProfileSwitchResult(Err(error));
    let output = redacted_debug(&action);

    // Error details should be redacted for security
    assert!(
        !output.contains("localhost"),
        "Should not contain connection details"
    );
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
        list_defaults: splunk_config::ListDefaults::default(),
        internal_logs_defaults: splunk_config::InternalLogsDefaults::default(),
        tutorial_completed: false,
        current_screen: "Search".to_string(),
        scroll_positions: splunk_config::ScrollPositions::default(),
        recent_export_paths: Vec::new(),
        export_format: "Json".to_string(),
        last_saved_at: None,
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
