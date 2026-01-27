//! Regression tests for hermetic test isolation around dotenv loading.
//!
//! Responsibilities:
//! - Prove that setting `DOTENV_DISABLED=1` prevents the CLI from loading `.env`.
//! - Prove that when not disabled, the CLI can load `.env` from the working directory.
//!
//! Does NOT:
//! - Validate `.env.test` live-test behavior (covered by live tests).
//!
//! Invariants / assumptions:
//! - The CLI loads dotenv before clap parsing (so clap `env = "..."` can read `.env` values).
//! - `ConfigLoader::load_dotenv()` is gated by `DOTENV_DISABLED` ("true" or "1" disables).
//! - The `health` command requires config; with no config it errors with "Base URL is required".

mod common;

use common::splunk_cmd;
use predicates::prelude::*;
use std::fs;
use tempfile::TempDir;

fn hermetic_cli_cmd(dir: &std::path::Path) -> assert_cmd::Command {
    let mut cmd = splunk_cmd();
    cmd.current_dir(dir);

    // Ensure no pre-existing env provides config (the `.env` file should be the only source
    // when we want dotenv enabled).
    // Note: splunk_cmd sets SPLUNK_API_TOKEN by default; we remove it here to prove it's loaded from .env
    cmd.env_remove("SPLUNK_API_TOKEN");

    cmd
}

#[test]
fn test_dotenv_disabled_ignores_env_file() {
    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join(".env");

    // If dotenv were loaded, this would provide a complete config (base_url + token).
    fs::write(
        &env_path,
        "SPLUNK_BASE_URL=https://127.0.0.1:9\nSPLUNK_API_TOKEN=test-token\n",
    )
    .unwrap();

    let mut cmd = hermetic_cli_cmd(temp_dir.path());
    cmd.env("DOTENV_DISABLED", "1");
    cmd.args(["health"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Base URL is required"));
}

#[test]
fn test_dotenv_enabled_loads_env_file() {
    let temp_dir = TempDir::new().unwrap();
    let env_path = temp_dir.path().join(".env");

    fs::write(
        &env_path,
        "SPLUNK_BASE_URL=https://127.0.0.1:9\nSPLUNK_API_TOKEN=test-token\n",
    )
    .unwrap();

    let mut cmd = hermetic_cli_cmd(temp_dir.path());

    // Explicitly enable dotenv for the spawned process, even if the parent runner has it disabled.
    cmd.env_remove("DOTENV_DISABLED");

    // We don't assert the exact connection error text (varies by platform),
    // but it must NOT be the config validation error from missing base URL.
    cmd.args(["health"])
        .assert()
        .failure()
        .stderr(predicate::str::contains("Base URL is required").not());
}
