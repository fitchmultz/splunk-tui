//! Live integration tests for `splunk-cli` against a real Splunk instance.
//!
//! Responsibilities:
//! - Validate end-to-end CLI wiring (args -> config -> HTTP -> output) against the dev server.
//! - Catch authentication/config regressions that mocks cannot.
//!
//! Explicitly does NOT cover:
//! - Exhaustive formatting correctness (covered by mocked integration/unit tests).
//! - TUI behavior (covered by `crates/tui` tests).
//!
//! Invariants / assumptions:
//! - A reachable Splunk server is available via `.env.test` or environment variables.
//! - Credentials are provided via environment variables or `.env.test` at the workspace root.
//! - Self-signed TLS is expected; `SPLUNK_SKIP_VERIFY=true` is recommended.
//!
//! Run with: cargo test -p splunk-cli --test live_tests -- --ignored

use predicates::prelude::*;
use std::sync::{Mutex, OnceLock};

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

fn clear_splunk_env() {
    // `.env.test` should be the source of truth for live tests. Clear any
    // pre-existing SPLUNK_* variables so dotenv parsing can't be bypassed.
    for (key, _) in std::env::vars() {
        if key.starts_with("SPLUNK_") {
            unsafe {
                std::env::remove_var(key);
            }
        }
    }
}

fn splunk_cli_cmd() -> assert_cmd::Command {
    assert_cmd::cargo::cargo_bin_cmd!("splunk-cli")
}

/// Serializes and normalizes live-test env configuration for this test binary.
struct LiveEnvGuard {
    _lock: std::sync::MutexGuard<'static, ()>,
}

impl LiveEnvGuard {
    fn new() -> Self {
        let lock = env_lock()
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());

        clear_splunk_env();

        let env_test_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
            .join("..")
            .join("..")
            .join(".env.test");

        dotenvy::from_path_override(env_test_path)
            .expect(".env.test must exist at the workspace root for live CLI tests");

        // Force the expected dev-server setting (self-signed TLS).
        unsafe {
            std::env::set_var("SPLUNK_SKIP_VERIFY", "true");
        }

        // Fail fast with actionable messages, but never print secrets.
        assert!(
            std::env::var("SPLUNK_BASE_URL").is_ok(),
            "SPLUNK_BASE_URL must be set in .env.test"
        );
        assert!(
            std::env::var("SPLUNK_USERNAME").is_ok(),
            "SPLUNK_USERNAME must be set in .env.test"
        );
        assert!(
            std::env::var("SPLUNK_PASSWORD").is_ok(),
            "SPLUNK_PASSWORD must be set in .env.test"
        );

        Self { _lock: lock }
    }
}

impl Drop for LiveEnvGuard {
    fn drop(&mut self) {
        clear_splunk_env();
    }
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_indexes_json() {
    let _env = LiveEnvGuard::new();
    let mut cmd = splunk_cli_cmd();

    cmd.args(["--output", "json", "indexes", "--count", "5"])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_users_json() {
    let _env = LiveEnvGuard::new();
    let mut cmd = splunk_cli_cmd();

    cmd.args(["--output", "json", "users", "--count", "5"])
        .assert()
        .success()
        .stdout(predicate::str::contains("admin"));
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_apps_list_json() {
    let _env = LiveEnvGuard::new();
    let mut cmd = splunk_cli_cmd();

    cmd.args(["--output", "json", "apps", "list", "--count", "5"])
        .assert()
        .success();
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_health_json() {
    let _env = LiveEnvGuard::new();
    let mut cmd = splunk_cli_cmd();

    cmd.args(["--output", "json", "health"]).assert().success();
}

#[test]
#[ignore = "requires live Splunk server"]
fn test_live_cli_search_wait_json() {
    let _env = LiveEnvGuard::new();
    let mut cmd = splunk_cli_cmd();

    cmd.args([
        "--output",
        "json",
        "search",
        "search index=main | head 1",
        "--wait",
        "--count",
        "1",
    ])
    .assert()
    .success();
}
