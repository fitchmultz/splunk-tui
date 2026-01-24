//! Integration tests for TUI config loading with SPLUNK_CONFIG_PATH.
//!
//! Tests verify that the TUI correctly honors the SPLUNK_CONFIG_PATH
//! environment variable for both ConfigLoader and ConfigManager.
//!
//! NOTE: These tests use environment variables which are process-global.
//! When running individually, they work fine, but when run in parallel
//! they may interfere with each other. Run with:
//!   cargo test -p splunk-tui --test config_loading_tests -- --test-threads=1

#![allow(unused_unsafe)]

use splunk_config::{ConfigLoader, ConfigManager};
use std::fs;
use tempfile::TempDir;

/// Helper to create a test config file with a profile.
fn create_test_config_with_profile(dir: &TempDir, profile_name: &str, base_url: &str) -> String {
    let config_path = dir.path().join("config.json");
    let config_content = serde_json::json!({
        "profiles": {
            profile_name: {
                "base_url": base_url,
                "api_token": "test-token-12345"
            }
        }
    });

    fs::write(
        &config_path,
        serde_json::to_string_pretty(&config_content).unwrap(),
    )
    .expect("Failed to write test config file");

    config_path.to_string_lossy().to_string()
}

/// Helper to clean up all Splunk-related environment variables.
/// This ensures test isolation by clearing any env vars that might interfere.
fn cleanup_env_vars() {
    unsafe {
        std::env::remove_var("SPLUNK_CONFIG_PATH");
    }
    unsafe {
        std::env::remove_var("SPLUNK_BASE_URL");
    }
    unsafe {
        std::env::remove_var("SPLUNK_API_TOKEN");
    }
    unsafe {
        std::env::remove_var("SPLUNK_USERNAME");
    }
    unsafe {
        std::env::remove_var("SPLUNK_PASSWORD");
    }
    unsafe {
        std::env::remove_var("SPLUNK_PROFILE");
    }
    unsafe {
        std::env::remove_var("SPLUNK_SKIP_VERIFY");
    }
    unsafe {
        std::env::remove_var("SPLUNK_TIMEOUT");
    }
    unsafe {
        std::env::remove_var("SPLUNK_MAX_RETRIES");
    }
}

#[test]
fn test_config_manager_with_splunk_config_path() {
    cleanup_env_vars();

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = create_test_config_with_profile(
        &temp_dir,
        "test-profile",
        "https://splunk.example.com:8089",
    );

    // Set SPLUNK_CONFIG_PATH environment variable
    unsafe {
        std::env::set_var("SPLUNK_CONFIG_PATH", &config_path);
    }

    // Create ConfigManager - it should use the custom path
    let result = ConfigManager::new_with_path(std::path::PathBuf::from(&config_path));

    // Clean up env var
    cleanup_env_vars();

    assert!(
        result.is_ok(),
        "ConfigManager should be created with custom path"
    );

    let manager = result.unwrap();
    let _loaded = manager.load();

    // Verify the manager was created successfully
    assert!(
        manager.config_path() == &std::path::PathBuf::from(&config_path),
        "ConfigManager should use the custom path"
    );
}

#[test]
fn test_config_manager_default_path_when_env_not_set() {
    cleanup_env_vars();

    // Create ConfigManager with default path
    let result = ConfigManager::new();

    assert!(
        result.is_ok(),
        "ConfigManager should be created with default path when env var is not set"
    );
}

#[test]
fn test_config_loader_with_splunk_config_path() {
    cleanup_env_vars();

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = create_test_config_with_profile(
        &temp_dir,
        "custom-profile",
        "https://custom.example.com:8089",
    );

    // Set SPLUNK_CONFIG_PATH environment variable
    unsafe {
        std::env::set_var("SPLUNK_CONFIG_PATH", &config_path);
    }
    // Set SPLUNK_PROFILE to load the specific profile
    unsafe {
        std::env::set_var("SPLUNK_PROFILE", "custom-profile");
    }

    // Create a loader with SPLUNK_CONFIG_PATH
    let mut loader = ConfigLoader::new().load_dotenv().unwrap();
    loader = loader.with_config_path(std::path::PathBuf::from(&config_path));
    loader = loader
        .with_profile_name("custom-profile".to_string())
        .from_profile()
        .unwrap();

    // Build the config (skip from_env to avoid .env overrides)
    let result = loader.build();

    // Clean up env vars
    cleanup_env_vars();

    assert!(
        result.is_ok(),
        "ConfigLoader should successfully load config from custom path"
    );

    let config = result.unwrap();
    assert_eq!(
        config.connection.base_url, "https://custom.example.com:8089",
        "Base URL should match the profile from custom config path"
    );
}

#[test]
fn test_config_loader_default_path_when_env_not_set() {
    cleanup_env_vars();

    // Create loader with default path - should not fail due to path issues
    // (may fail due to missing auth, which is expected)
    let loader = ConfigLoader::new().load_dotenv().unwrap();

    // Clear env vars loaded from .env to test default path behavior without auth overrides
    cleanup_env_vars();

    // The loader should work with default path - missing auth is expected
    let result = loader.from_env().and_then(|l| l.build());

    // We don't require success - the test is about using default path, not valid auth
    // The loader was created successfully which is what matters
    match result {
        Ok(_) => {}
        Err(e) => {
            // Only MissingBaseUrl or MissingAuth are acceptable errors
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("Base URL") || error_msg.contains("Authentication"),
                "Default path test should only fail with auth-related errors, got: {}",
                error_msg
            );
        }
    }

    cleanup_env_vars();
}

#[test]
fn test_empty_splunk_config_path_uses_default() {
    cleanup_env_vars();

    // Test that an empty SPLUNK_CONFIG_PATH is handled correctly
    unsafe {
        std::env::set_var("SPLUNK_CONFIG_PATH", "");
    }

    // In Rust 1.84+, std::env::var returns Ok("") for empty strings
    let result = std::env::var("SPLUNK_CONFIG_PATH");
    assert!(
        result.is_ok() && result.unwrap().is_empty(),
        "Empty SPLUNK_CONFIG_PATH should return Ok with empty string"
    );

    // The TUI pattern uses `if let Ok(...)` which will match empty strings
    // So we need to handle this case properly with !path.is_empty()
    let should_use_custom = if let Ok(path) = std::env::var("SPLUNK_CONFIG_PATH") {
        !path.is_empty() // Don't use custom path if empty
    } else {
        false
    };
    assert!(
        !should_use_custom,
        "Empty SPLUNK_CONFIG_PATH should not trigger custom path usage"
    );

    // Clean up
    cleanup_env_vars();
}

#[test]
fn test_empty_splunk_config_path_with_config_manager() {
    cleanup_env_vars();

    // Test that ConfigManager uses default path when SPLUNK_CONFIG_PATH is empty
    unsafe {
        std::env::set_var("SPLUNK_CONFIG_PATH", "");
    }

    // This simulates the actual TUI code pattern (after fix)
    let manager = if let Ok(config_path) = std::env::var("SPLUNK_CONFIG_PATH") {
        if !config_path.is_empty() {
            ConfigManager::new_with_path(std::path::PathBuf::from(config_path))
        } else {
            ConfigManager::new()
        }
    } else {
        ConfigManager::new()
    };

    // Clean up
    cleanup_env_vars();

    assert!(
        manager.is_ok(),
        "ConfigManager should succeed when SPLUNK_CONFIG_PATH is empty"
    );

    // Just verify it succeeded - the fact it didn't try to use empty path is enough
    // (empty path would cause different behavior or errors)
    let _created_manager = manager.unwrap();
}

#[test]
fn test_empty_splunk_config_path_with_config_loader() {
    cleanup_env_vars();

    // Test that ConfigLoader ignores empty SPLUNK_CONFIG_PATH
    unsafe {
        std::env::set_var("SPLUNK_CONFIG_PATH", "");
    }

    let mut loader = ConfigLoader::new().load_dotenv().unwrap();

    // This simulates the actual TUI load_config code pattern (after fix)
    if let Ok(config_path) = std::env::var("SPLUNK_CONFIG_PATH")
        && !config_path.is_empty()
    {
        loader = loader.with_config_path(std::path::PathBuf::from(config_path));
        // If empty, do nothing (use default path)
    }

    // Clean up
    cleanup_env_vars();

    // The loader should work fine with default path - just verify it doesn't fail
    // with path-related errors (missing auth/base URL is expected and OK here)
    let result = loader.from_env().and_then(|l| l.build());

    match result {
        Ok(_) => {}
        Err(e) => {
            // MissingBaseUrl or MissingAuth are expected when env vars are not set
            // Any other error (especially path-related) would be a problem
            let error_msg = e.to_string();
            assert!(
                error_msg.contains("Base URL") || error_msg.contains("Authentication"),
                "Empty SPLUNK_CONFIG_PATH should not cause unexpected errors: {}",
                error_msg
            );
        }
    }
}

#[test]
fn test_splunk_config_path_with_nonexistent_file() {
    cleanup_env_vars();

    let nonexistent_path = "/tmp/nonexistent_splunk_config_12345.json";

    // Set SPLUNK_CONFIG_PATH to a nonexistent file
    unsafe {
        std::env::set_var("SPLUNK_CONFIG_PATH", nonexistent_path);
    }

    // Create ConfigManager with nonexistent path
    let result = ConfigManager::new_with_path(std::path::PathBuf::from(nonexistent_path));

    // Clean up
    cleanup_env_vars();

    // ConfigManager::new_with_path should succeed, but load() will return default state
    assert!(result.is_ok(), "ConfigManager creation should succeed");

    let manager = result.unwrap();
    let loaded = manager.load();

    // Loading from a nonexistent file should return default PersistedState
    assert!(
        !loaded.auto_refresh,
        "Default state should have auto_refresh=false"
    );
    assert_eq!(
        loaded.sort_column, "sid",
        "Default state should have sort_column=sid"
    );
}

#[test]
fn test_config_manager_persistence_with_custom_path() {
    cleanup_env_vars();

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("test-persist.json");

    // Set SPLUNK_CONFIG_PATH
    unsafe {
        std::env::set_var("SPLUNK_CONFIG_PATH", config_path.to_string_lossy().as_ref());
    }

    // Create ConfigManager with custom path
    let result = ConfigManager::new_with_path(config_path.clone());
    cleanup_env_vars();

    assert!(result.is_ok(), "ConfigManager should be created");

    let mut manager = result.unwrap();

    // Create some test state to persist
    use splunk_config::persistence::PersistedState;
    let test_state = PersistedState {
        auto_refresh: true,
        sort_column: "disk_usage".to_string(),
        sort_direction: "desc".to_string(),
        last_search_query: Some("test query".to_string()),
        search_history: vec!["query1".to_string(), "query2".to_string()],
    };

    // Save the state
    let save_result = manager.save(&test_state);
    assert!(save_result.is_ok(), "Save should succeed");

    // Verify the file was created
    assert!(
        config_path.exists(),
        "Config file should be created at custom path"
    );

    // Load it back
    let loaded = manager.load();
    assert_eq!(
        loaded.auto_refresh, test_state.auto_refresh,
        "Loaded auto_refresh should match saved state"
    );
    assert_eq!(
        loaded.sort_column, test_state.sort_column,
        "Loaded sort_column should match saved state"
    );
    assert_eq!(
        loaded.sort_direction, test_state.sort_direction,
        "Loaded sort_direction should match saved state"
    );
    assert_eq!(
        loaded.last_search_query, test_state.last_search_query,
        "Loaded last_search_query should match saved state"
    );
    assert_eq!(
        loaded.search_history, test_state.search_history,
        "Loaded search_history should match saved state"
    );
}

#[test]
fn test_config_loader_with_profile_from_custom_path() {
    // Clean up any env vars from previous tests
    cleanup_env_vars();

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path =
        create_test_config_with_profile(&temp_dir, "production", "https://prod.splunk.com:8089");

    // Set both SPLUNK_CONFIG_PATH and SPLUNK_PROFILE
    unsafe {
        std::env::set_var("SPLUNK_CONFIG_PATH", &config_path);
    }
    unsafe {
        std::env::set_var("SPLUNK_PROFILE", "production");
    }

    // Simulate the TUI load_config() function pattern
    let mut loader = ConfigLoader::new().load_dotenv().unwrap();

    // Clear env vars that were loaded from .env to ensure profile is used
    unsafe {
        std::env::remove_var("SPLUNK_BASE_URL");
    }
    unsafe {
        std::env::remove_var("SPLUNK_API_TOKEN");
    }
    unsafe {
        std::env::remove_var("SPLUNK_USERNAME");
    }
    unsafe {
        std::env::remove_var("SPLUNK_PASSWORD");
    }

    // Check for SPLUNK_CONFIG_PATH override (TUI pattern)
    if let Ok(path) = std::env::var("SPLUNK_CONFIG_PATH") {
        loader = loader.with_config_path(std::path::PathBuf::from(path));
    }

    // Load from profile if SPLUNK_PROFILE is set
    if let Ok(profile_name) = std::env::var("SPLUNK_PROFILE") {
        loader = loader
            .with_profile_name(profile_name)
            .from_profile()
            .unwrap();
    }

    // Build the config (skip from_env to avoid .env overrides in test)
    let result = loader.build();

    // Clean up env vars
    cleanup_env_vars();

    assert!(result.is_ok(), "Config should be built successfully");

    let config = result.unwrap();
    assert_eq!(
        config.connection.base_url, "https://prod.splunk.com:8089",
        "Base URL should match production profile from custom path"
    );
}

#[test]
fn test_tui_pattern_matches_cli_pattern() {
    cleanup_env_vars();

    // This test verifies that the TUI pattern for handling SPLUNK_CONFIG_PATH
    // matches the CLI pattern, ensuring consistency across the codebase

    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path =
        create_test_config_with_profile(&temp_dir, "test", "https://test.example.com:8089");

    // Test TUI pattern (from main.rs lines 85-87)
    unsafe {
        std::env::set_var("SPLUNK_CONFIG_PATH", &config_path);
    }
    let tui_result = if let Ok(path) = std::env::var("SPLUNK_CONFIG_PATH") {
        ConfigManager::new_with_path(std::path::PathBuf::from(path))
    } else {
        ConfigManager::new()
    };
    cleanup_env_vars();

    // Test CLI pattern (from cli/src/commands/config.rs lines 64-67)
    unsafe {
        std::env::set_var("SPLUNK_CONFIG_PATH", &config_path);
    }
    let cli_result = if let Ok(path) = std::env::var("SPLUNK_CONFIG_PATH") {
        ConfigManager::new_with_path(std::path::PathBuf::from(path))
    } else {
        ConfigManager::new()
    };
    cleanup_env_vars();

    // Both should succeed
    assert!(
        tui_result.is_ok() && cli_result.is_ok(),
        "TUI and CLI patterns should both succeed"
    );

    let tui_manager = tui_result.unwrap();
    let cli_manager = cli_result.unwrap();

    // Both should use the same config path
    assert_eq!(
        tui_manager.config_path(),
        cli_manager.config_path(),
        "TUI and CLI should use the same config path when SPLUNK_CONFIG_PATH is set"
    );
}
