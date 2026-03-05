//! Tests for splunk-tui CLI argument parsing and --help output.
//!
//! Responsibilities:
//! - Verify CLI argument parsing works correctly.
//! - Validate --help output contains expected options and examples.
//! - Ensure --version output is correct.
//!
//! Does NOT handle:
//! - Integration testing of configuration loading (see config crate tests).
//! - TUI behavior testing (see app_tests.rs).
//!
//! Invariants:
//! - Tests must run with DOTENV_DISABLED=1 to ensure hermetic behavior.
//! - Binary must be built before running these tests.

use std::process::Command;

use serial_test::serial;

/// Returns the path to the splunk-tui binary.
/// Uses CARGO_BIN_EXE_splunk-tui when available (set by cargo during test runs).
fn splunk_tui_bin() -> &'static str {
    env!("CARGO_BIN_EXE_splunk-tui")
}

#[test]
#[serial]
fn test_help_exits_successfully() {
    let output = Command::new(splunk_tui_bin())
        .arg("--help")
        .env("DOTENV_DISABLED", "1")
        .output()
        .expect("Failed to run splunk-tui --help");

    assert!(output.status.success(), "--help should exit successfully");
}

#[test]
#[serial]
fn test_help_contains_expected_options() {
    let output = Command::new(splunk_tui_bin())
        .arg("--help")
        .env("DOTENV_DISABLED", "1")
        .output()
        .expect("Failed to run splunk-tui --help");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify expected options are present
    assert!(
        stdout.contains("--profile"),
        "Help should mention --profile"
    );
    assert!(
        stdout.contains("--config-path"),
        "Help should mention --config-path"
    );
    assert!(
        stdout.contains("--log-dir"),
        "Help should mention --log-dir"
    );
    assert!(
        stdout.contains("--no-mouse"),
        "Help should mention --no-mouse"
    );
    assert!(
        stdout.contains("--version"),
        "Help should mention --version"
    );
    assert!(stdout.contains("--help"), "Help should mention --help");
}

#[test]
#[serial]
fn test_help_contains_examples() {
    let output = Command::new(splunk_tui_bin())
        .arg("--help")
        .env("DOTENV_DISABLED", "1")
        .output()
        .expect("Failed to run splunk-tui --help");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify examples are present
    assert!(
        stdout.contains("Examples:"),
        "Help should include examples section"
    );
    assert!(
        stdout.contains("--profile production"),
        "Examples should show profile usage"
    );
    assert!(
        stdout.contains("--config-path"),
        "Examples should show config-path usage"
    );
    assert!(
        stdout.contains("--log-dir"),
        "Examples should show log-dir usage"
    );
    assert!(
        stdout.contains("--no-mouse"),
        "Examples should show no-mouse usage"
    );
}

#[test]
#[serial]
fn test_version_exits_successfully() {
    let output = Command::new(splunk_tui_bin())
        .arg("--version")
        .env("DOTENV_DISABLED", "1")
        .output()
        .expect("Failed to run splunk-tui --version");

    assert!(
        output.status.success(),
        "--version should exit successfully"
    );
}

#[test]
#[serial]
fn test_version_contains_binary_name() {
    let output = Command::new(splunk_tui_bin())
        .arg("--version")
        .env("DOTENV_DISABLED", "1")
        .output()
        .expect("Failed to run splunk-tui --version");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("splunk-tui"),
        "Version should contain binary name"
    );
}

#[test]
#[serial]
fn test_help_short_flag() {
    let output = Command::new(splunk_tui_bin())
        .arg("-h")
        .env("DOTENV_DISABLED", "1")
        .output()
        .expect("Failed to run splunk-tui -h");

    assert!(output.status.success(), "-h should exit successfully");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("--profile") || stdout.contains("-p"),
        "Short help should mention profile option"
    );
}

#[test]
#[serial]
fn test_version_short_flag() {
    let output = Command::new(splunk_tui_bin())
        .arg("-V")
        .env("DOTENV_DISABLED", "1")
        .output()
        .expect("Failed to run splunk-tui -V");

    assert!(output.status.success(), "-V should exit successfully");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(
        stdout.contains("splunk-tui"),
        "Short version should contain binary name"
    );
}

#[test]
#[serial]
fn test_invalid_argument_exits_with_error() {
    let output = Command::new(splunk_tui_bin())
        .arg("--invalid-argument")
        .env("DOTENV_DISABLED", "1")
        .output()
        .expect("Failed to run splunk-tui with invalid argument");

    assert!(
        !output.status.success(),
        "Invalid argument should exit with error"
    );

    let stderr = String::from_utf8_lossy(&output.stderr);
    assert!(
        stderr.contains("error") || stderr.contains("Error"),
        "Error output should mention 'error'"
    );
}

#[test]
#[serial]
fn test_profile_short_flag_accepted() {
    // Test that -p is accepted as a short flag for --profile
    // Use --help to verify the flag is recognized without actually starting the TUI,
    // which would hang in a non-interactive CI environment
    let output = Command::new(splunk_tui_bin())
        .args(["-p", "test-profile", "--help"])
        .env("DOTENV_DISABLED", "1")
        .output()
        .expect("Failed to run splunk-tui -p test-profile --help");

    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);

    // --help should succeed and show the help output
    assert!(
        output.status.success(),
        "--help should exit successfully even with -p flag. stdout: {}, stderr: {}",
        stdout,
        stderr
    );

    // Should NOT contain "unexpected argument" since -p is valid
    assert!(
        !stderr.contains("unexpected argument"),
        "-p should be recognized as a valid short flag, got: {}",
        stderr
    );

    // Help output should be present
    assert!(
        stdout.contains("--profile"),
        "Help output should mention --profile"
    );
}

#[test]
#[serial]
fn test_help_output_snapshot() {
    let output = Command::new(splunk_tui_bin())
        .arg("--help")
        .env("DOTENV_DISABLED", "1")
        .output()
        .expect("Failed to run splunk-tui --help");

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Normalize version numbers to prevent snapshot drift
    // Replace sequences of digits and dots with "X.X.X" pattern
    let normalized = regex_replace_version(&stdout);

    insta::assert_snapshot!("splunk_tui_help", normalized);
}

/// Replace version numbers like "1.2.3" with "X.X.X" for stable snapshots.
fn regex_replace_version(s: &str) -> String {
    // Simple state machine to replace digit sequences
    let mut result = String::with_capacity(s.len());
    let mut in_version = false;

    for c in s.chars() {
        if c.is_ascii_digit() {
            if !in_version {
                result.push('X');
                in_version = true;
            }
            // Skip additional digits in the same number
        } else if c == '.' && in_version {
            result.push('.');
            in_version = false; // Reset to expect next number
        } else {
            result.push(c);
            in_version = false;
        }
    }

    result
}
