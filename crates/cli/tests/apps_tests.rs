//! Integration tests for `splunk-cli apps` command.
//!
//! Responsibilities:
//! - Validate `apps list`, `info`, `enable`, `disable`, `install`, and `remove` subcommands.
//! - Ensure help text and argument validation work correctly.
//! - Verify that commands attempt network connection with correct parameters.
//!
//! Does NOT:
//! - Perform live tests against a real Splunk server (see `test-live`).
//!
//! Invariants:
//! - All tests use the hermetic `splunk_cmd()` helper.

mod common;

use common::{connection_error_predicate, splunk_cmd};
use predicates::prelude::*;

/// Test that `splunk-cli apps` shows help
#[test]
fn test_apps_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["apps", "--help"]).assert().success().stdout(
        predicate::str::contains("List and manage installed Splunk apps")
            .and(predicate::str::contains("list"))
            .and(predicate::str::contains("info"))
            .and(predicate::str::contains("enable"))
            .and(predicate::str::contains("disable"))
            .and(predicate::str::contains("install"))
            .and(predicate::str::contains("remove")),
    );
}

/// Test that `splunk-cli apps list --help` shows list options
#[test]
fn test_apps_list_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["apps", "list", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("List installed apps")
                .and(predicate::str::contains("--count")),
        );
}

/// Test that `splunk-cli apps list --count` with valid value
#[test]
fn test_apps_list_count_valid() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["apps", "list", "--count", "10"]).assert();

    // Should attempt to connect (pass count parameter to endpoint)
    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli apps info --help` shows info usage
#[test]
fn test_apps_info_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["apps", "info", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Show detailed information about an app")
                .and(predicate::str::contains("<APP_NAME>")),
        );
}

/// Test that missing app name for info shows error
#[test]
fn test_apps_info_missing_name() {
    let mut cmd = splunk_cmd();

    cmd.args(["apps", "info"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ));
}

/// Test that `splunk-cli apps info <name>` with valid name
#[test]
fn test_apps_info_valid_name() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["apps", "info", "search"]).assert();

    // Should attempt to connect
    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli apps enable --help` shows enable usage
#[test]
fn test_apps_enable_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["apps", "enable", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Enable an app by name"));
}

/// Test that missing app name for enable shows error
#[test]
fn test_apps_enable_missing_name() {
    let mut cmd = splunk_cmd();

    cmd.args(["apps", "enable"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ));
}

/// Test that `splunk-cli apps enable <name>` with valid name
#[test]
fn test_apps_enable_valid_name() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["apps", "enable", "my_app"]).assert();

    // Should attempt to connect
    result.failure().stderr(connection_error_predicate());
}

/// Test that `splunk-cli apps disable --help` shows disable usage
#[test]
fn test_apps_disable_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["apps", "disable", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Disable an app by name"));
}

/// Test that missing app name for disable shows error
#[test]
fn test_apps_disable_missing_name() {
    let mut cmd = splunk_cmd();

    cmd.args(["apps", "disable"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ));
}

/// Test that `splunk-cli apps disable <name>` with valid name
#[test]
fn test_apps_disable_valid_name() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["apps", "disable", "my_app"]).assert();

    // Should attempt to connect
    result.failure().stderr(connection_error_predicate());
}

// Note: Live integration tests (with mocked HTTP server) would require:
// - mockito or similar HTTP mocking library
// - Mocking GET /services/apps/local/{name} for info
// - Mocking POST /services/apps/local/{name} for enable/disable
// - Mocking POST /services/apps/appinstall for install
// - Mocking DELETE /services/apps/local/{name} for remove
// Following patterns from existing test files (jobs_tests.rs, etc.)

/// Test that `splunk-cli apps install --help` shows install usage
#[test]
fn test_apps_install_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["apps", "install", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Install an app from a .spl file")
                .and(predicate::str::contains("<FILE_PATH>")),
        );
}

/// Test that missing file path for install shows error
#[test]
fn test_apps_install_missing_file() {
    let mut cmd = splunk_cmd();

    cmd.args(["apps", "install"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ));
}

/// Test that non-existent file shows error
#[test]
fn test_apps_install_nonexistent_file() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd
        .args(["apps", "install", "/nonexistent/path/app.spl"])
        .assert();

    // Should fail with file not found error before attempting network
    result
        .failure()
        .stderr(predicate::str::contains("App package file not found"));
}

/// Test that `splunk-cli apps remove --help` shows remove usage
#[test]
fn test_apps_remove_help() {
    let mut cmd = splunk_cmd();

    cmd.args(["apps", "remove", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("Remove (uninstall) an app by name")
                .and(predicate::str::contains("<APP_NAME>"))
                .and(predicate::str::contains("--force")),
        );
}

/// Test that missing app name for remove shows error
#[test]
fn test_apps_remove_missing_name() {
    let mut cmd = splunk_cmd();

    cmd.args(["apps", "remove"])
        .assert()
        .failure()
        .stderr(predicate::str::contains(
            "required arguments were not provided",
        ));
}

/// Test that `splunk-cli apps remove <name>` with valid name
#[test]
fn test_apps_remove_valid_name() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    let result = cmd.args(["apps", "remove", "my_app", "--force"]).assert();

    // Should attempt to connect
    result.failure().stderr(connection_error_predicate());
}
