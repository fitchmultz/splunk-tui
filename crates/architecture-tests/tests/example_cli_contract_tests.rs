//! Purpose: Catch stale examples and docs that drift away from the current splunk-cli contract.
//! Responsibilities: Scan shipped shell examples and public markdown for retired flags and brittle command patterns.
//! Scope: `examples/**/*.sh`, `README.md`, `docs/**/*.md`, and `examples/**/*.md`.
//! Usage: Runs with `cargo test -p architecture-tests` and as part of `make ci`.
//! Invariants/Assumptions: Public examples should use the same CLI surface exposed by the current binaries.

use std::fs;
use std::path::{Path, PathBuf};

use walkdir::WalkDir;

#[test]
fn example_scripts_do_not_use_known_stale_cli_patterns() {
    let workspace_root = find_workspace_root();
    let examples_root = workspace_root.join("examples");

    let forbidden_patterns = [
        ("--output-format", "use the global `--output` flag instead"),
        (
            "search \"$search_query\" --limit",
            "use `--count` for search result limits",
        ),
        ("alerts list --limit", "use `--count` for alerts pagination"),
        (
            "jobs --results \"$sid\" --format",
            "use the global `--output` flag for job results",
        ),
        (
            "logs --search",
            "the logs command only supports `--earliest`, `--count`, and `--tail` filters",
        ),
        (
            "saved-searches list 2>/dev/null | grep -q",
            "table output is not stable for existence checks; use `saved-searches info` instead",
        ),
    ];

    let mut violations = Vec::new();

    for script in find_shell_scripts(&examples_root) {
        let content = fs::read_to_string(&script)
            .unwrap_or_else(|e| panic!("Failed reading script {:?}: {}", script, e));

        for (pattern, guidance) in forbidden_patterns {
            if content.contains(pattern) {
                violations.push(format!(
                    "{} contains `{}` ({})",
                    relative_to_workspace(&script, &workspace_root),
                    pattern,
                    guidance
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Found stale splunk-cli usage in example scripts:\n{}",
        violations.join("\n")
    );
}

#[test]
fn markdown_examples_do_not_use_known_stale_cli_patterns() {
    let workspace_root = find_workspace_root();

    let mut markdown_files = vec![workspace_root.join("README.md")];
    markdown_files.extend(find_files_with_extension(
        &workspace_root.join("docs"),
        "md",
    ));
    markdown_files.extend(find_files_with_extension(
        &workspace_root.join("examples"),
        "md",
    ));

    let forbidden_patterns = [
        ("--output-format", "use the global `--output` flag instead"),
        ("alerts list --limit", "use `--count` for alerts pagination"),
        (
            "splunk-cli jobs --results 1705852800.123 --format",
            "use the global `--output` flag for job results",
        ),
        (
            "splunk-cli search \"index=main\" --limit",
            "use `--count` for search result limits",
        ),
        (
            "splunk-cli logs --search",
            "the logs command only supports `--earliest`, `--count`, and `--tail` filters",
        ),
        (
            "./scheduled-reports.sh --report-list",
            "`scheduled-reports.sh` only supports one `--report` at a time",
        ),
        (
            "./scheduled-reports.sh --search",
            "`scheduled-reports.sh` runs saved searches only; ad-hoc SPL is unsupported",
        ),
        (
            "./scheduled-reports.sh --output-file",
            "`scheduled-reports.sh` manages output paths via `--output-dir`",
        ),
        (
            "./data-onboarding.sh --retention",
            "`data-onboarding.sh` does not support retention configuration",
        ),
        (
            "./data-onboarding.sh --validate",
            "`data-onboarding.sh` validates by default and only supports `--skip-validation`",
        ),
        (
            "delete-lookup-files",
            "`bulk-operations.sh` only supports saved-search operations",
        ),
    ];

    let mut violations = Vec::new();

    for file in markdown_files {
        if !file.exists() {
            continue;
        }

        let content = fs::read_to_string(&file)
            .unwrap_or_else(|e| panic!("Failed reading markdown file {:?}: {}", file, e));

        for (pattern, guidance) in forbidden_patterns {
            if content.contains(pattern) {
                violations.push(format!(
                    "{} contains `{}` ({})",
                    relative_to_workspace(&file, &workspace_root),
                    pattern,
                    guidance
                ));
            }
        }
    }

    assert!(
        violations.is_empty(),
        "Found stale splunk-cli usage in markdown examples:\n{}",
        violations.join("\n")
    );
}

fn find_shell_scripts(root: &Path) -> Vec<PathBuf> {
    find_files_with_extension(root, "sh")
}

fn find_files_with_extension(root: &Path, extension: &str) -> Vec<PathBuf> {
    if !root.exists() {
        return Vec::new();
    }

    WalkDir::new(root)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|entry| entry.file_type().is_file())
        .filter(|entry| entry.path().extension().is_some_and(|ext| ext == extension))
        .map(|entry| entry.into_path())
        .collect()
}

fn relative_to_workspace(path: &Path, workspace_root: &Path) -> String {
    path.strip_prefix(workspace_root)
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| path.display().to_string())
}

fn find_workspace_root() -> PathBuf {
    let current_dir = std::env::current_dir().expect("Failed to get current directory");

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
            None => return current_dir,
        }
    }
}
