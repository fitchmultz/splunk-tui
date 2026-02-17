//! Purpose: Enforce deterministic temp file cleanup patterns in tests.
//!
//! Ensures all temp file creation uses the tempfile crate's RAII types
//! rather than std::env::temp_dir() with manual cleanup.

use std::fs;

/// Files exempt from the tempfile requirement (e.g., they use other patterns correctly)
const EXEMPT_FILES: &[&str] = &[
    // The fix in this PR will make this file compliant
];

#[test]
fn test_no_manual_temp_dir_usage() {
    let mut violations: Vec<String> = Vec::new();

    for entry in walkdir::WalkDir::new("crates")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
    {
        let path = entry.path();
        let path_str = path.to_string_lossy();

        // Skip exempt files
        if EXEMPT_FILES.iter().any(|exempt| path_str.contains(exempt)) {
            continue;
        }

        // Skip non-test files (only check tests and test modules)
        let _is_test_file = path_str.contains("/tests/") || path_str.contains("_tests.rs");
        let content = fs::read_to_string(path).unwrap_or_default();

        if !content.contains("#[test]") && !content.contains("#[tokio::test]") {
            continue;
        }

        // Check for std::env::temp_dir() usage in test files
        if content.contains("std::env::temp_dir()") {
            violations.push(format!(
                "{}: uses std::env::temp_dir() - prefer tempfile::tempdir() for RAII cleanup",
                path.display()
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Found manual temp dir usage (not panic-safe):\n{}",
        violations.join("\n")
    );
}
