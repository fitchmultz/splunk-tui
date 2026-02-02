//! Profile side effect handler tests.
//!
//! This module tests profile-related side effect handlers including
//! OpenProfileSwitcher and SwitchToSettings.

mod common;

use common::*;
use splunk_config::ProfileConfig;

#[tokio::test]
async fn test_save_profile_without_rename() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Create initial profile
    {
        let mut cm = harness.config_manager.lock().await;
        let profile = ProfileConfig {
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

    // Save profile without rename (original_name is None)
    let profile = ProfileConfig {
        base_url: Some("https://updated.example.com:8089".to_string()),
        username: Some("admin".to_string()),
        password: Some(splunk_config::SecureValue::Plain(
            secrecy::SecretString::new("newpassword".to_string().into()),
        )),
        api_token: None,
        skip_verify: Some(true),
        timeout_seconds: Some(30),
        max_retries: Some(3),
        session_expiry_buffer_seconds: Some(60),
        session_ttl_seconds: Some(3600),
        health_check_interval_seconds: Some(60),
    };

    let actions = harness
        .handle_and_collect(
            Action::SaveProfile {
                name: "test-profile".to_string(),
                profile,
                use_keyring: false,
                original_name: None,
            },
            1,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::ProfileSaved(Ok(name)) if name == "test-profile")),
        "Should send ProfileSaved success"
    );

    // Verify profile was updated
    {
        let cm = harness.config_manager.lock().await;
        let profiles = cm.list_profiles();
        assert_eq!(profiles.len(), 1);
        assert!(profiles.contains_key("test-profile"));
        assert_eq!(
            profiles["test-profile"].base_url,
            Some("https://updated.example.com:8089".to_string())
        );
    }
}

#[tokio::test]
async fn test_save_profile_with_rename() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Create initial profile
    {
        let mut cm = harness.config_manager.lock().await;
        let profile = ProfileConfig {
            base_url: Some("https://old.example.com:8089".to_string()),
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
        cm.save_profile("old-profile", profile)
            .expect("Failed to save profile");
    }

    // Save profile with rename (original_name is Some)
    let profile = ProfileConfig {
        base_url: Some("https://new.example.com:8089".to_string()),
        username: Some("admin".to_string()),
        password: Some(splunk_config::SecureValue::Plain(
            secrecy::SecretString::new("newpassword".to_string().into()),
        )),
        api_token: None,
        skip_verify: Some(true),
        timeout_seconds: Some(30),
        max_retries: Some(3),
        session_expiry_buffer_seconds: Some(60),
        session_ttl_seconds: Some(3600),
        health_check_interval_seconds: Some(60),
    };

    let actions = harness
        .handle_and_collect(
            Action::SaveProfile {
                name: "new-profile".to_string(),
                profile,
                use_keyring: false,
                original_name: Some("old-profile".to_string()),
            },
            1,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::ProfileSaved(Ok(name)) if name == "new-profile")),
        "Should send ProfileSaved success with new name"
    );

    // Verify both old profile deleted and new profile created
    {
        let cm = harness.config_manager.lock().await;
        let profiles = cm.list_profiles();
        assert_eq!(
            profiles.len(),
            1,
            "Should have exactly one profile after rename"
        );
        assert!(
            !profiles.contains_key("old-profile"),
            "Old profile should be deleted"
        );
        assert!(
            profiles.contains_key("new-profile"),
            "New profile should exist"
        );
        assert_eq!(
            profiles["new-profile"].base_url,
            Some("https://new.example.com:8089".to_string())
        );
    }
}

#[tokio::test]
async fn test_save_profile_rename_same_name() {
    let mut harness = SideEffectsTestHarness::new().await;

    // Create initial profile
    {
        let mut cm = harness.config_manager.lock().await;
        let profile = ProfileConfig {
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
        cm.save_profile("same-profile", profile)
            .expect("Failed to save profile");
    }

    // Save profile with rename where original_name equals new name (should not delete)
    let profile = ProfileConfig {
        base_url: Some("https://updated.example.com:8089".to_string()),
        username: Some("admin".to_string()),
        password: Some(splunk_config::SecureValue::Plain(
            secrecy::SecretString::new("newpassword".to_string().into()),
        )),
        api_token: None,
        skip_verify: Some(true),
        timeout_seconds: Some(30),
        max_retries: Some(3),
        session_expiry_buffer_seconds: Some(60),
        session_ttl_seconds: Some(3600),
        health_check_interval_seconds: Some(60),
    };

    let actions = harness
        .handle_and_collect(
            Action::SaveProfile {
                name: "same-profile".to_string(),
                profile,
                use_keyring: false,
                original_name: Some("same-profile".to_string()),
            },
            1,
        )
        .await;

    assert!(
        actions
            .iter()
            .any(|a| matches!(a, Action::ProfileSaved(Ok(name)) if name == "same-profile")),
        "Should send ProfileSaved success"
    );

    // Verify profile still exists and was updated
    {
        let cm = harness.config_manager.lock().await;
        let profiles = cm.list_profiles();
        assert_eq!(profiles.len(), 1, "Should have exactly one profile");
        assert!(
            profiles.contains_key("same-profile"),
            "Profile should still exist"
        );
        assert_eq!(
            profiles["same-profile"].base_url,
            Some("https://updated.example.com:8089".to_string())
        );
    }
}

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
