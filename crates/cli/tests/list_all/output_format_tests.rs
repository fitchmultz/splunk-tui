//! Output format validation tests for `splunk-cli list-all`.
//!
//! Tests verify JSON, table, CSV, and XML output formats.

use crate::common::splunk_cmd;
use predicates::prelude::*;

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
