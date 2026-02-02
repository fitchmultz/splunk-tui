//! Common test utilities for integration tests.
//!
//! This module provides shared helper functions and re-exports commonly used
//! types for testing the Splunk client. All integration tests should use
//! these utilities to ensure consistency.
//!
//! # Invariants
//! - Fixtures are loaded from the `fixtures/` directory relative to the crate root
//! - All fixture files must be valid JSON
//!
//! # What this does NOT handle
//! - Mock server setup (use wiremock directly in tests)
//! - Test-specific assertions or test logic

use std::path::Path;
use std::time::Duration;

/// Load a JSON fixture file from the fixtures directory.
///
/// # Arguments
/// * `fixture_path` - Relative path within the fixtures directory (e.g., "auth/login_success.json")
///
/// # Panics
/// - If the fixture file cannot be read
/// - If the file content is not valid JSON
#[allow(dead_code)]
pub fn load_fixture(fixture_path: &str) -> serde_json::Value {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir.join("fixtures");
    let full_path = fixture_dir.join(fixture_path);
    let content = std::fs::read_to_string(&full_path)
        .unwrap_or_else(|_| panic!("Failed to load fixture: {}", full_path.display()));
    serde_json::from_str(&content).expect("Invalid JSON in fixture")
}

// Re-export commonly used types for test convenience
// These are used by various test files that import common::*
#[allow(unused_imports)]
pub use reqwest::Client;
#[allow(unused_imports)]
pub use splunk_client::endpoints;
pub use wiremock::{Mock, MockServer, ResponseTemplate};

/// Advance Tokio's paused clock and yield so sleepers can observe the change.
#[allow(dead_code)]
pub async fn advance_and_yield(duration: Duration) {
    tokio::time::advance(duration).await;
    tokio::task::yield_now().await;
}

/// Assert that a task has not completed after yielding to the scheduler.
#[allow(dead_code)]
pub async fn assert_pending<T>(handle: &tokio::task::JoinHandle<T>, context: &str) {
    tokio::task::yield_now().await;
    assert!(!handle.is_finished(), "Expected pending task: {}", context);
}
