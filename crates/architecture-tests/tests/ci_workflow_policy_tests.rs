//! Purpose: Enforce CI workflow policy separation between fast PR gate and full validation gate.
//! Responsibilities: Validate `.github/workflows/ci.yml` and `.github/workflows/ci-full.yml` stay aligned with repo CI strategy.
//! Scope: Workflow trigger intent and Makefile target mapping.
//! Usage: Runs via `cargo test -p architecture-tests` and `make ci`.
//! Invariants/Assumptions: Fast workflow must run `make ci-fast`; full workflow must run `make ci`.

use std::fs;
use std::path::PathBuf;

#[test]
fn fast_ci_workflow_uses_fast_gate_only() {
    let workspace_root = find_workspace_root();
    let workflow_path = workspace_root.join(".github/workflows/ci.yml");

    let content = fs::read_to_string(&workflow_path)
        .unwrap_or_else(|e| panic!("Failed reading {}: {}", workflow_path.display(), e));

    assert!(
        content.contains("pull_request:"),
        "Fast CI workflow must trigger on pull_request"
    );
    assert!(
        content.contains("run: make ci-fast"),
        "Fast CI workflow must run `make ci-fast`"
    );

    assert!(
        !content.contains("branches: [main]"),
        "Fast CI workflow should not run push-to-main full validation"
    );
}

#[test]
fn full_ci_workflow_runs_full_gate_on_main_and_nightly() {
    let workspace_root = find_workspace_root();
    let workflow_path = workspace_root.join(".github/workflows/ci-full.yml");

    let content = fs::read_to_string(&workflow_path)
        .unwrap_or_else(|e| panic!("Failed reading {}: {}", workflow_path.display(), e));

    assert!(
        content.contains("push:") && content.contains("branches: [main]"),
        "Full CI workflow must include push-to-main trigger"
    );
    assert!(
        content.contains("schedule:"),
        "Full CI workflow must include scheduled nightly trigger"
    );
    assert!(
        content.contains("workflow_dispatch:"),
        "Full CI workflow must include manual trigger"
    );
    assert!(
        content.contains("run: make ci"),
        "Full CI workflow must run `make ci`"
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
