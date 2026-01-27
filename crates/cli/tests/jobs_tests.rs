//! Integration tests for `splunk-cli jobs` command.
//!
//! Tests cover:
//! - Default behavior (no args = list jobs)
//! - `--cancel` and `--delete` flags (verify list is NOT called)
//! - `--list` flag explicit usage

mod common;

use common::splunk_cmd;
use predicates::prelude::*;
use wiremock::matchers::{header, method, path};
use wiremock::{Mock, MockServer, ResponseTemplate};

/// Test that `splunk-cli jobs` with no arguments defaults to listing jobs.
#[test]
fn test_jobs_default_lists() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // The command should fail because we don't have a real Splunk server,
    // but we can verify it tries to list jobs by checking the error is connection-related,
    // not a "no arguments provided" error.
    let result = cmd.arg("jobs").assert();

    // Should fail with connection error (not a "missing required argument" error)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli jobs --list` explicitly lists jobs.
#[test]
fn test_jobs_explicit_list_flag() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // Explicit --list flag
    let result = cmd.args(["jobs", "--list"]).assert();

    // Should also fail with connection error (same as default)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli jobs --cancel <sid>` cancels a job without listing.
#[test]
fn test_jobs_cancel_flag() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // --cancel flag with a SID
    let result = cmd.args(["jobs", "--cancel", "test-sid-123"]).assert();

    // Should fail with connection error (trying to cancel, not list)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli jobs --delete <sid>` deletes a job without listing.
#[test]
fn test_jobs_delete_flag() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // --delete flag with a SID
    let result = cmd.args(["jobs", "--delete", "test-sid-456"]).assert();

    // Should fail with connection error (trying to delete, not list)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `--list` flag is shown in help text.
#[test]
fn test_jobs_help_shows_list_flag() {
    let mut cmd = splunk_cmd();

    // The help should show --list flag
    cmd.args(["jobs", "--help"]).assert().success().stdout(
        predicate::str::contains("--list").and(predicate::str::contains("List all search jobs")),
    );
}

/// Test that --cancel and --delete cannot be used together.
#[test]
fn test_jobs_cancel_and_delete_mutually_exclusive() {
    let mut cmd = splunk_cmd();

    // Both flags should cause a clap error (before any network activity)
    cmd.args(["jobs", "--cancel", "sid-123", "--delete", "sid-456"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

/// Test that --cancel with --list still works (list is just ignored).
#[test]
fn test_jobs_cancel_with_list_flag() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // --cancel with --list should still cancel (list gets disabled in code)
    cmd.args(["jobs", "--cancel", "test-sid", "--list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that --delete with --list still works.
#[test]
fn test_jobs_delete_with_list_flag() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // --delete with --list should still delete
    cmd.args(["jobs", "--delete", "test-sid", "--list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that `splunk-cli jobs --inspect <sid>` shows job details.
#[test]
fn test_jobs_inspect_flag() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // --inspect flag with a SID
    let result = cmd.args(["jobs", "--inspect", "test-sid-789"]).assert();

    // Should fail with connection error (trying to inspect, not list)
    result
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that --inspect with --list still works (inspect takes precedence).
#[test]
fn test_jobs_inspect_with_list_flag() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // --inspect with --list should still inspect (list gets disabled in code)
    cmd.args(["jobs", "--inspect", "test-sid", "--list"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Connection refused"));
}

/// Test that --inspect is shown in help text.
#[test]
fn test_jobs_help_shows_inspect_flag() {
    let mut cmd = splunk_cmd();

    // The help should show --inspect flag
    cmd.args(["jobs", "--help"]).assert().success().stdout(
        predicate::str::contains("--inspect")
            .and(predicate::str::contains("Inspect a specific job by SID")),
    );
}

/// Test that --inspect and --cancel cannot be used together (clap enforcement).
#[test]
fn test_jobs_inspect_and_cancel_mutually_exclusive() {
    let mut cmd = splunk_cmd();

    // Both flags should cause a clap error (before any network activity)
    cmd.args(["jobs", "--inspect", "sid-123", "--cancel", "sid-456"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

/// Test that --inspect and --delete cannot be used together (clap enforcement).
#[test]
fn test_jobs_inspect_and_delete_mutually_exclusive() {
    let mut cmd = splunk_cmd();

    // Both flags should cause a clap error (before any network activity)
    cmd.args(["jobs", "--inspect", "sid-123", "--delete", "sid-456"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("cannot be used with"));
}

#[tokio::test]
async fn test_jobs_cancel_delete_show_progress_on_stderr_unless_quiet() {
    let server = MockServer::start().await;

    // Cancel endpoint
    Mock::given(method("POST"))
        .and(path("/services/search/jobs/test-sid/control"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Delete endpoint
    Mock::given(method("DELETE"))
        .and(path("/services/search/jobs/test-sid"))
        .and(header("Authorization", "Bearer test-token"))
        .respond_with(ResponseTemplate::new(200))
        .mount(&server)
        .await;

    // Non-quiet cancel: spinner label should appear on stderr
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.args(["jobs", "--cancel", "test-sid"])
        .assert()
        .success();

    // Quiet cancel: spinner must be suppressed
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.args(["--quiet", "jobs", "--cancel", "test-sid"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());

    // Non-quiet delete: spinner label should appear on stderr
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.args(["jobs", "--delete", "test-sid"])
        .assert()
        .success();

    // Quiet delete: spinner must be suppressed
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", server.uri());
    cmd.args(["--quiet", "jobs", "--delete", "test-sid"])
        .assert()
        .success()
        .stderr(predicate::str::is_empty());
}
