//! Show command tests for `splunk-cli config`.
//!
//! Tests the `config show` subcommand with various output formats.

use crate::common::splunk_cmd;
use crate::config::setup_temp_config;
use predicates::prelude::*;

/// Test that `splunk-cli config show --help` shows options.
#[test]
fn test_config_show_help() {
    splunk_cmd()
        .args(["config", "show", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Show a profile's configuration"));
}

/// Test that `splunk-cli config edit --help` shows options.
#[test]
fn test_config_edit_help() {
    splunk_cmd()
        .args(["config", "edit", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Edit a profile interactively"));
}

/// Test showing a profile that doesn't exist.
#[test]
fn test_config_show_nonexistent() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", config_path)
        .args(["config", "show", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"));
}

/// Test showing a profile with table format (default).
#[test]
fn test_config_show_table_format() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Create a profile
    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--username",
            "admin",
            "--password",
            "testpass",
            "--skip-verify",
            "true",
            "--timeout",
            "60",
            "--max-retries",
            "5",
        ])
        .assert()
        .success();

    // Show profile
    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .args(["config", "show", "test-profile"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Profile Name:"))
        .stdout(predicate::str::contains("test-profile"))
        .stdout(predicate::str::contains("Base URL:"))
        .stdout(predicate::str::contains("https://splunk.example.com:8089"))
        .stdout(predicate::str::contains("Username:"))
        .stdout(predicate::str::contains("admin"));
}

/// Test showing a profile with JSON format.
#[test]
fn test_config_show_json_format() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Create a profile
    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--username",
            "admin",
            "--password",
            "testpass",
        ])
        .assert()
        .success();

    // Show profile in JSON format
    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .args(["-o", "json", "config", "show", "test-profile"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"name\""))
        .stdout(predicate::str::contains("test-profile"))
        .stdout(predicate::str::contains("\"base_url\""))
        .stdout(predicate::str::contains("https://splunk.example.com:8089"));
}

/// Test showing a profile with CSV format.
#[test]
fn test_config_show_csv_format() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Create a profile
    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--username",
            "admin",
            "--password",
            "testpass",
        ])
        .assert()
        .success();

    // Show profile in CSV format
    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .args(["-o", "csv", "config", "show", "test-profile"])
        .assert()
        .success()
        .stdout(predicate::str::contains("field,value"))
        .stdout(predicate::str::contains("Profile Name"))
        .stdout(predicate::str::contains("test-profile"));
}

/// Test showing a profile with XML format.
#[test]
fn test_config_show_xml_format() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Create a profile
    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--username",
            "admin",
            "--password",
            "testpass",
        ])
        .assert()
        .success();

    // Show profile in XML format
    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .args(["-o", "xml", "config", "show", "test-profile"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<?xml"))
        .stdout(predicate::str::contains("<profile>"))
        .stdout(predicate::str::contains("<name>test-profile</name>"));
}

/// Test error for invalid output format in show command.
#[test]
fn test_config_show_invalid_output_format() {
    let (_temp_dir, config_path) = setup_temp_config();

    // Create a profile
    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--api-token",
            "test-token",
        ])
        .assert()
        .success();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", config_path)
        .args(["-o", "invalid", "config", "show", "test-profile"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid output format"));
}
