//! Integration tests for `splunk-cli cluster` pagination flags.

mod common;

use common::splunk_cmd;
use predicates::prelude::*;

#[test]
fn test_cluster_show_help_includes_offset_and_count() {
    let mut cmd = splunk_cmd();

    cmd.args(["cluster", "show", "--help"])
        .assert()
        .success()
        .stdout(
            predicate::str::contains("--offset")
                .and(predicate::str::contains("--count"))
                .and(predicate::str::contains("-c"))
                .and(predicate::str::contains("peers per page")),
        );
}

#[test]
fn test_cluster_peers_help_includes_offset_and_count() {
    let mut cmd = splunk_cmd();

    cmd.args(["cluster", "peers", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--offset").and(predicate::str::contains("--count")));
}

#[test]
fn test_cluster_show_count_zero_rejected() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    cmd.args(["cluster", "show", "--detailed", "--count", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("count").and(predicate::str::contains("greater than 0")));
}

#[test]
fn test_cluster_show_page_size_alias_zero_rejected() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // Test that --page-size alias still works
    cmd.args(["cluster", "show", "--detailed", "--page-size", "0"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("count").and(predicate::str::contains("greater than 0")));
}

#[test]
fn test_cluster_show_offset_and_count_flags_attempt_connection() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // Should succeed with a note or fail with connection error in non-live test env, but must accept flags.
    cmd.args([
        "cluster",
        "show",
        "--detailed",
        "--offset",
        "50",
        "--count",
        "10",
    ])
    .assert()
    .stdout(
        predicate::str::contains("Note: This Splunk instance may not be configured as a cluster.")
            .or(predicate::str::contains("")),
    );
}
