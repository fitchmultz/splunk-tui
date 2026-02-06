//! Integration tests for `splunk-cli config` command.
//!
//! This module contains tests organized by subcommand and functionality.
//!
//! Responsibilities:
//! - Test all config subcommands: list, set, delete, show, edit
//! - Test output formats and validation
//! - Test keyring integration and config path handling
//!
//! Does NOT:
//! - Test live Splunk server interactions (see `test-live` in Makefile).
//!
//! Invariants:
//! - All tests use hermetic CLI commands via `splunk_cmd()` to prevent env leakage.
//! - Tests use temporary config files to avoid interfering with user config.

mod delete_tests;
mod edge_cases_tests;
mod help_tests;
mod keyring_tests;
mod list_tests;
mod set_tests;
mod show_tests;

use tempfile::TempDir;

/// Helper function to create a temporary config file for testing.
fn setup_temp_config() -> (TempDir, String) {
    let temp_dir = TempDir::new().expect("Failed to create temp directory");
    let config_path = temp_dir.path().join("config.json");
    let config_path_str = config_path.to_string_lossy().to_string();
    (temp_dir, config_path_str)
}
