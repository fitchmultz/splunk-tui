//! Tests for the configuration loader builder.
//!
//! Responsibilities:
//! - Test builder methods for configuration loading.
//! - Test profile loading from files.
//! - Test environment variable handling and precedence.
//! - Test search defaults, session TTL, and buffer settings.
//!
//! Does NOT handle:
//! - Direct environment variable parsing logic (tested in env.rs).
//! - Profile file loading logic (tested in profile.rs).
//! - Persisting configuration changes (tested in persistence.rs).
//!
//! Invariants:
//! - Tests use `serial_test` to prevent environment variable pollution.
//! - Tests use `global_test_lock()` for additional synchronization.
//! - Temporary directories are cleaned up automatically via `tempfile`.

use std::sync::Mutex;

pub mod basic_tests;
pub mod dotenv_tests;
pub mod env_tests;
pub mod internal_logs_defaults_tests;
pub mod profile_tests;
pub mod search_defaults_tests;
pub mod session_tests;
pub mod validation_tests;

/// Returns the global test lock for environment variable isolation.
pub fn env_lock() -> &'static Mutex<()> {
    crate::test_util::global_test_lock()
}
