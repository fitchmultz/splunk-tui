//! Integration tests for `splunk-cli list-all` command.
//!
//! Tests cover:
//! - Default behavior (list all resources)
//! - `--resources` flag filtering
//! - Output format variations (json, table, csv, xml)
//! - Error handling when resources fail to fetch

mod common;

use common::splunk_cmd;
use predicates::prelude::*;

/// Test that `splunk-cli list-all --help` shows command.
#[test]
fn test_list_all_help() {
    let mut cmd = splunk_cmd();
    cmd.args(["list-all", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains(
            "List all Splunk resources in unified overview",
        ))
        .stdout(predicate::str::contains("--resources"))
        .stdout(predicate::str::contains(
            "Optional comma-separated list of resource types",
        ));
}

/// Test that `splunk-cli list-all` defaults to listing all resources.
#[test]
fn test_list_all_default_lists_all() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.arg("list-all").assert();

    // Should succeed (handles errors gracefully, returns results with error status)
    result
        .success()
        .stdout(predicate::str::contains("Timestamp"))
        .stdout(predicate::str::contains("indexes"))
        .stdout(predicate::str::contains("jobs"))
        .stdout(predicate::str::contains("users"));
}

/// Test that `splunk-cli list-all --resources indexes,jobs,users` filters resources.
#[test]
fn test_list_all_filtered_resources() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd
        .args(["list-all", "--resources", "indexes,jobs,users"])
        .assert();

    // Should succeed (handles errors gracefully, returns results with error status)
    result
        .success()
        .stdout(predicate::str::contains("indexes"))
        .stdout(predicate::str::contains("jobs"))
        .stdout(predicate::str::contains("users"))
        .stdout(predicate::str::contains("error")); // Connection errors expected
}

/// Test that `splunk-cli list-all --resources` with single resource works.
#[test]
fn test_list_all_single_resource() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["list-all", "--resources", "indexes"]).assert();

    // Should succeed (handles errors gracefully, returns results with error status)
    result
        .success()
        .stdout(predicate::str::contains("indexes"))
        .stdout(predicate::str::contains("error")); // Connection error expected
}

/// Test that `splunk-cli list-all --resources` with invalid resource type shows error.
#[test]
fn test_list_all_invalid_resource_type() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd
        .args(["list-all", "--resources", "invalid_type"])
        .assert();

    // Should fail with validation error (not connection error)
    result
        .failure()
        .stderr(predicate::str::contains("Invalid resource type"))
        .stderr(predicate::str::contains("Valid types:"));
}

/// Test that `splunk-cli list-all --output json` works.
#[test]
fn test_list_all_json_output() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["list-all", "--output", "json"]).assert();

    // Should succeed with JSON format
    result
        .success()
        .stdout(predicate::str::contains("\"timestamp\""))
        .stdout(predicate::str::contains("\"resources\""))
        .stdout(predicate::str::contains("\"resource_type\""));
}

/// Test that `splunk-cli list-all --output table` works.
#[test]
fn test_list_all_table_output() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["list-all", "--output", "table"]).assert();

    // Should succeed with table format
    result
        .success()
        .stdout(predicate::str::contains("Timestamp"))
        .stdout(predicate::str::contains("Resource Type"))
        .stdout(predicate::str::contains("Count"))
        .stdout(predicate::str::contains("Status"))
        .stdout(predicate::str::contains("Error"));
}

/// Test that `splunk-cli list-all --output csv` works.
#[test]
fn test_list_all_csv_output() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["list-all", "--output", "csv"]).assert();

    // Should succeed with CSV format
    result.success().stdout(predicate::str::contains(
        "timestamp,resource_type,count,status,error",
    ));
}

/// Test that `splunk-cli list-all --output xml` works.
#[test]
fn test_list_all_xml_output() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["list-all", "--output", "xml"]).assert();

    // Should succeed with XML format (now uses list_all_multi structure)
    result
        .success()
        .stdout(predicate::str::contains("<?xml version=\"1.0\""))
        .stdout(predicate::str::contains("<list_all_multi>"))
        .stdout(predicate::str::contains("<timestamp>"))
        .stdout(predicate::str::contains("<profiles>"));
}

/// Test that `splunk-cli list-all` handles error gracefully when resources fail to fetch.
/// This test verifies that partial failures don't crash the command.
#[test]
fn test_list_all_error_handling() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.arg("list-all").assert();

    // Should succeed (handles errors gracefully, returns results with error status)
    result.success().stdout(predicate::str::contains("error")); // Errors captured in output
}

/// Test that `splunk-cli list-all --resources` with comma-separated values works.
#[test]
fn test_list_all_comma_separated_resources() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd
        .args([
            "list-all",
            "--resources",
            "indexes,jobs,apps,users,cluster,health,kvstore,license,saved-searches",
        ])
        .assert();

    // Should succeed (handles errors gracefully, returns results with error status)
    result
        .success()
        .stdout(predicate::str::contains("indexes"))
        .stdout(predicate::str::contains("jobs"))
        .stdout(predicate::str::contains("apps"))
        .stdout(predicate::str::contains("users"))
        .stdout(predicate::str::contains("cluster"))
        .stdout(predicate::str::contains("health"))
        .stdout(predicate::str::contains("kvstore"))
        .stdout(predicate::str::contains("license"))
        .stdout(predicate::str::contains("saved-searches"));
}

/// Test that list-all has timeout protection and doesn't hang on slow/unresponsive endpoints.
/// This test verifies that individual resource fetches have timeout protection.
#[test]
fn test_list_all_timeout_protection() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");
    cmd.env("SPLUNK_TIMEOUT", "1");

    let result = cmd.args(["list-all", "--resources", "indexes"]).assert();

    // Should complete quickly (not hang) even with short timeout
    // Result will show error status for indexes (connection error expected, not timeout)
    result.success();
}

/// Test that `splunk-cli list-all --help` shows multi-profile options.
#[test]
fn test_list_all_help_shows_multi_profile_options() {
    let mut cmd = splunk_cmd();
    cmd.args(["list-all", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--profiles"))
        .stdout(predicate::str::contains("--all-profiles"));
}

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

/// Test that single-profile mode still works (backward compatibility).
#[test]
fn test_list_all_single_profile_backward_compat() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089").args([
        "list-all",
        "--resources",
        "health",
    ]);

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Timestamp"));
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
