use predicates::prelude::*;

#[test]
fn test_saved_searches_help() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");

    cmd.args(["saved-searches", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("List saved searches"))
        .stdout(predicate::str::contains("Show detailed information"))
        .stdout(predicate::str::contains("Run a saved search"));
}

#[test]
fn test_saved_searches_list_help() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");

    cmd.args(["saved-searches", "list", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--count"));
}

#[test]
fn test_saved_searches_run_help() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");

    cmd.args(["saved-searches", "run", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("--wait"))
        .stdout(predicate::str::contains("--earliest"))
        .stdout(predicate::str::contains("--latest"));
}

#[test]
fn test_saved_searches_info_help() {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");

    cmd.args(["saved-searches", "info", "--help"])
        .assert()
        .success()
        .stdout(predicate::str::contains("<NAME>"));
}

#[test]
fn test_saved_searches_list_output_format_parsing() {
    let formats = ["json", "table", "csv", "xml"];

    for format in formats {
        let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");
        cmd.args(["saved-searches", "list", "--output", format, "--help"])
            .assert()
            .success();
    }
}
