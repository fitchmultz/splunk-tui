//! Purpose: Enforce deterministic temp file cleanup patterns in tests.
//!
//! Ensures all temp file creation uses the tempfile crate's RAII types
//! rather than std::env::temp_dir() with manual cleanup.
//!
//! Non-scope: This test does not verify runtime behavior; it only checks
//! source code patterns. Files are analyzed statically.
//!
//! Invariants:
//! - All test files must use tempfile crate for temp file management
//! - No hardcoded /tmp paths allowed in tests
//! - Manual cleanup via std::fs::remove_file is discouraged in tests

use std::fs;

/// Files exempt from the tempfile requirement (e.g., they use other patterns correctly)
const EXEMPT_FILES: &[&str] = &[];

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
        let is_test_file = path_str.contains("/tests/") || path_str.contains("_tests.rs");
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

        // Check for hardcoded /tmp paths in test files
        if content.contains("\"/tmp") || content.contains("'/tmp") {
            violations.push(format!(
                "{}: contains hardcoded /tmp path - prefer tempfile crate",
                path.display()
            ));
        }

        // Check for manual remove_file in test files (fragile cleanup pattern)
        // This is a warning-level check - manual cleanup may be acceptable in some cases
        if is_test_file && content.contains("std::fs::remove_file") && !content.contains("tempfile")
        {
            violations.push(format!(
                "{}: uses std::fs::remove_file without tempfile - prefer NamedTempFile for automatic cleanup",
                path.display()
            ));
        }
    }

    assert!(
        violations.is_empty(),
        "Found manual temp file patterns (not panic-safe):\n{}",
        violations.join("\n")
    );
}

#[test]
fn test_tempfile_bindings_retained() {
    // This test scans for patterns where tempfile instances are created
    // but immediately dropped, defeating RAII cleanup
    let mut violations: Vec<String> = Vec::new();

    for entry in walkdir::WalkDir::new("crates")
        .into_iter()
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|ext| ext == "rs"))
    {
        let path = entry.path();
        let path_str = path.to_string_lossy();

        // Skip non-test files
        if !path_str.contains("/tests/") && !path_str.contains("_tests.rs") {
            continue;
        }

        let content = fs::read_to_string(path).unwrap_or_default();

        // Skip files without tests
        if !content.contains("#[test]") && !content.contains("#[tokio::test]") {
            continue;
        }

        // Check for patterns where tempfile is created but not bound to a variable
        // Pattern: `let _ = tempfile::` or `tempfile::tempdir().unwrap();` on its own line
        let lines: Vec<&str> = content.lines().collect();
        for (i, line) in lines.iter().enumerate() {
            let trimmed = line.trim();

            // Check for `let _ = tempfile::` pattern (immediate drop)
            if trimmed.starts_with("let _ = tempfile::")
                || trimmed.starts_with("let _ = tempdir()")
                || trimmed.starts_with("let _ = NamedTempFile")
            {
                violations.push(format!(
                    "{}:{}: tempfile instance bound to `_` - use a named variable for RAII cleanup",
                    path.display(),
                    i + 1
                ));
            }

            // Check for standalone tempfile call not assigned (would be immediately dropped)
            if (trimmed.contains("tempfile::tempdir()") || trimmed.contains("tempdir().unwrap()"))
                && !trimmed.starts_with("let ")
                && !trimmed.starts_with("//")
            {
                violations.push(format!(
                    "{}:{}: tempfile call result not retained - bind to a variable for RAII cleanup",
                    path.display(),
                    i + 1
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Found tempfile instances not properly retained:\n{}",
        violations.join("\n")
    );
}
