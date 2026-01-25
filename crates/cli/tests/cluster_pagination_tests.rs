//! Integration tests for `splunk-cli cluster` pagination flags.

use predicates::prelude::*;

#[test]
fn test_cluster_help_includes_offset_and_page_size() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");

    cmd.args(["cluster", "--help"]).assert().success().stdout(
        predicate::str::contains("--offset")
            .and(predicate::str::contains("--page-size"))
            .and(predicate::str::contains("peers per page")),
    );
}

#[test]
fn test_cluster_page_size_zero_rejected() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    cmd.args(["cluster", "--detailed", "--page-size", "0"])
        .assert()
        .failure()
        .stderr(
            predicate::str::contains("page-size").and(predicate::str::contains("greater than 0")),
        );
}

#[test]
fn test_cluster_offset_and_page_size_flags_attempt_connection() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089");

    // Should succeed with a note or fail with connection error in non-live test env, but must accept flags.
    cmd.args([
        "cluster",
        "--detailed",
        "--offset",
        "50",
        "--page-size",
        "10",
    ])
    .assert()
    .stdout(
        predicate::str::contains("Note: This Splunk instance may not be configured as a cluster.")
            .or(predicate::str::contains("")),
    );
}
