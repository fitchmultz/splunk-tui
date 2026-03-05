//! Tests for app enable/disable operations (RQ-0135).
//!
//! This module tests:
//! - Enable app opens confirmation popup
//! - Disable app opens confirmation popup
//! - Enable already enabled app shows info
//! - Disable already disabled app shows info
//!
//! ## Invariants
//! - Enable/disable actions must open confirmation popups
//! - Already-enabled/disabled apps must show info toast, not popup
//!
//! ## Test Organization
//! Tests are grouped by operation type.

mod helpers;
use helpers::*;
use splunk_client::models::App as SplunkApp;
use splunk_tui::{CurrentScreen, ToastLevel, action::Action, app::App, app::ConnectionContext};

#[test]
fn test_apps_enable_opens_confirmation_popup() {
    let mut app = App::new(None, ConnectionContext::default());

    // Set up apps list with a disabled app
    app.apps = Some(vec![SplunkApp {
        name: "test-app".to_string(),
        label: Some("Test App".to_string()),
        version: Some("1.0.0".to_string()),
        disabled: true,
        description: None,
        author: None,
        is_configured: Some(true),
        is_visible: Some(true),
    }]);
    app.apps_state.select(Some(0));
    app.current_screen = CurrentScreen::Apps;

    // Press 'e' to enable
    let action = app.handle_input(key('e'));

    assert!(action.is_none());
    assert!(app.popup.is_some());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(splunk_tui::PopupType::ConfirmEnableApp(name)) if name == "test-app"
    ));
}

#[test]
fn test_apps_disable_opens_confirmation_popup() {
    let mut app = App::new(None, ConnectionContext::default());

    // Set up apps list with an enabled app
    app.apps = Some(vec![SplunkApp {
        name: "test-app".to_string(),
        label: Some("Test App".to_string()),
        version: Some("1.0.0".to_string()),
        disabled: false,
        description: None,
        author: None,
        is_configured: Some(true),
        is_visible: Some(true),
    }]);
    app.apps_state.select(Some(0));
    app.current_screen = CurrentScreen::Apps;

    // Press 'd' to disable
    let action = app.handle_input(key('d'));

    assert!(action.is_none());
    assert!(app.popup.is_some());
    assert!(matches!(
        app.popup.as_ref().map(|p| &p.kind),
        Some(splunk_tui::PopupType::ConfirmDisableApp(name)) if name == "test-app"
    ));
}

#[test]
fn test_apps_enable_already_enabled_shows_info() {
    let mut app = App::new(None, ConnectionContext::default());

    // Set up apps list with an already enabled app
    app.apps = Some(vec![SplunkApp {
        name: "test-app".to_string(),
        label: Some("Test App".to_string()),
        version: Some("1.0.0".to_string()),
        disabled: false,
        description: None,
        author: None,
        is_configured: Some(true),
        is_visible: Some(true),
    }]);
    app.apps_state.select(Some(0));
    app.current_screen = CurrentScreen::Apps;

    // Press 'e' to enable (but it's already enabled)
    let action = app.handle_input(key('e'));

    assert!(matches!(
        action,
        Some(Action::Notify(ToastLevel::Info, msg)) if msg.contains("already enabled")
    ));
    assert!(app.popup.is_none());
}

#[test]
fn test_apps_disable_already_disabled_shows_info() {
    let mut app = App::new(None, ConnectionContext::default());

    // Set up apps list with an already disabled app
    app.apps = Some(vec![SplunkApp {
        name: "test-app".to_string(),
        label: Some("Test App".to_string()),
        version: Some("1.0.0".to_string()),
        disabled: true,
        description: None,
        author: None,
        is_configured: Some(true),
        is_visible: Some(true),
    }]);
    app.apps_state.select(Some(0));
    app.current_screen = CurrentScreen::Apps;

    // Press 'd' to disable (but it's already disabled)
    let action = app.handle_input(key('d'));

    assert!(matches!(
        action,
        Some(Action::Notify(ToastLevel::Info, msg)) if msg.contains("already disabled")
    ));
    assert!(app.popup.is_none());
}
