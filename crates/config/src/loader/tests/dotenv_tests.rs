//! Tests for dotenv loading behavior.
//!
//! Responsibilities:
//! - Test that missing `.env` files are silently ignored.
//! - Test that invalid `.env` files return errors without leaking secrets.
//! - Test that `DOTENV_DISABLED=1`/`true` skips dotenv loading.
//!
//! Invariants / Assumptions:
//! - Tests use `env_lock()` to prevent cross-test contamination.
//! - Tests must serialize mutations to process-global state (cwd/env).
//! - Error messages must never contain secret values from `.env` files.

use std::fs;
use std::path::PathBuf;
use tempfile::TempDir;

use super::env_lock;
use crate::loader::builder::ConfigLoader;
use crate::loader::error::ConfigError;

/// RAII guard for temporarily changing the current working directory.
struct CwdGuard {
    original_dir: PathBuf,
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

/// Helper to clear the DOTENV_DISABLED variable.
fn enable_dotenv() {
    unsafe {
        std::env::remove_var("DOTENV_DISABLED");
    }
}

/// Helper to set DOTENV_DISABLED to "1".
fn disable_dotenv() {
    unsafe {
        std::env::set_var("DOTENV_DISABLED", "1");
    }
}

#[test]
fn test_missing_dotenv_is_ok() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _cwd_guard = CwdGuard::new(&temp_dir);

    enable_dotenv();

    // No .env file in temp_dir
    let result = ConfigLoader::new().load_dotenv();

    assert!(
        result.is_ok(),
        "Missing .env file should be silently ignored"
    );
}

#[test]
fn test_valid_dotenv_is_ok() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _cwd_guard = CwdGuard::new(&temp_dir);

    enable_dotenv();

    // Create a valid .env file
    fs::write(
        temp_dir.path().join(".env"),
        "SPLUNK_BASE_URL=https://example.com:8089\nSPLUNK_API_TOKEN=test-token\n",
    )
    .unwrap();

    let result = ConfigLoader::new().load_dotenv();

    assert!(result.is_ok(), "Valid .env file should load successfully");
}

#[test]
fn test_invalid_dotenv_returns_parse_error() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _cwd_guard = CwdGuard::new(&temp_dir);

    enable_dotenv();

    // Create an invalid .env file with a line that has no '='
    fs::write(temp_dir.path().join(".env"), "INVALID_LINE_WITHOUT_EQUALS").unwrap();

    let result = ConfigLoader::new().load_dotenv();

    match result {
        Err(ConfigError::DotenvParse { .. }) => {}
        Err(other) => panic!(
            "Invalid .env should return DotenvParse error, got {}",
            other
        ),
        Ok(_) => panic!("Invalid .env should return DotenvParse error, got Ok"),
    }
}

#[test]
fn test_dotenv_parse_error_does_not_leak_secrets() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _cwd_guard = CwdGuard::new(&temp_dir);

    enable_dotenv();

    let secret_value = "supersecret_token_12345";

    // Create a .env file with a valid line followed by an invalid line containing a secret
    fs::write(
        temp_dir.path().join(".env"),
        format!(
            "SPLUNK_PASSWORD={}\nINVALID_LINE_WITHOUT_EQUALS",
            secret_value
        ),
    )
    .unwrap();

    let result = ConfigLoader::new().load_dotenv();

    match &result {
        Err(e) => {
            let error_string = e.to_string();
            assert!(
                !error_string.contains(secret_value),
                "Error message should NOT contain the secret value: {}",
                error_string
            );
            assert!(
                error_string.contains(".env"),
                "Error message should mention .env file: {}",
                error_string
            );
            assert!(
                error_string.contains("DOTENV_DISABLED"),
                "Error should hint about DOTENV_DISABLED: {}",
                error_string
            );
        }
        Ok(_) => panic!("Expected error for invalid .env file, got Ok"),
    }
}

#[test]
fn test_dotenv_disabled_with_value_1() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _cwd_guard = CwdGuard::new(&temp_dir);

    // Create an invalid .env file
    fs::write(temp_dir.path().join(".env"), "INVALID_LINE_WITHOUT_EQUALS").unwrap();

    disable_dotenv();

    // With DOTENV_DISABLED=1, the invalid .env should be skipped
    let result = ConfigLoader::new().load_dotenv();

    assert!(
        result.is_ok(),
        "DOTENV_DISABLED=1 should skip .env loading even if file is invalid"
    );
}

#[test]
fn test_dotenv_disabled_with_value_true() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _cwd_guard = CwdGuard::new(&temp_dir);

    // Create an invalid .env file
    fs::write(temp_dir.path().join(".env"), "INVALID_LINE_WITHOUT_EQUALS").unwrap();

    unsafe {
        std::env::set_var("DOTENV_DISABLED", "true");
    }

    // With DOTENV_DISABLED=true, the invalid .env should be skipped
    let result = ConfigLoader::new().load_dotenv();

    assert!(
        result.is_ok(),
        "DOTENV_DISABLED=true should skip .env loading even if file is invalid"
    );
}

#[test]
fn test_dotenv_disabled_other_values_not_disabled() {
    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _cwd_guard = CwdGuard::new(&temp_dir);

    // Create an invalid .env file
    fs::write(temp_dir.path().join(".env"), "INVALID_LINE_WITHOUT_EQUALS").unwrap();

    unsafe {
        std::env::set_var("DOTENV_DISABLED", "false");
    }

    // With DOTENV_DISABLED=false, the invalid .env should NOT be skipped
    let result = ConfigLoader::new().load_dotenv();

    match result {
        Err(ConfigError::DotenvParse { .. }) => {}
        Err(other) => panic!(
            "DOTENV_DISABLED=false should NOT disable dotenv loading, got {}",
            other
        ),
        Ok(_) => panic!("DOTENV_DISABLED=false should NOT disable dotenv loading, got Ok"),
    }
}

#[cfg(unix)]
#[test]
fn test_dotenv_io_error_on_permission_denied() {
    use std::os::unix::fs::PermissionsExt;

    let _lock = env_lock().lock().unwrap();
    let temp_dir = TempDir::new().unwrap();
    let _cwd_guard = CwdGuard::new(&temp_dir);

    enable_dotenv();

    // Create a valid .env file
    let env_path = temp_dir.path().join(".env");
    fs::write(&env_path, "SPLUNK_BASE_URL=https://example.com:8089\n").unwrap();

    // Remove all permissions from the file
    let mut permissions = fs::metadata(&env_path).unwrap().permissions();
    permissions.set_mode(0o000);
    fs::set_permissions(&env_path, permissions).unwrap();

    // Try to load the .env file
    let result = ConfigLoader::new().load_dotenv();

    // Restore permissions for cleanup
    let mut permissions = fs::metadata(&env_path).unwrap().permissions();
    permissions.set_mode(0o644);
    fs::set_permissions(&env_path, permissions).unwrap();

    // The result should be an IO error (either PermissionDenied or similar)
    // Note: The exact error may vary by platform and Rust version
    match &result {
        Err(ConfigError::DotenvIo { kind }) => {
            // Permission denied is the expected error
            assert!(
                matches!(
                    kind,
                    std::io::ErrorKind::PermissionDenied | std::io::ErrorKind::Other
                ),
                "Expected PermissionDenied or Other, got {:?}",
                kind
            );
        }
        Ok(_) => {
            // Some systems (like running as root) might still succeed
            // That's acceptable - we just need to not panic
        }
        Err(other) => panic!("Expected DotenvIo error, got {}", other),
    }
}
