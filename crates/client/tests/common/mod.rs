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

use std::time::Duration;

// Re-export test utilities from splunk-client
#[allow(unused_imports)]
pub use splunk_client::testing::load_fixture;

// Re-export commonly used types for test convenience
// These are used via `use common::*;` in test files
#[allow(unused_imports)]
pub use reqwest::Client;
#[allow(unused_imports)]
pub use splunk_client::endpoints;
#[allow(unused_imports)]
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
