//! Integration tests for completions and manpage commands.
//!
//! Responsibilities:
//! - Verify shell completion generation for all supported shells.
//! - Verify manpage generation produces valid output.
//! - Ensure commands work without network or config requirements.
//!
//! Does NOT test:
//! - Installation of completions/manpages (system-specific).

mod common;

use common::splunk_cmd;
use predicates::prelude::*;

#[test]
fn test_completions_bash_outputs_non_empty() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("bash");
    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_zsh_outputs_non_empty() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("zsh");
    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_fish_outputs_non_empty() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("fish");
    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_power_shell_outputs_non_empty() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("powershell");
    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_elvish_outputs_non_empty() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("elvish");
    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_man_outputs_non_empty() {
    let mut cmd = splunk_cmd();
    cmd.arg("man");
    cmd.assert()
        .success()
        .stdout(predicate::str::is_empty().not());
}

#[test]
fn test_completions_help_shows_examples() {
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("--help");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("bash"))
        .stdout(predicate::str::contains("zsh"))
        .stdout(predicate::str::contains("fish"));
}

#[test]
fn test_man_help_shows_usage() {
    let mut cmd = splunk_cmd();
    cmd.arg("man").arg("--help");
    cmd.assert().success();
}

#[test]
fn test_completions_bash_contains_cli_reference() {
    // Verify the bash completions contain references to splunk-cli
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("bash");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("splunk-cli"));
}

#[test]
fn test_completions_zsh_contains_cli_reference() {
    // Verify the zsh completions contain references to splunk-cli
    let mut cmd = splunk_cmd();
    cmd.arg("completions").arg("zsh");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("splunk-cli"));
}

#[test]
fn test_man_contains_name_section() {
    // Verify manpage contains standard manpage sections
    let mut cmd = splunk_cmd();
    cmd.arg("man");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("NAME"))
        .stdout(predicate::str::contains("splunk-cli"));
}

#[test]
fn test_man_contains_description_section() {
    // Verify manpage contains DESCRIPTION section
    let mut cmd = splunk_cmd();
    cmd.arg("man");
    cmd.assert()
        .success()
        .stdout(predicate::str::contains("DESCRIPTION"));
}
