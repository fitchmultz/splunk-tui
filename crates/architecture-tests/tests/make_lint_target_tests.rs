//! Architecture tests for the Makefile `lint` target contract.
//!
//! Validates that the `lint:` target in the workspace root Makefile follows
//! the expected two-phase clippy pattern to ensure warnings are properly
//! enforced (not silently ignored via `cargo clippy --fix`'s exit code behavior).
//!
//! # What This Test Validates
//!
//! - There is a `cargo clippy` **autofix** phase containing `--fix` and the
//!   shared flags (`--workspace`, `--all-targets`, `--all-features`, `--locked`).
//! - The autofix phase does **not** contain `-D warnings` (it should be best-effort,
//!   not the enforcement gate).
//! - There is a subsequent `cargo clippy` **check** phase (no `--fix`) containing
//!   `-- -D warnings` and the shared flags.
//! - There is a `cargo fmt --all --check` line after the strict clippy check.
//!
//! # What This Test Does NOT Do
//!
//! - It does NOT run `cargo clippy` or validate actual clippy output.
//! - It does NOT execute the Makefile or test the behavior of the commands.
//!
//! # Assumptions
//!
//! - The Makefile uses tab-indented recipe lines (standard Make syntax).
//! - The lint target is named exactly `lint:`.
//! - The workspace root Cargo.toml contains `[workspace]` marker.

use std::fs;
use std::path::PathBuf;

/// Shared clippy flags expected in both phases
const SHARED_CLIPPY_FLAGS: &[&str] =
    &["--workspace", "--all-targets", "--all-features", "--locked"];

/// Test that the Makefile lint target follows the two-phase clippy contract.
///
/// # Failure Conditions
///
/// This test fails if any of the following are not satisfied:
/// - The autofix phase does not contain `--fix`
/// - The autofix phase contains `-D warnings` (it shouldn't fail on warnings)
/// - The strict check phase contains `--fix` (it shouldn't modify code)
/// - The strict check phase does not contain `-D warnings`
/// - Format check is not present after the clippy phases
#[test]
fn makefile_lint_target_contract() {
    let workspace_root = find_workspace_root();
    let makefile_path = workspace_root.join("Makefile");

    assert!(
        makefile_path.exists(),
        "Makefile not found at {:?}",
        makefile_path
    );

    let makefile_content = fs::read_to_string(&makefile_path).expect("Failed to read Makefile");

    // Parse the lint target recipe lines
    let lint_lines = extract_lint_target_lines(&makefile_content);

    assert!(
        !lint_lines.is_empty(),
        "Could not find 'lint:' target in Makefile, or target has no recipe lines\n\
         Expected: A target starting with 'lint:' followed by tab-indented recipe lines"
    );

    // Find the clippy autofix line (contains --fix)
    let autofix_line = lint_lines
        .iter()
        .find(|line| line.contains("cargo clippy") && line.contains("--fix"))
        .copied();

    // Find the clippy strict check line (contains -D warnings, no --fix)
    let strict_check_line = lint_lines
        .iter()
        .find(|line| {
            line.contains("cargo clippy") && !line.contains("--fix") && line.contains("-D warnings")
        })
        .copied();

    // Find the format check line
    let format_check_line = lint_lines
        .iter()
        .find(|line| line.contains("cargo fmt") && line.contains("--check"))
        .copied();

    // Validate autofix phase
    if let Some(line) = autofix_line {
        // Must have shared flags
        for flag in SHARED_CLIPPY_FLAGS {
            assert!(
                line.contains(flag),
                "Makefile 'lint' target: autofix phase (cargo clippy --fix) is missing required flag '{}'\n\
                 Found line: {}\n\
                 Expected: cargo clippy --fix --allow-dirty --workspace --all-targets --all-features --locked",
                flag,
                line
            );
        }

        // Must NOT have -D warnings
        assert!(
            !line.contains("-D warnings"),
            "Makefile 'lint' target: autofix phase should NOT contain '-D warnings'\n\
             The autofix phase is best-effort and should not fail on warnings.\n\
             Found line: {}\n\
             Expected: cargo clippy --fix --allow-dirty --workspace --all-targets --all-features --locked (without -D warnings)",
            line
        );
    } else {
        panic!(
            "Makefile 'lint' target: missing clippy autofix phase\n\
             Expected a line with: cargo clippy --fix --allow-dirty --workspace --all-targets --all-features --locked\n\
             Found lines:\n{}",
            lint_lines.join("\n")
        );
    }

    // Validate strict check phase
    if let Some(line) = strict_check_line {
        // Must have shared flags
        for flag in SHARED_CLIPPY_FLAGS {
            assert!(
                line.contains(flag),
                "Makefile 'lint' target: strict check phase is missing required flag '{}'\n\
                 Found line: {}\n\
                 Expected: cargo clippy --workspace --all-targets --all-features --locked -- -D warnings",
                flag,
                line
            );
        }

        // Must NOT have --fix
        assert!(
            !line.contains("--fix"),
            "Makefile 'lint' target: strict check phase should NOT contain '--fix'\n\
             The strict check phase must only validate, not modify code.\n\
             Found line: {}\n\
             Expected: cargo clippy --workspace --all-targets --all-features --locked -- -D warnings",
            line
        );
    } else {
        panic!(
            "Makefile 'lint' target: missing clippy strict check phase\n\
             Expected a line with: cargo clippy --workspace --all-targets --all-features --locked -- -D warnings\n\
             (This phase must run AFTER the autofix phase and WITHOUT --fix)\n\
             Found lines:\n{}",
            lint_lines.join("\n")
        );
    }

    // Validate format check is present
    if let Some(line) = format_check_line {
        assert!(
            line.contains("--all"),
            "Makefile 'lint' target: format check should use '--all' flag\n\
             Found line: {}",
            line
        );
    } else {
        panic!(
            "Makefile 'lint' target: missing format check phase\n\
             Expected a line with: cargo fmt --all --check\n\
             Found lines:\n{}",
            lint_lines.join("\n")
        );
    }

    // Validate ordering: autofix comes before strict check
    let autofix_index = lint_lines
        .iter()
        .position(|line| line.contains("cargo clippy") && line.contains("--fix"));
    let strict_check_index = lint_lines.iter().position(|line| {
        line.contains("cargo clippy") && !line.contains("--fix") && line.contains("-D warnings")
    });

    match (autofix_index, strict_check_index) {
        (Some(ai), Some(si)) => {
            assert!(
                ai < si,
                "Makefile 'lint' target: autofix phase must come BEFORE strict check phase\n\
                 Autofix is at line index {}, strict check is at line index {}",
                ai,
                si
            );
        }
        _ => {
            // Should have already panicked above, but just in case
            panic!("Could not determine ordering of clippy phases");
        }
    }

    // Validate ordering: strict check comes before format check
    let format_check_index = lint_lines
        .iter()
        .position(|line| line.contains("cargo fmt") && line.contains("--check"));

    match (strict_check_index, format_check_index) {
        (Some(si), Some(fi)) => {
            assert!(
                si < fi,
                "Makefile 'lint' target: strict check phase must come BEFORE format check phase\n\
                 Strict check is at line index {}, format check is at line index {}",
                si,
                fi
            );
        }
        _ => {
            panic!("Could not determine ordering of lint phases");
        }
    }

    eprintln!("[architecture] Makefile 'lint' target contract validated successfully");
}

/// Extract the recipe lines for the `lint:` target from Makefile content.
///
/// Returns the tab-indented lines following `lint:` until the next non-recipe line
/// (empty line, comment, or new target definition).
fn extract_lint_target_lines(makefile_content: &str) -> Vec<&str> {
    let lines = makefile_content.lines().peekable();
    let mut lint_lines = Vec::new();
    let mut in_lint_target = false;

    for line in lines {
        let trimmed = line.trim();

        // Check if this is the start of the lint target
        if trimmed == "lint:" || trimmed.starts_with("lint:") && !trimmed.starts_with("lint-") {
            in_lint_target = true;
            continue;
        }

        if in_lint_target {
            // Recipe lines start with a tab character
            if line.starts_with('\t') {
                lint_lines.push(line.trim_start_matches('\t'));
            } else if trimmed.is_empty() {
                // Empty line ends the recipe (but we continue to check if there are more)
                // Actually in Make, an empty line can end a recipe, but let's be lenient
                // and stop at the next target definition (line with : but not a recipe continuation)
                continue;
            } else if trimmed.contains(':') && !trimmed.starts_with('#') {
                // This looks like a new target definition
                break;
            }
            // Other lines (comments, etc.) don't end the recipe but we don't collect them
        }
    }

    lint_lines
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
fn test_extract_lint_target_lines() {
    let makefile = r#"
# Some comment
lint:
	@echo "→ Clippy autofix (phase 1/2)..."
	@cargo clippy --fix --allow-dirty --workspace --all-targets --all-features --locked
	@echo "→ Clippy strict check (phase 2/2)..."
	@cargo clippy --workspace --all-targets --all-features --locked -- -D warnings
	@echo "→ Format check..."
	@cargo fmt --all --check
	@echo "  ✓ Lint complete"

other-target:
	@echo "Other"
"#;

    let lines = extract_lint_target_lines(makefile);
    assert_eq!(lines.len(), 7);
    assert!(lines[0].contains("Clippy autofix"));
    assert!(lines[1].contains("cargo clippy") && lines[1].contains("--fix"));
    assert!(lines[2].contains("strict check"));
    assert!(lines[3].contains("cargo clippy") && lines[3].contains("-D warnings"));
    assert!(lines[4].contains("Format check"));
    assert!(lines[5].contains("cargo fmt"));
    assert!(lines[6].contains("Lint complete"));
}

#[test]
fn test_extract_lint_target_lines_not_found() {
    let makefile = r#"
format:
	@cargo fmt
"#;

    let lines = extract_lint_target_lines(makefile);
    assert!(lines.is_empty());
}
