//! Purpose: Prevent accidental tracking of local-only runtime artifacts and secret-bearing files.
//! Responsibilities: Fail when forbidden paths are committed to git index.
//! Scope: Repository hygiene checks for logs, local agent state, and env files.
//! Usage: Runs with `cargo test -p architecture-tests` and as part of `make ci`.
//! Invariants/Assumptions: Git metadata is available in test environments.

use std::fs;
use std::path::PathBuf;
use std::process::Command;

#[test]
fn forbidden_artifacts_are_not_tracked() {
    let workspace_root = find_workspace_root();

    let output = Command::new("git")
        .arg("ls-files")
        .arg("--")
        .arg("logs/")
        .arg(".ralph/")
        .arg("crates/tui/logs/")
        .arg(".env")
        .arg(".env.test")
        .current_dir(&workspace_root)
        .output()
        .expect("Failed to run git ls-files for forbidden artifact check");

    assert!(
        output.status.success(),
        "git ls-files failed with status {:?}: {}",
        output.status,
        String::from_utf8_lossy(&output.stderr)
    );

    let tracked: Vec<String> = String::from_utf8_lossy(&output.stdout)
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(ToOwned::to_owned)
        .collect();

    assert!(
        tracked.is_empty(),
        "Forbidden tracked artifacts detected:\n{}\n\nRemove with `git rm --cached -- <path>` and re-run checks.",
        tracked.join("\n")
    );
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
