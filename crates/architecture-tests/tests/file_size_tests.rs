//! Architecture tests for file size limits.
//!
//! Enforces CLAUDE.md guidelines:
//! - Files >700 LOC require justification (warning logged)
//! - Files >1000 LOC are presumed mis-scoped (test failure)
//!
//! This test walks all .rs files in the crates/ directory and checks their
//! line counts against established thresholds.

use std::fs;
use std::path::{Path, PathBuf};

/// Thresholds from CLAUDE.md
const WARNING_THRESHOLD: usize = 700;
const FAILURE_THRESHOLD: usize = 1000;

/// Files excluded from size checks with justification.
///
/// Each entry is a (path_suffix, justification) tuple.
/// The path_suffix is matched against the end of the file path.
const EXCLUDED_FILES: &[(&str, &str)] = &[
    (
        "formatters/tests.rs",
        "Comprehensive formatter test suite - test files may be large for coverage",
    ),
    (
        "retry_tests.rs",
        "Retry logic integration tests - test files may be large for coverage",
    ),
    (
        "side_effects_tests.rs",
        "Comprehensive side effect handler tests - covers 23 async API operations",
    ),
    (
        "keymap/bindings/screens.rs",
        "Keybinding definitions for all TUI screens - each screen requires multiple keybinding entries",
    ),
    (
        "app/popups/mod.rs",
        "Popup dispatch logic + comprehensive integration tests for all popup types",
    ),
    (
        "app/popups/profile.rs",
        "Profile management popup handlers - complex multi-field form handling for CreateProfile, EditProfile, ProfileSelector",
    ),
    (
        "app/popups/macros.rs",
        "Macro popup handlers - CreateMacro and EditMacro form handling with comprehensive test coverage",
    ),
    (
        "app/actions/data_loading.rs",
        "Data loading action handlers - handles 30+ data loading actions with comprehensive tests",
    ),
];

/// Test that enforces file size limits across the codebase.
///
/// # Failures
/// Files exceeding 1000 LOC (unless excluded) will cause this test to fail.
///
/// # Warnings
/// Files exceeding 700 LOC will produce warnings to stderr but not fail.
#[test]
fn file_size_limits() {
    let workspace_root = find_workspace_root();
    let crates_dir = workspace_root.join("crates");

    assert!(
        crates_dir.exists(),
        "crates/ directory not found at {:?}",
        crates_dir
    );

    let rust_files = find_rust_files(&crates_dir);
    let mut failures = Vec::new();
    let mut warnings = Vec::new();

    for file_path in &rust_files {
        let loc = count_loc(file_path);
        let relative_path = file_path.strip_prefix(&workspace_root).unwrap_or(file_path);
        let relative_str = relative_path.to_string_lossy();

        // Check if file is excluded
        let is_excluded = EXCLUDED_FILES
            .iter()
            .any(|(suffix, _)| relative_str.ends_with(suffix));

        if loc > FAILURE_THRESHOLD {
            if is_excluded {
                // Log excluded files that would fail but don't count as failure
                eprintln!(
                    "[EXCLUDED] {}: {} LOC (threshold: {})",
                    relative_str, loc, FAILURE_THRESHOLD
                );
            } else {
                failures.push((relative_str.to_string(), loc));
            }
        } else if loc > WARNING_THRESHOLD {
            warnings.push((relative_str.to_string(), loc));
        }
    }

    // Print warnings to stderr
    if !warnings.is_empty() {
        eprintln!(
            "\n=== File Size Warnings (files >{} LOC) ===",
            WARNING_THRESHOLD
        );
        eprintln!(
            "The following files exceed {} LOC and require justification:\n",
            WARNING_THRESHOLD
        );
        for (path, loc) in &warnings {
            eprintln!("  - {}: {} lines", path, loc);
        }
        eprintln!(
            "\nConsider refactoring these files or document why they must exceed this threshold."
        );
        eprintln!();
    }

    // Print and fail on violations
    if !failures.is_empty() {
        let mut error_message = format!(
            "\n=== Architecture Test Failed: File Size Limit Exceeded ===\n\n\
             Files exceeding {} LOC (presumed mis-scoped):\n",
            FAILURE_THRESHOLD
        );

        for (path, loc) in &failures {
            error_message.push_str(&format!("  - {}: {} lines\n", path, loc));
        }

        error_message.push_str("\nThese files must be refactored or added to EXCLUDED_FILES\n");
        error_message.push_str("with a documented justification.\n");

        if !EXCLUDED_FILES.is_empty() {
            error_message.push_str("\nCurrently excluded files:\n");
            for (pattern, justification) in EXCLUDED_FILES {
                error_message.push_str(&format!("  - {}: {}\n", pattern, justification));
            }
        }

        panic!("{}", error_message);
    }

    // Print summary
    let total_checked = rust_files.len();
    eprintln!(
        "\n[architecture] Checked {} Rust files for size limits.",
        total_checked
    );
    if !warnings.is_empty() {
        eprintln!(
            "[architecture] {} files exceed {} LOC (warnings).",
            warnings.len(),
            WARNING_THRESHOLD
        );
    }
    eprintln!("[architecture] All files within acceptable size limits.");
}

/// Count lines of code in a file, excluding blank lines and comments.
///
/// # Counting Rules
/// - Empty lines (whitespace only) are skipped
/// - Lines starting with `//` are skipped (single-line comments)
/// - Lines starting with `///` or `//!` are skipped (doc comments)
/// - Lines containing code followed by `//` are counted
/// - Block comments (`/* */`) are treated simplistically:
///   lines within block comments are still counted if they don't start with `*`
fn count_loc(path: &Path) -> usize {
    let content = fs::read_to_string(path).expect("Failed to read file");
    let mut count = 0;
    let mut in_block_comment = false;

    for line in content.lines() {
        let trimmed = line.trim();

        // Skip empty lines
        if trimmed.is_empty() {
            continue;
        }

        // Track block comment state (simplified)
        if trimmed.starts_with("/*") && !trimmed.starts_with("/**") {
            in_block_comment = true;
        }
        if trimmed.ends_with("*/") {
            in_block_comment = false;
            continue;
        }

        // Skip lines that are only block comment continuations
        if in_block_comment && trimmed.starts_with('*') {
            continue;
        }

        // Skip single-line comments
        if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
            continue;
        }

        count += 1;
    }

    count
}

/// Recursively find all .rs files in a directory.
fn find_rust_files(dir: &Path) -> Vec<PathBuf> {
    let mut files = Vec::new();

    if let Ok(entries) = fs::read_dir(dir) {
        for entry in entries.flatten() {
            let path = entry.path();

            if path.is_dir() {
                // Skip target directories (build artifacts)
                if path.file_name() == Some(std::ffi::OsStr::new("target")) {
                    continue;
                }
                // Skip the architecture-tests crate itself to avoid recursion issues
                if path.file_name() == Some(std::ffi::OsStr::new("architecture-tests")) {
                    continue;
                }
                files.extend(find_rust_files(&path));
            } else if path.extension() == Some(std::ffi::OsStr::new("rs")) {
                files.push(path);
            }
        }
    }

    files
}

/// Find the workspace root by looking for Cargo.toml with [workspace].
fn find_workspace_root() -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");

    // Start from current directory and walk up
    let mut dir = current_dir.as_path();
    loop {
        let cargo_toml = dir.join("Cargo.toml");
        if cargo_toml.exists()
            && let Ok(content) = fs::read_to_string(&cargo_toml)
            && content.contains("[workspace]")
        {
            return dir.to_path_buf();
        }

        match dir.parent() {
            Some(parent) => dir = parent,
            None => {
                // Fall back to current directory if no workspace found
                return current_dir;
            }
        }
    }
}

#[test]
fn test_count_loc_basic() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_loc_count.rs");

    fs::write(
        &test_file,
        r#"// This is a comment
fn main() {
    let x = 5; // inline comment

    // Another comment
    println!("Hello");
}
"#,
    )
    .unwrap();

    let loc = count_loc(&test_file);
    // Should count: fn main() {, let x = 5;, println!(...);, }
    assert_eq!(loc, 4);

    let _ = fs::remove_file(&test_file);
}

#[test]
fn test_count_loc_doc_comments() {
    let temp_dir = std::env::temp_dir();
    let test_file = temp_dir.join("test_doc_comments.rs");

    fs::write(
        &test_file,
        r#"//! Module documentation

/// Function documentation
fn test() {
    // implementation
    let x = 1;
}
"#,
    )
    .unwrap();

    let loc = count_loc(&test_file);
    // Should count: fn test() {, let x = 1;, }
    assert_eq!(loc, 3);

    let _ = fs::remove_file(&test_file);
}

#[test]
fn test_excluded_files_list_is_consistent() {
    // Verify that all excluded file patterns are non-empty
    for (pattern, justification) in EXCLUDED_FILES {
        assert!(
            !pattern.is_empty(),
            "Excluded file pattern must not be empty"
        );
        assert!(
            !justification.is_empty(),
            "Justification for '{}' must not be empty",
            pattern
        );
    }
}
