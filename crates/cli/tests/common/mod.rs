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
use predicates::{Predicate, prelude::*};

/// Returns a predicate that matches common connection error messages.
///
/// This predicate matches:
/// - "Connection refused" (standard TCP connection failure)
/// - "client error (Connect)" (reqwest connection error)
/// - "invalid peer certificate" (TLS certificate errors)
/// - "API error (401)" (authentication errors when a real server responds)
///
/// Use this in tests that expect connection failures when no Splunk server is running.
#[allow(dead_code)]
pub fn connection_error_predicate() -> impl Predicate<str> {
    predicate::str::contains("Connection refused")
        .or(predicate::str::contains("client error (Connect)"))
        .or(predicate::str::contains("invalid peer certificate"))
        .or(predicate::str::contains("API error (401)"))
}

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

/// Returns a hermetic `splunk-cli` command with a specific base URL.
///
/// This is a convenience wrapper around `splunk_cmd()` that sets `SPLUNK_BASE_URL`
/// to the provided value. All other hermeticity guarantees (DOTENV_DISABLED=1,
/// cleared env vars) are preserved.
#[allow(dead_code)]
pub fn splunk_cmd_with_base_url(base_url: &str) -> Command {
    let mut cmd = splunk_cmd();
    cmd.env("SPLUNK_BASE_URL", base_url);
    cmd
}
