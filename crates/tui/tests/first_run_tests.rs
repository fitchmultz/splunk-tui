//! First-run detection integration tests
//!
//! Tests for verifying the first-run detection logic that triggers
//! the tutorial on initial startup.
//!
//! Note: Uses the shared `should_launch_tutorial` helper from the startup module
//! to ensure consistency with main.rs startup logic (RQ-0454).

use splunk_config::{ConfigManager, PersistedState};
use splunk_tui::runtime::startup::should_launch_tutorial;
use std::io::Write;
use tempfile::NamedTempFile;

/// Local alias for the shared helper to maintain test readability
fn is_first_run(profiles_empty: bool, skip_tutorial_flag: bool, tutorial_completed: bool) -> bool {
    should_launch_tutorial(profiles_empty, skip_tutorial_flag, tutorial_completed)
}

#[test]
fn test_first_run_detection_no_profiles() {
    temp_env::with_var("SPLUNK_CONFIG_NO_MIGRATE", Some("1"), || {
        // Create a temporary config file
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        // Create config manager (will create empty config since file is empty)
        let config_manager = ConfigManager::new_with_path(config_path).unwrap();

        // Check first-run conditions
        let profiles = config_manager.list_profiles();
        let persisted_state = config_manager.load();
        let is_first = is_first_run(
            profiles.is_empty(),
            false,
            persisted_state.tutorial_completed,
        );

        assert!(is_first, "Should be first run when no profiles exist");
    });
}

#[test]
fn test_first_run_detection_tutorial_completed() {
    temp_env::with_var("SPLUNK_CONFIG_NO_MIGRATE", Some("1"), || {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        // Create config with tutorial_completed = true
        let mut config_manager = ConfigManager::new_with_path(config_path.clone()).unwrap();
        let mut persisted_state = config_manager.load();
        persisted_state.tutorial_completed = true;
        config_manager.save(&persisted_state).unwrap();

        // Reload and check
        let config_manager = ConfigManager::new_with_path(config_path).unwrap();
        let persisted_state = config_manager.load();
        let profiles = config_manager.list_profiles();
        let is_first = is_first_run(
            profiles.is_empty(),
            false,
            persisted_state.tutorial_completed,
        );

        assert!(
            !is_first,
            "Should NOT be first run when tutorial is completed"
        );
    });
}

#[test]
fn test_first_run_detection_with_skip_flag() {
    temp_env::with_var("SPLUNK_CONFIG_NO_MIGRATE", Some("1"), || {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        // Create config manager with empty profiles
        let config_manager = ConfigManager::new_with_path(config_path).unwrap();

        // Check with skip_tutorial flag set
        let profiles = config_manager.list_profiles();
        let persisted_state = config_manager.load();
        let is_first = is_first_run(
            profiles.is_empty(),
            true,
            persisted_state.tutorial_completed,
        );

        assert!(
            !is_first,
            "Should NOT be first run when --skip-tutorial flag is set"
        );
    });
}

#[test]
fn test_first_run_detection_with_profiles() {
    temp_env::with_var("SPLUNK_CONFIG_NO_MIGRATE", Some("1"), || {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        // Create config manager and add a profile
        let mut config_manager = ConfigManager::new_with_path(config_path.clone()).unwrap();
        let profile = splunk_config::types::ProfileConfig {
            base_url: Some("https://localhost:8089".to_string()),
            username: Some("admin".to_string()),
            password: Some(splunk_config::types::SecureValue::Plain(
                secrecy::SecretString::from("changeme"),
            )),
            api_token: None,
            skip_verify: Some(false),
            timeout_seconds: Some(30),
            max_retries: Some(3),
            health_check_interval_seconds: None,
            session_expiry_buffer_seconds: None,
            session_ttl_seconds: None,
        };
        config_manager.save_profile("default", profile).unwrap();

        // Reload and check
        let config_manager = ConfigManager::new_with_path(config_path).unwrap();
        let persisted_state = config_manager.load();
        let profiles = config_manager.list_profiles();
        let is_first = is_first_run(
            profiles.is_empty(),
            false,
            persisted_state.tutorial_completed,
        );

        assert!(!is_first, "Should NOT be first run when profiles exist");
    });
}

#[test]
fn test_first_run_default_tutorial_completed_is_false() {
    // Verify that PersistedState defaults to tutorial_completed = false
    let state = PersistedState::default();
    assert!(
        !state.tutorial_completed,
        "tutorial_completed should default to false"
    );
}

#[test]
fn test_persisted_state_roundtrip() {
    temp_env::with_var("SPLUNK_CONFIG_NO_MIGRATE", Some("1"), || {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        // Create and save state with tutorial_completed = true
        let mut config_manager = ConfigManager::new_with_path(config_path.clone()).unwrap();
        let mut persisted_state = config_manager.load();
        persisted_state.tutorial_completed = true;
        config_manager.save(&persisted_state).unwrap();

        // Reload and verify
        let config_manager = ConfigManager::new_with_path(config_path).unwrap();
        let reloaded_state = config_manager.load();

        assert!(
            reloaded_state.tutorial_completed,
            "tutorial_completed should persist across save/load"
        );
    });
}

#[test]
fn test_config_file_with_existing_profiles_and_tutorial_completed() {
    temp_env::with_var("SPLUNK_CONFIG_NO_MIGRATE", Some("1"), || {
        let mut temp_file = NamedTempFile::new().unwrap();

        // Create a config file with existing profiles and tutorial_completed = true
        let config_json = r#"{
        "profiles": {
            "production": {
                "base_url": "https://splunk.prod:8089",
                "username": "admin",
                "skip_verify": false,
                "timeout_seconds": 30,
                "max_retries": 3
            }
        },
        "state": {
            "auto_refresh": false,
            "sort_column": "sid",
            "sort_direction": "asc",
            "tutorial_completed": true
        }
    }"#;

        temp_file.write_all(config_json.as_bytes()).unwrap();
        temp_file.flush().unwrap();

        let config_manager = ConfigManager::new_with_path(temp_file.path().to_path_buf()).unwrap();
        let profiles = config_manager.list_profiles();
        let persisted_state = config_manager.load();

        assert_eq!(profiles.len(), 1);
        assert!(persisted_state.tutorial_completed);

        let is_first = is_first_run(
            profiles.is_empty(),
            false,
            persisted_state.tutorial_completed,
        );
        assert!(!is_first);
    });
}

#[test]
fn test_first_run_opens_tutorial_popup() {
    temp_env::with_var("SPLUNK_CONFIG_NO_MIGRATE", Some("1"), || {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        // Create config with NO profiles and tutorial not completed
        let config_manager = ConfigManager::new_with_path(config_path).unwrap();
        let persisted_state = config_manager.load();

        // Verify conditions for first-run
        assert!(
            config_manager.list_profiles().is_empty(),
            "Should have no profiles"
        );
        assert!(
            !persisted_state.tutorial_completed,
            "Tutorial should not be completed"
        );

        // Verify should_launch_tutorial returns true
        let should_launch = should_launch_tutorial(
            config_manager.list_profiles().is_empty(),
            false, // skip_tutorial_flag
            persisted_state.tutorial_completed,
        );

        assert!(
            should_launch,
            "should_launch_tutorial should return true for first-run conditions"
        );

        // Note: The actual popup opening is handled in main.rs:411-420.
        // This test verifies the logic condition; full integration would require
        // testing App initialization which is covered by bootstrap_startup_tests.rs
    });
}

#[test]
fn test_first_run_respects_skip_tutorial_flag() {
    temp_env::with_var("SPLUNK_CONFIG_NO_MIGRATE", Some("1"), || {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        let config_manager = ConfigManager::new_with_path(config_path).unwrap();
        let persisted_state = config_manager.load();

        // Even with empty profiles, skip_tutorial_flag should prevent launch
        let should_launch = should_launch_tutorial(
            config_manager.list_profiles().is_empty(),
            true, // skip_tutorial_flag = true
            persisted_state.tutorial_completed,
        );

        assert!(
            !should_launch,
            "should_launch_tutorial should return false when skip_tutorial_flag is true"
        );
    });
}

#[test]
fn test_first_run_respects_completed_tutorial() {
    temp_env::with_var("SPLUNK_CONFIG_NO_MIGRATE", Some("1"), || {
        let temp_file = NamedTempFile::new().unwrap();
        let config_path = temp_file.path().to_path_buf();

        // Create config with tutorial completed but no profiles
        let mut config_manager = ConfigManager::new_with_path(config_path.clone()).unwrap();
        let mut persisted_state = config_manager.load();
        persisted_state.tutorial_completed = true;
        config_manager.save(&persisted_state).unwrap();

        let config_manager = ConfigManager::new_with_path(config_path).unwrap();
        let persisted_state = config_manager.load();

        let should_launch = should_launch_tutorial(
            config_manager.list_profiles().is_empty(),
            false,
            persisted_state.tutorial_completed,
        );

        assert!(
            !should_launch,
            "should_launch_tutorial should return false when tutorial already completed"
        );
    });
}
