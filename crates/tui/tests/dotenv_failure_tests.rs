//! Integration tests for dotenv failure handling in TUI.
//!
//! Responsibilities:
//! - Prove that invalid `.env` files cause config loading to fail.
//! - Prove that error messages do not leak secrets from the `.env` file.
//! - Ensure DOTENV_DISABLED=1 allows the TUI to skip a malformed `.env`.
//!
//! Invariants / Assumptions:
//! - Tests use `serial_test` to prevent cross-test contamination.
//! - Tests serialize mutations to process-global state (cwd/env).
//! - Error messages must never contain secret values from `.env` files.

use serial_test::serial;
use splunk_config::ConfigLoader;
use std::fs;
use std::sync::{Mutex, OnceLock};
use tempfile::TempDir;

fn env_lock() -> &'static Mutex<()> {
    static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
    LOCK.get_or_init(|| Mutex::new(()))
}

/// RAII guard for temporarily changing the current working directory.
struct CwdGuard {
    original_dir: std::path::PathBuf,
}

impl CwdGuard {
    fn new(temp_dir: &TempDir) -> Self {
        let original_dir = std::env::current_dir().expect("Failed to get current directory");
        std::env::set_current_dir(temp_dir.path()).expect("Failed to set current directory");
        Self { original_dir }
    }
}

impl Drop for CwdGuard {
    fn drop(&mut self) {
        let _ = std::env::set_current_dir(&self.original_dir);
    }
}

/// Helper to enable dotenv loading by removing DOTENV_DISABLED.
fn enable_dotenv() {
    unsafe {
        std::env::remove_var("DOTENV_DISABLED");
    }
}

/// Helper to disable dotenv loading.
fn disable_dotenv() {
    unsafe {
        std::env::set_var("DOTENV_DISABLED", "1");
    }
}

/// Helper to clear all SPLUNK_* environment variables.
fn clear_splunk_env() {
    for (key, _) in std::env::vars() {
        if key.starts_with("SPLUNK_") {
            unsafe {
                std::env::remove_var(&key);
            }
        }
    }
}

/// Helper to extract error message from result without requiring Debug on ConfigLoader.
fn get_error_string<T>(result: Result<T, splunk_config::ConfigError>) -> Option<String> {
    match result {
        Err(e) => Some(e.to_string()),
        Ok(_) => None,
    }
}

#[test]
#[serial]
fn test_invalid_dotenv_causes_config_loading_failure() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _cwd_guard = CwdGuard::new(&temp_dir);

    // Create an invalid .env file
    fs::write(temp_dir.path().join(".env"), "INVALID_LINE_WITHOUT_EQUALS").unwrap();

    enable_dotenv();
    clear_splunk_env();

    let result = ConfigLoader::new().load_dotenv();

    let err_string = get_error_string(result).expect("Expected error for invalid .env file");

    assert!(
        err_string.contains(".env"),
        "Error message should mention .env file: {}",
        err_string
    );
}

#[test]
#[serial]
fn test_invalid_dotenv_does_not_leak_secrets() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _cwd_guard = CwdGuard::new(&temp_dir);

    let secret_value = "supersecret_tui_token_12345";

    // Create a .env file with a secret followed by an invalid line
    fs::write(
        temp_dir.path().join(".env"),
        format!("SPLUNK_API_TOKEN={}\nINVALID_LINE", secret_value),
    )
    .unwrap();

    enable_dotenv();
    clear_splunk_env();

    let result = ConfigLoader::new().load_dotenv();

    let err_string = get_error_string(result).expect("Expected error for invalid .env file");

    // Verify the error message does NOT contain the secret
    assert!(
        !err_string.contains(secret_value),
        "Error message should NOT contain the secret value: {}",
        err_string
    );

    // Verify the error message DOES mention .env
    assert!(
        err_string.contains(".env"),
        "Error message should mention .env file: {}",
        err_string
    );
}

#[test]
#[serial]
fn test_dotenv_disabled_skips_invalid_env_file() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _cwd_guard = CwdGuard::new(&temp_dir);

    // Create an invalid .env file
    fs::write(temp_dir.path().join(".env"), "INVALID_LINE_WITHOUT_EQUALS").unwrap();

    disable_dotenv();
    clear_splunk_env();

    // With DOTENV_DISABLED=1, the invalid .env should be skipped
    let result = ConfigLoader::new().load_dotenv();

    assert!(
        result.is_ok(),
        "DOTENV_DISABLED=1 should skip invalid .env file"
    );
}

#[test]
#[serial]
fn test_dotenv_error_message_includes_dotenv_disabled_hint() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _cwd_guard = CwdGuard::new(&temp_dir);

    // Create an invalid .env file
    fs::write(temp_dir.path().join(".env"), "INVALID_LINE_WITHOUT_EQUALS").unwrap();

    enable_dotenv();
    clear_splunk_env();

    let result = ConfigLoader::new().load_dotenv();

    let err_string = get_error_string(result).expect("Expected error for invalid .env file");

    // Verify the error message includes a hint about DOTENV_DISABLED
    assert!(
        err_string.contains("DOTENV_DISABLED"),
        "Error message should hint about DOTENV_DISABLED: {}",
        err_string
    );
}

// Note: Testing load_config_with_search_defaults directly requires constructing
// a Cli instance, which requires clap parsing. Instead, we rely on the
// ConfigLoader::load_dotenv() tests above to verify dotenv behavior, and the
// integration tests in dotenv_isolation_tests.rs to verify the full TUI startup path.
