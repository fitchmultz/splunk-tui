//! List command tests for `splunk-cli config`.
//!
//! Tests the `config list` subcommand with various output formats and states.

use crate::common::splunk_cmd;
use crate::config::setup_temp_config;
use predicates::prelude::*;

/// Test that `splunk-cli config list` executes successfully.
#[test]
fn test_config_list_executes() {
    splunk_cmd()
        .args(["-o", "json", "config", "list"])
        .assert()
        .success();
}

/// Test that `splunk-cli config list` executes successfully with whitespace SPLUNK_CONFIG_PATH.
#[test]
fn test_config_list_executes_with_whitespace_splunk_config_path() {
    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", "   ")
        .args(["-o", "json", "config", "list"])
        .assert()
        .success();
}

/// Test that `splunk-cli config list` accepts table format.
#[test]
fn test_config_list_table_format_empty() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", config_path)
        .args(["-o", "table", "config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("No profiles configured"));
}

/// Test that `splunk-cli config list` shows no profiles message when empty.
#[test]
fn test_config_list_empty() {
    let (_temp_dir, config_path) = setup_temp_config();

    splunk_cmd()
        .env("SPLUNK_CONFIG_PATH", config_path)
        .args(["-o", "json", "config", "list"])
        .assert()
        .success()
        .stdout(predicate::str::contains("\"profiles\": {}"));
}
