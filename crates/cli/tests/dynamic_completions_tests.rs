//! Integration tests for dynamic completion functionality.
//!
//! Responsibilities:
//! - Test completion cache operations
//! - Test profile completer (offline)
//! - Test dynamic flag in completion generation
//!
//! Does NOT test:
//! - Live server fetching (requires Splunk server)
//! - Shell-specific integration (platform-specific)

mod common;

use common::splunk_cmd;
use predicates::prelude::*;

#[test]
fn test_completions_with_dynamic_flag_outputs_helpers() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions")
        .arg("bash")
        .arg("--dynamic")
        .arg("--completion-cache-ttl")
        .arg("120");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("_splunk_cli_complete_profiles"))
        .stdout(predicate::str::contains("Cache TTL: 120 seconds"));
}

#[test]
fn test_completions_without_dynamic_flag_no_helpers() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("bash");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("_splunk_cli_complete_profiles").not());
}

#[test]
fn test_completions_zsh_dynamic_outputs_helpers() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("zsh").arg("--dynamic");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("_splunk_cli_profiles"))
        .stdout(predicate::str::contains("_splunk_cli_indexes"))
        .stdout(predicate::str::contains("_splunk_cli_jobs"))
        .stdout(predicate::str::contains("_splunk_cli_apps"));
}

#[test]
fn test_completions_fish_dynamic_outputs_helpers() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("fish").arg("--dynamic");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("__splunk_cli_profiles"))
        .stdout(predicate::str::contains("__splunk_cli_indexes"));
}

#[test]
fn test_completions_powershell_dynamic_outputs_helpers() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("powershell").arg("--dynamic");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Get-SplunkCliProfiles"))
        .stdout(predicate::str::contains("Get-SplunkCliIndexes"));
}

#[test]
fn test_completions_elvish_dynamic_outputs_documentation() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("elvish").arg("--dynamic");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("splunk-cli complete profiles"))
        .stdout(predicate::str::contains("splunk-cli complete indexes"));
}

#[test]
fn test_complete_profiles_works_offline() {
    // Profile completions should work without server connection
    // (uses local config file)
    let mut cmd = splunk_cmd();
    cmd.arg("complete").arg("profiles");

    // Should succeed even if no profiles exist (returns empty list)
    cmd.assert().success();
}

#[test]
fn test_complete_command_hidden_from_help() {
    // The complete command should be hidden from --help output
    let mut cmd = splunk_cmd();
    cmd.arg("--help");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("complete").not());
}

#[test]
fn test_complete_command_accepts_cache_ttl() {
    // The complete command should accept --cache-ttl flag
    let mut cmd = splunk_cmd();
    cmd.arg("complete")
        .arg("profiles")
        .arg("--cache-ttl")
        .arg("300");

    // Should succeed (profiles work offline)
    cmd.assert().success();
}

#[test]
fn test_completions_dynamic_flag_requires_shell() {
    // --dynamic flag should work but still require shell argument
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("--dynamic"); // Missing shell argument

    cmd.assert().failure();
}

#[test]
fn test_completions_bash_contains_splunk_cli_reference() {
    // Verify the bash completions still contain references to splunk-cli
    // (even without dynamic flag)
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("bash");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("splunk-cli"));
}

#[test]
fn test_completions_with_dynamic_includes_all_helper_types() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("bash").arg("--dynamic");

    let output = cmd.output().expect("Failed to execute command");
    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify all helper types are included
    assert!(
        stdout.contains("_splunk_cli_complete_profiles"),
        "Should contain profiles helper"
    );
    assert!(
        stdout.contains("_splunk_cli_complete_indexes"),
        "Should contain indexes helper"
    );
    assert!(
        stdout.contains("_splunk_cli_complete_saved_searches"),
        "Should contain saved_searches helper"
    );
    assert!(
        stdout.contains("_splunk_cli_complete_jobs"),
        "Should contain jobs helper"
    );
    assert!(
        stdout.contains("_splunk_cli_complete_apps"),
        "Should contain apps helper"
    );
}

#[test]
fn test_completions_default_cache_ttl_is_60() {
    // When --dynamic is used without explicit --completion-cache-ttl,
    // should default to 60 seconds
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("bash").arg("--dynamic");

    cmd.assert()
        .success()
        .stdout(predicate::str::contains("Cache TTL: 60 seconds"));
}

#[test]
fn test_completions_all_shells_support_dynamic() {
    // Test that all supported shells can generate dynamic helpers
    let shells = ["bash", "zsh", "fish", "powershell", "elvish"];

    for shell in &shells {
        let mut cmd = splunk_cmd();
        cmd.arg("completions").arg(shell).arg("--dynamic");

        cmd.assert().success();
    }
}
