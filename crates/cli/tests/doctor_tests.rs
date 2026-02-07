//! Integration tests for `splunk-cli doctor` command.

mod common;

use common::splunk_cmd;
use predicates::prelude::*;
use std::io::Read;
use tempfile::TempDir;

/// Test that `splunk-cli doctor --help` shows the command and examples.
#[test]
fn test_doctor_help() {
    let mut cmd = splunk_cmd();
    cmd.args(["doctor", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("Run comprehensive diagnostics"))
        .stdout(predicate::str::contains("--bundle"))
        .stdout(predicate::str::contains("--include-logs"))
        .stdout(predicate::str::contains("Write a redacted support bundle"));
}

/// Test that `splunk-cli doctor` fails gracefully with missing config.
#[test]
fn test_doctor_missing_config() {
    let mut cmd = splunk_cmd();
    cmd.env_remove("SPLUNK_API_TOKEN")
        .env_remove("SPLUNK_BASE_URL")
        .arg("doctor")
        .assert()
        .failure()
        .stderr(predicate::str::contains("Base URL is required"));
}

/// Test that `splunk-cli doctor` fails gracefully with invalid URL.
#[test]
fn test_doctor_invalid_url() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "not-a-valid-url")
        .arg("doctor")
        .assert()
        .failure()
        .stderr(predicate::str::contains("URL").or(predicate::str::contains("url")));
}

/// Test that doctor command produces JSON output correctly.
#[test]
fn test_doctor_json_output() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args(["doctor", "--output", "json"])
        .assert()
        .failure() // Will fail due to connection refused
        .stdout(predicate::str::contains("cli_version"))
        .stdout(predicate::str::contains("checks"));
}

/// Test that doctor command produces table output correctly.
#[test]
fn test_doctor_table_output() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args(["doctor", "--output", "table"])
        .assert()
        .failure() // Will fail due to connection refused
        .stdout(predicate::str::contains("Doctor Report"))
        .stdout(predicate::str::contains("Configuration Summary"));
}

/// Test that bundle generation works and creates a valid zip file.
#[test]
fn test_doctor_bundle_generation() {
    let temp_dir = TempDir::new().unwrap();
    let bundle_path = temp_dir.path().join("support-bundle.zip");

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args(["doctor", "--bundle", bundle_path.to_str().unwrap()])
        .assert()
        .failure(); // Connection will fail, but bundle should still be created before the failure

    // Verify bundle was created
    assert!(bundle_path.exists(), "Support bundle should be created");

    // Verify bundle is a valid zip file
    let file = std::fs::File::open(&bundle_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();

    // Should contain diagnostic_report.json
    let has_report = archive.file_names().any(|n| n == "diagnostic_report.json");
    assert!(has_report, "Bundle should contain diagnostic_report.json");

    // Should contain environment.txt
    let has_env = archive.file_names().any(|n| n == "environment.txt");
    assert!(has_env, "Bundle should contain environment.txt");

    // Verify the report doesn't contain health_output (redacted for security)
    let mut report_file = archive.by_name("diagnostic_report.json").unwrap();
    let mut report_content = String::new();
    report_file.read_to_string(&mut report_content).unwrap();
    drop(report_file);

    // Parse and verify structure
    let report: serde_json::Value = serde_json::from_str(&report_content).unwrap();
    assert!(
        report.get("cli_version").is_some(),
        "Report should have cli_version"
    );
    assert!(report.get("checks").is_some(), "Report should have checks");

    // health_output and partial_errors should NOT be in the bundle (security)
    assert!(
        report.get("health_output").is_none(),
        "health_output should be redacted from bundle"
    );
    assert!(
        report.get("partial_errors").is_none(),
        "partial_errors should be redacted from bundle"
    );
}

/// Test that bundle generation with --include-logs flag is accepted.
#[test]
fn test_doctor_bundle_with_logs_flag() {
    let temp_dir = TempDir::new().unwrap();
    let bundle_path = temp_dir.path().join("support-bundle.zip");

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args([
            "doctor",
            "--bundle",
            bundle_path.to_str().unwrap(),
            "--include-logs",
        ])
        .assert()
        .failure();

    // Verify bundle was created
    assert!(bundle_path.exists(), "Support bundle should be created");
}

/// Test that doctor command includes all expected diagnostic checks in output.
#[test]
fn test_doctor_output_includes_checks() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args(["doctor", "--output", "json"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("config_load"))
        .stdout(predicate::str::contains("auth_strategy"))
        .stdout(predicate::str::contains("client_build"));
}

/// Test that environment variable values are redacted in support bundles.
#[test]
fn test_doctor_bundle_redaction() {
    let temp_dir = TempDir::new().unwrap();
    let bundle_path = temp_dir.path().join("support-bundle.zip");

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089")
        .env("SPLUNK_API_TOKEN", "super-secret-token-12345")
        .args(["doctor", "--bundle", bundle_path.to_str().unwrap()])
        .assert()
        .failure();

    // Verify bundle was created
    assert!(bundle_path.exists(), "Support bundle should be created");

    // Read and verify environment.txt redaction
    let file = std::fs::File::open(&bundle_path).unwrap();
    let mut archive = zip::ZipArchive::new(file).unwrap();
    let mut env_file = archive.by_name("environment.txt").unwrap();
    let mut env_content = String::new();
    env_file.read_to_string(&mut env_content).unwrap();

    // Verify redaction marker is present
    assert!(
        env_content.contains("***REDACTED***"),
        "Environment values should be redacted: {}",
        env_content
    );

    // Verify the secret token is NOT in the bundle
    assert!(
        !env_content.contains("super-secret-token-12345"),
        "Secret token should not appear in bundle"
    );
}

/// Test that doctor CSV output works correctly.
#[test]
fn test_doctor_csv_output() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args(["doctor", "--output", "csv"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("cli_version"))
        .stdout(predicate::str::contains("check_name"));
}

/// Test that doctor XML output works correctly.
#[test]
fn test_doctor_xml_output() {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args(["doctor", "--output", "xml"])
        .assert()
        .failure()
        .stdout(predicate::str::contains("<?xml"))
        .stdout(predicate::str::contains("diagnosticReport"));
}

/// Test that doctor command can write output to a file.
#[test]
fn test_doctor_output_to_file() {
    let temp_dir = TempDir::new().unwrap();
    let output_path = temp_dir.path().join("doctor-output.json");

    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", "https://localhost:8089")
        .args([
            "doctor",
            "--output",
            "json",
            "--output-file",
            output_path.to_str().unwrap(),
        ])
        .assert()
        .failure();

    // Verify output file was created
    assert!(output_path.exists(), "Output file should be created");

    // Verify it contains valid JSON
    let content = std::fs::read_to_string(&output_path).unwrap();
    let output: serde_json::Value = serde_json::from_str(&content).unwrap();
    assert!(output.get("cli_version").is_some());
    assert!(output.get("checks").is_some());
}
