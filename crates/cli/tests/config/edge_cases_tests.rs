//! Edge case and validation tests for `splunk-cli config`.
//!
//! Tests validation errors, edit command edge cases, and config path handling.

use crate::common::splunk_cmd;
use crate::config::setup_temp_config;
use predicates::prelude::*;
use std::fs;

/// Test error when username is provided without password or token
#[test]
fn test_config_set_username_without_password_error() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path)
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_API_TOKEN")
        .env_remove("SPLUNK_USERNAME")
        .env_remove("SPLUNK_BASE_URL")
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--username",
            "admin",
            // No --password or --api-token
        ])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "Either --password or --api-token must be provided when using username",
        ));
}

/// Test error for invalid output format.
#[test]
fn test_config_set_invalid_output_format() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", config_path)
        .args(["-o", "invalid", "config", "list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Invalid output format"));
}

/// Test edit command error when profile doesn't exist.
#[test]
fn test_config_edit_nonexistent() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", config_path)
        .args(["config", "edit", "nonexistent"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("not found"))
        .stderr(predicate::str::contains("Use 'config set'"));
}

/// Test that --config-path flag works and writes to the specified file.
#[test]
fn test_config_path_flag_works() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--api-token",
            "test-token",
            "--config-path",
            &config_path,
        ])
        .assert()
        .success();

    // Verify that file was created at the specified path
    assert!(std::path::Path::new(&config_path).exists());
    let content = fs::read_to_string(&config_path).unwrap();
    assert!(content.contains("test-profile"));
}

/// Test that --config-path flag takes precedence over SPLUNK_CONFIG_PATH env var.
#[test]
fn test_config_path_flag_takes_precedence_over_env() {
    let (_temp_dir1, config_path_env) = setup_temp_config();
    let (_temp_dir2, config_path_flag) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path_env)
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--api-token",
            "test-token",
            "--config-path",
            &config_path_flag,
        ])
        .assert()
        .success();

    // Verify that file was created at flag path, NOT env path
    assert!(std::path::Path::new(&config_path_flag).exists());
    assert!(!std::path::Path::new(&config_path_env).exists());
}

/// Test that blank --config-path flag is ignored and falls back to SPLUNK_CONFIG_PATH.
#[test]
fn test_config_path_flag_whitespace_ignored_falls_back_to_env() {
    let (_temp_dir, config_path_env) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", &config_path_env)
        .args([
            "config",
            "set",
            "test-profile",
            "--base-url",
            "https://splunk.example.com:8089",
            "--api-token",
            "test-token",
            "--config-path",
            "   ", // Whitespace flag
        ])
        .assert()
        .success();

    // Verify that file was created at env path because blank flag was ignored
    assert!(std::path::Path::new(&config_path_env).exists());
}
