//! Testing utilities for Splunk client tests.
//!
//! This module provides helper functions for loading test fixtures,
//! generating test data, and other test-related utilities.
//! Available when running tests or when the `test-utils` feature is enabled.
//!
//! # Example
//! ```ignore
//! use splunk_client::testing::{load_fixture, generators::SearchResultsGenerator};
//!
//! // Load a static fixture
//! let fixture = load_fixture("indexes/list_indexes.json");
//!
//! // Generate dynamic test data
//! let generator = SearchResultsGenerator::new()
//!     .with_row_count(100)
//!     .with_column_count(5);
//! let data = generator.generate();
//! ```

#[cfg(any(feature = "test-utils", test))]
pub mod generators;

use std::path::Path;

/// Load a JSON fixture file from the fixtures directory.
///
/// # Arguments
/// * `fixture_path` - Relative path within the fixtures directory (e.g., "indexes/list_indexes.json")
///
/// # Panics
/// - If the fixture file cannot be read
/// - If the file content is not valid JSON
pub fn load_fixture(fixture_path: &str) -> serde_json::Value {
    let manifest_dir = Path::new(env!("CARGO_MANIFEST_DIR"));
    let fixture_dir = manifest_dir.join("fixtures");
    let full_path = fixture_dir.join(fixture_path);
    let content = std::fs::read_to_string(&full_path)
        .unwrap_or_else(|_| panic!("Failed to load fixture: {}", full_path.display()));
    serde_json::from_str(&content).expect("Invalid JSON in fixture")
}
