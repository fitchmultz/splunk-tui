//! Multi-profile configuration tests for `splunk-cli list-all`.
//!
//! Tests verify multi-profile functionality including --profiles, --all-profiles,
//! profile validation, credential resolution, and CLI config precedence.

use crate::common::splunk_cmd;
use predicates::prelude::*;

/// Test that `--profiles` and `--all-profiles` conflict.
#[test]
fn test_list_all_profiles_conflict() {
    let mut cmd = splunk_cmd();
    cmd.args(["list-all", "--profiles", "dev", "--all-profiles"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

/// Test multi-profile JSON output structure.
#[test]
fn test_list_all_multi_profile_json() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create a config file with multiple profiles
    let config = serde_json::json!({
        "profiles": {
            "dev": {
                "base_url": "https://dev.splunk.local:8089",
                "username": "admin",
                "password": "devpass"
            },
            "prod": {
                "base_url": "https://prod.splunk.com:8089",
                "api_token": "prod-token"
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
        .args([
            "list-all",
            "--all-profiles",
            "--output",
            "json",
            "--resources",
            "health",
        ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("\"profiles\""))
        .stdout(predicate::str::contains("\"timestamp\""))
        .stdout(predicate::str::contains("\"profile_name\""))
        .stdout(predicate::str::contains("\"base_url\""));
}

/// Test that `--profiles` with non-existent profile fails fast.
#[test]
fn test_list_all_invalid_profile() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create a config file with one profile
    let config = serde_json::json!({
        "profiles": {
            "dev": {
                "base_url": "https://dev.splunk.local:8089",
                "username": "admin"
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
        .args(["list-all", "--profiles", "nonexistent"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("Profile 'nonexistent' not found"));
}

/// Test that `--all-profiles` with no profiles configured fails.
#[test]
fn test_list_all_all_profiles_no_profiles() {
    // Use a unique temp directory to avoid interference from parallel tests
    let temp_dir = tempfile::Builder::new()
        .prefix("splunk-cli-test-empty-")
        .tempdir()
        .unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create an empty config file with no profiles
    let config = serde_json::json!({
        "profiles": {}
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
        .args(["list-all", "--all-profiles"]);

    cmd.assert()
        .failure()
        .stderr(predicate::str::contains("No profiles configured"));
}

/// Test multi-profile CSV output.
#[test]
fn test_list_all_multi_profile_csv() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    let config = serde_json::json!({
        "profiles": {
            "dev": {
                "base_url": "https://dev.splunk.local:8089",
                "username": "admin"
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
        .args([
            "list-all",
            "--all-profiles",
            "--output",
            "csv",
            "--resources",
            "health",
        ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("profile_name,base_url"));
}

/// Test multi-profile XML output.
#[test]
fn test_list_all_multi_profile_xml() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    let config = serde_json::json!({
        "profiles": {
            "dev": {
                "base_url": "https://dev.splunk.local:8089",
                "username": "admin"
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
        .args([
            "list-all",
            "--all-profiles",
            "--output",
            "xml",
            "--resources",
            "health",
        ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("list_all_multi"))
        .stdout(predicate::str::contains("profile"));
}

/// Test that profile with missing credentials (no username/password/api_token) reports error.
#[test]
fn test_list_all_profile_missing_credentials() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create config with profile that has base_url but no credentials
    let config = serde_json::json!({
        "profiles": {
            "no-creds": {
                "base_url": "https://dev.splunk.local:8089"
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
        .args([
            "list-all",
            "--profiles",
            "no-creds",
            "--output",
            "json",
            "--resources",
            "health",
        ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("no-creds"))
        .stdout(predicate::str::contains(
            "No credentials configured (expected api_token or username/password)",
        ));
}

/// Test that profile with only username (no password) reports error.
#[test]
fn test_list_all_profile_missing_password() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create config with profile that has username but no password
    let config = serde_json::json!({
        "profiles": {
            "no-password": {
                "base_url": "https://dev.splunk.local:8089",
                "username": "admin"
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
        .args([
            "list-all",
            "--profiles",
            "no-password",
            "--output",
            "json",
            "--resources",
            "health",
        ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("no-password"))
        .stdout(predicate::str::contains(
            "Username configured but password is missing",
        ));
}

/// Test that profile with only password (no username) reports error.
#[test]
fn test_list_all_profile_missing_username() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create config with profile that has password but no username
    let config = serde_json::json!({
        "profiles": {
            "no-username": {
                "base_url": "https://dev.splunk.local:8089",
                "password": "secret123"
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
        .args([
            "list-all",
            "--profiles",
            "no-username",
            "--output",
            "json",
            "--resources",
            "health",
        ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("no-username"))
        .stdout(predicate::str::contains(
            "Password configured but username is missing",
        ));
}

/// Test that profile with keyring-based password resolution failure reports error.
#[test]
fn test_list_all_profile_keyring_resolution_failure() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create config with profile that references non-existent keyring entry
    let config = serde_json::json!({
        "profiles": {
            "keyring-fail": {
                "base_url": "https://dev.splunk.local:8089",
                "username": "admin",
                "password": {
                    "keyring_account": "nonexistent_account_12345"
                }
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
        .args([
            "list-all",
            "--profiles",
            "keyring-fail",
            "--output",
            "json",
            "--resources",
            "health",
        ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("keyring-fail"))
        .stdout(predicate::str::contains(
            "Failed to resolve password from keyring",
        ));
}

/// Test that profile with keyring-based API token resolution failure reports error.
#[test]
fn test_list_all_profile_api_token_resolution_failure() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create config with profile that references non-existent keyring entry for API token
    let config = serde_json::json!({
        "profiles": {
            "token-fail": {
                "base_url": "https://dev.splunk.local:8089",
                "api_token": {
                    "keyring_account": "nonexistent_token_account_12345"
                }
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap())
        .args([
            "list-all",
            "--profiles",
            "token-fail",
            "--output",
            "json",
            "--resources",
            "health",
        ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("token-fail"))
        .stdout(predicate::str::contains(
            "Failed to resolve API token from keyring",
        ));
}

/// Test that `--config-path` CLI flag takes precedence over SPLUNK_CONFIG_PATH env var.
#[test]
fn test_list_all_config_path_cli_precedence() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let cli_config_path = temp_dir.path().join("cli_config.json");
    let env_config_path = temp_dir.path().join("env_config.json");

    // Create CLI config with "cli-profile"
    let cli_config = serde_json::json!({
        "profiles": {
            "cli-profile": {
                "base_url": "https://cli.splunk.local:8089",
                "username": "admin"
            }
        }
    });
    std::fs::write(&cli_config_path, cli_config.to_string()).unwrap();

    // Create env config with "env-profile"
    let env_config = serde_json::json!({
        "profiles": {
            "env-profile": {
                "base_url": "https://env.splunk.local:8089",
                "username": "admin"
            }
        }
    });
    std::fs::write(&env_config_path, env_config.to_string()).unwrap();

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_CONFIG_PATH", env_config_path.to_str().unwrap())
        .args([
            "list-all",
            "--config-path",
            cli_config_path.to_str().unwrap(),
            "--all-profiles",
            "--output",
            "json",
            "--resources",
            "health",
        ]);

    // Should use CLI config, not env config
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("cli-profile"))
        .stdout(predicate::str::contains("https://cli.splunk.local:8089"))
        .stdout(predicate::str::contains("env-profile").not());
}

/// Test that multi-profile mode ignores connection environment variables.
///
/// This test verifies that when running in multi-profile mode (--all-profiles or --profiles),
/// the command does NOT require or use SPLUNK_BASE_URL or other connection env vars.
/// If the refactor accidentally routes multi-profile through single-profile path,
/// this test would fail because the invalid SPLUNK_BASE_URL would cause config validation to fail.
#[test]
fn test_list_all_multi_profile_ignores_connection_env_vars() {
    let temp_dir = tempfile::TempDir::new().unwrap();
    let config_path = temp_dir.path().join("config.json");

    // Create a config file with a valid-looking profile
    let config = serde_json::json!({
        "profiles": {
            "test-profile": {
                "base_url": "https://test.splunk.local:8089",
                "username": "admin"
            }
        }
    });
    std::fs::write(&config_path, config.to_string()).unwrap();

    let mut cmd = crate::common::splunk_cmd();

    // Set the valid config path
    cmd.env("SPLUNK_CONFIG_PATH", config_path.to_str().unwrap());

    // Set SPLUNK_BASE_URL to an intentionally invalid value
    // In multi-profile mode, this should be ignored and the command should succeed
    cmd.env("SPLUNK_BASE_URL", "not-a-valid-url");

    // Also clear the default API token to ensure we're not accidentally
    // falling back to single-profile mode
    cmd.env_remove("SPLUNK_API_TOKEN");

    cmd.args([
        "list-all",
        "--all-profiles",
        "--output",
        "json",
        "--resources",
        "health",
    ]);

    // Command should succeed despite invalid SPLUNK_BASE_URL
    // because multi-profile mode doesn't use/validate it
    cmd.assert()
        .success()
        // Verify we got multi-profile output structure
        .stdout(predicate::str::contains("\"profiles\""))
        .stdout(predicate::str::contains("\"timestamp\""))
        // Verify the profile from config file was used
        .stdout(predicate::str::contains("test-profile"))
        .stdout(predicate::str::contains("https://test.splunk.local:8089"));
}
