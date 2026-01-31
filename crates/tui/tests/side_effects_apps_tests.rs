//! Apps side effect handler tests.
//!
//! This module tests app-related side effect handlers including
//! LoadApps, EnableApp, and DisableApp.

mod common;

use common::*;
use wiremock::matchers::{method, path};

#[tokio::test]
async fn test_load_apps_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    let fixture = serde_json::json!({
        "entry": [
            {
                "name": "search",
                "content": {
                    "label": "Search & Reporting",
                    "version": "1.0",
                    "disabled": false
                }
            },
            {
                "name": "launcher",
                "content": {
                    "label": "Home",
                    "version": "1.0",
                    "disabled": false
                }
            }
        ]
    });

    Mock::given(method("GET"))
        .and(path("/services/apps/local"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(
            Action::LoadApps {
                count: 100,
                offset: 0,
            },
            2,
        )
        .await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::AppsLoaded(Ok(_)))),
        "Should send AppsLoaded(Ok)"
    );
}

#[tokio::test]
async fn test_enable_app_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("POST"))
        .and(path("/services/apps/local/test-app/enable"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&harness.mock_server)
        .await;

    // Mock apps list reload
    let apps_fixture = serde_json::json!({
        "entry": [{"name": "test-app", "content": {"disabled": false}}]
    });
    Mock::given(method("GET"))
        .and(path("/services/apps/local"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&apps_fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::EnableApp("test-app".to_string()), 2)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Success, _))),
        "Should send success notification"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::LoadApps { .. })),
        "Should send LoadApps to refresh"
    );
}

#[tokio::test]
async fn test_enable_app_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("POST"))
        .and(path("/services/apps/local/test-app/enable"))
        .respond_with(ResponseTemplate::new(404).set_body_string("App not found"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::EnableApp("test-app".to_string()), 2)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Error, _))),
        "Should send error notification"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(false))),
        "Should send Loading(false) on error"
    );
}

#[tokio::test]
async fn test_disable_app_success() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("POST"))
        .and(path("/services/apps/local/test-app/disable"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&harness.mock_server)
        .await;

    // Mock apps list reload
    let apps_fixture = serde_json::json!({
        "entry": [{"name": "test-app", "content": {"disabled": true}}]
    });
    Mock::given(method("GET"))
        .and(path("/services/apps/local"))
        .respond_with(ResponseTemplate::new(200).set_body_json(&apps_fixture))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::DisableApp("test-app".to_string()), 2)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Success, _))),
        "Should send success notification"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::LoadApps { .. })),
        "Should send LoadApps to refresh"
    );
}

#[tokio::test]
async fn test_disable_app_error() {
    let mut harness = SideEffectsTestHarness::new().await;

    Mock::given(method("POST"))
        .and(path("/services/apps/local/test-app/disable"))
        .respond_with(ResponseTemplate::new(500).set_body_string("Server error"))
        .mount(&harness.mock_server)
        .await;

    let actions = harness
        .handle_and_collect(Action::DisableApp("test-app".to_string()), 2)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Error, _))),
        "Should send error notification"
    );
    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(false))),
        "Should send Loading(false) on error"
    );
}
