//! Purpose: Enforce exit-code documentation consistency with the CLI contract.
//! Responsibilities: Verify docs mention the same exit codes exposed by `splunk-cli`.
//! Scope: `docs/containers.md` exit-code reference table.
//! Usage: Runs via `cargo test -p architecture-tests` and `make ci`.
//! Invariants/Assumptions: `crates/cli/src/error.rs` is the source-of-truth for CLI exit codes.

use std::fs;
use std::path::PathBuf;

#[test]
fn containers_doc_exit_codes_match_cli_contract() {
    let workspace_root = find_workspace_root();
    let doc_path = workspace_root.join("docs/containers.md");

    let content = fs::read_to_string(&doc_path)
        .unwrap_or_else(|e| panic!("Failed reading {}: {}", doc_path.display(), e));

    let expected_rows = [
        "| 0 | Success |",
        "| 1 | General error |",
        "| 2 | Authentication failure |",
        "| 3 | Connection error |",
        "| 4 | Resource not found |",
        "| 5 | Validation error |",
        "| 6 | Permission denied |",
        "| 7 | Rate limited |",
        "| 8 | Service unavailable |",
        "| 130 | Interrupted (Ctrl+C) |",
    ];

    for row in expected_rows {
        assert!(
            content.contains(row),
            "Missing exit-code row in docs/containers.md: {}",
            row
        );
    }

    // Guard against old/incorrect mapping that previously drifted.
    let stale_rows = ["| 2 | Invalid arguments |", "| 5 | Timeout |"];

    for stale in stale_rows {
        assert!(
            !content.contains(stale),
            "Found stale exit-code row in docs/containers.md: {}",
            stale
        );
    }
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
