//! Tests for configs screen data loading.
//!
//! This module tests:
//! - ConfigFilesLoaded populates config_files state
//! - ConfigStanzasLoaded populates config_stanzas state
//! - Error cases show toasts and reset loading state

use splunk_client::models::ConfigFile;
use splunk_tui::{action::Action, app::App, app::ConnectionContext, app::state::CurrentScreen};
use std::collections::HashMap;
use std::sync::Arc;

fn create_test_config_files() -> Vec<ConfigFile> {
    vec![
        ConfigFile {
            name: "props".to_string(),
            title: "props.conf".to_string(),
            description: Some("Properties configuration".to_string()),
        },
        ConfigFile {
            name: "transforms".to_string(),
            title: "transforms.conf".to_string(),
            description: Some("Transformations".to_string()),
        },
    ]
}

fn create_test_config_stanzas() -> Vec<splunk_client::models::ConfigStanza> {
    vec![
        splunk_client::models::ConfigStanza {
            name: "default".to_string(),
            config_file: "props".to_string(),
            settings: {
                let mut m = HashMap::new();
                m.insert("TRUNCATE".to_string(), serde_json::json!(10000));
                m
            },
        },
        splunk_client::models::ConfigStanza {
            name: "access_combined".to_string(),
            config_file: "props".to_string(),
            settings: {
                let mut m = HashMap::new();
                m.insert(
                    "EXTRACT-access".to_string(),
                    serde_json::json!("^(?<client_ip>\\S+)"),
                );
                m
            },
        },
    ]
}

#[test]
fn test_config_files_loaded_populates_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Configs;
    app.loading = true;

    let files = create_test_config_files();
    app.update(Action::ConfigFilesLoaded(Ok(files)));

    assert!(
        app.config_files.is_some(),
        "config_files should be populated"
    );
    assert_eq!(app.config_files.as_ref().unwrap().len(), 2);
    assert!(!app.loading, "loading should be reset");
}

#[test]
fn test_config_files_loaded_error_shows_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Configs;
    app.loading = true;

    let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
    app.update(Action::ConfigFilesLoaded(Err(Arc::new(error))));

    assert!(
        app.config_files.is_none(),
        "config_files should remain None on error"
    );
    assert!(!app.loading, "loading should be reset");
    assert_eq!(app.toasts.len(), 1, "should show error toast");
    assert!(
        app.toasts[0].message.contains("config files"),
        "toast should mention config files: {}",
        app.toasts[0].message
    );
}

#[test]
fn test_config_stanzas_loaded_populates_state() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Configs;
    app.loading = true;
    app.config_view_mode = splunk_tui::ui::screens::configs::ConfigViewMode::StanzaList;

    let stanzas = create_test_config_stanzas();
    app.update(Action::ConfigStanzasLoaded(Ok(stanzas)));

    assert!(
        app.config_stanzas.is_some(),
        "config_stanzas should be populated"
    );
    assert_eq!(app.config_stanzas.as_ref().unwrap().len(), 2);
    assert!(!app.loading, "loading should be reset");
    // filtered_stanza_indices should be rebuilt
    assert_eq!(app.filtered_stanza_indices.len(), 2);
}

#[test]
fn test_config_stanzas_loaded_error_shows_toast() {
    let mut app = App::new(None, ConnectionContext::default());
    app.current_screen = CurrentScreen::Configs;
    app.loading = true;

    let error = splunk_client::ClientError::ConnectionRefused("test error".to_string());
    app.update(Action::ConfigStanzasLoaded(Err(Arc::new(error))));

    assert!(
        app.config_stanzas.is_none(),
        "config_stanzas should remain None on error"
    );
    assert!(!app.loading, "loading should be reset");
    assert_eq!(app.toasts.len(), 1, "should show error toast");
    assert!(
        app.toasts[0].message.contains("config stanzas"),
        "toast should mention config stanzas: {}",
        app.toasts[0].message
    );
}

#[test]
fn test_config_files_loaded_via_update_routing() {
    // Verify that ConfigFilesLoaded is routed through update() to the data loading handler
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    let files = vec![ConfigFile {
        name: "inputs".to_string(),
        title: "inputs.conf".to_string(),
        description: None,
    }];

    // This tests the routing through actions.rs match arm
    app.update(Action::ConfigFilesLoaded(Ok(files)));

    assert!(app.config_files.is_some());
    assert_eq!(app.config_files.as_ref().unwrap()[0].name, "inputs");
    assert!(!app.loading);
}

#[test]
fn test_config_stanzas_loaded_via_update_routing() {
    // Verify that ConfigStanzasLoaded is routed through update() to the data loading handler
    let mut app = App::new(None, ConnectionContext::default());
    app.loading = true;

    let stanzas = vec![splunk_client::models::ConfigStanza {
        name: "monitor:///var/log".to_string(),
        config_file: "inputs".to_string(),
        settings: HashMap::new(),
    }];

    // This tests the routing through actions.rs match arm
    app.update(Action::ConfigStanzasLoaded(Ok(stanzas)));

    assert!(app.config_stanzas.is_some());
    assert_eq!(
        app.config_stanzas.as_ref().unwrap()[0].name,
        "monitor:///var/log"
    );
    assert!(!app.loading);
}
