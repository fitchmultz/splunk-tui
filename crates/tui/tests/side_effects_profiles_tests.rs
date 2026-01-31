//! Profile side effect handler tests.
//!
//! This module tests profile-related side effect handlers including
//! OpenProfileSwitcher and SwitchToSettings.

mod common;

use common::*;

#[tokio::test]
async fn test_open_profile_switcher_with_profiles() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Add a profile to the config using ConfigManager's save_profile method
    {
        let mut cm = harness.config_manager.lock().await;
        let profile = splunk_config::ProfileConfig {
            base_url: Some("https://test.example.com:8089".to_string()),
            username: Some("admin".to_string()),
            password: Some(splunk_config::SecureValue::Plain(
                secrecy::SecretString::new("password".to_string().into()),
            )),
            api_token: None,
            skip_verify: Some(true),
            timeout_seconds: Some(30),
            max_retries: Some(3),
            session_expiry_buffer_seconds: Some(60),
            session_ttl_seconds: Some(3600),
            health_check_interval_seconds: Some(60),
        };
        cm.save_profile("test-profile", profile)
            .expect("Failed to save profile");
    }

    let actions = harness
        .handle_and_collect(Action::OpenProfileSwitcher, 1)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::OpenProfileSelectorWithList(profiles) if profiles.contains(&"test-profile".to_string()))),
        "Should send OpenProfileSelectorWithList with test-profile"
    );
}

#[tokio::test]
async fn test_open_profile_switcher_no_profiles() {
    let mut harness = SideEffectsTestHarness::new().await;

    // No profiles added to config
    let actions = harness
        .handle_and_collect(Action::OpenProfileSwitcher, 1)
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::Notify(splunk_tui::ui::ToastLevel::Error, _))),
        "Should send error notification when no profiles configured"
    );
}

#[tokio::test]
async fn test_switch_to_settings() {
    let mut harness = SideEffectsTestHarness::new().await;

    let actions = harness
        .handle_and_collect(Action::SwitchToSettings, 1)
        .await;

    assert!(
        actions.iter().any(|a| matches!(a, Action::Loading(true))),
        "Should send Loading(true)"
    );
    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::SettingsLoaded(_))),
        "Should send SettingsLoaded"
    );
}
