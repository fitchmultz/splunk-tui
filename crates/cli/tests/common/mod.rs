//! Shared test utilities for splunk-cli integration tests.
//!
//! Responsibilities:
//! - Provide a hermetic CLI command factory that prevents dotenv loading.
//! - Ensure consistent test environment setup (tokens, base URLs).
//!
//! Does NOT:
//! - Handle live test configuration (see `test-live` in Makefile).
//!
//! Invariants / Assumptions:
//! - All integration tests using this helper will be hermetic by default.
//! - `SPLUNK_API_TOKEN` is set to "test-token" unless overridden.

use assert_cmd::Command;

/// Returns a hermetic `splunk-cli` command for integration testing.
///
/// It ensures:
/// - `DOTENV_DISABLED=1` is set to prevent local `.env` contamination.
/// - `SPLUNK_API_TOKEN` is set to a dummy value to satisfy config validation.
/// - Other sensitive env vars are cleared to ensure no leakage from the host.
pub fn splunk_cmd() -> Command {
    let mut cmd = assert_cmd::cargo::cargo_bin_cmd!("splunk-cli");

    // Hermeticity: prevent loading local .env
    cmd.env("DOTENV_DISABLED", "1");

    // Satisfy configuration requirements for non-config tests
    cmd.env("SPLUNK_API_TOKEN", "test-token");

    // Clear potential host leakage
    cmd.env_remove("SPLUNK_BASE_URL")
        .env_remove("SPLUNK_USERNAME")
        .env_remove("SPLUNK_PASSWORD")
        .env_remove("SPLUNK_PROFILE")
        .env_remove("SPLUNK_CONFIG_PATH");

    cmd
}
