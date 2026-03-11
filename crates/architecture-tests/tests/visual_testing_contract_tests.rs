//! Purpose: Architecture-level contract checks for visual testing targets in the Makefile.
//! Responsibilities: Ensure visual gates exist and are wired into smoke CI coverage.
//! Scope: Static Makefile parsing only (no command execution).
//! Usage: Runs under `cargo test -p architecture-tests`.
//! Invariants/Assumptions: Makefile keeps target names stable (`tui-visual`, `tui-accessibility`, `test-smoke`).

use std::fs;
use std::path::PathBuf;

#[test]
fn makefile_visual_testing_targets_are_wired_into_smoke_gate() {
    let workspace_root = find_workspace_root();
    let makefile_content = load_makefile_contract_view(&workspace_root);

    let visual_lines = extract_target_recipe(&makefile_content, "tui-visual");
    assert!(
        !visual_lines.is_empty(),
        "Expected Makefile target 'tui-visual:' to exist with recipe lines"
    );
    assert!(
        visual_lines
            .iter()
            .any(|line| line.contains("snapshot_styled_tests")),
        "Expected tui-visual recipe to run snapshot_styled_tests"
    );
    assert!(
        visual_lines
            .iter()
            .any(|line| line.contains("interaction_render_tests")),
        "Expected tui-visual recipe to run interaction_render_tests"
    );

    let accessibility_lines = extract_target_recipe(&makefile_content, "tui-accessibility");
    assert!(
        !accessibility_lines.is_empty(),
        "Expected Makefile target 'tui-accessibility:' to exist with recipe lines"
    );
    assert!(
        accessibility_lines
            .iter()
            .any(|line| line.contains("accessibility_contrast_tests")),
        "Expected tui-accessibility recipe to run accessibility_contrast_tests"
    );

    let smoke_lines = extract_target_recipe(&makefile_content, "test-smoke");
    assert!(
        !smoke_lines.is_empty(),
        "Expected Makefile target 'test-smoke:' to exist with recipe lines"
    );
    assert!(
        smoke_lines
            .iter()
            .any(|line| line.contains("$(MAKE) tui-visual")),
        "Expected test-smoke to include $(MAKE) tui-visual"
    );
    assert!(
        smoke_lines
            .iter()
            .any(|line| line.contains("$(MAKE) tui-accessibility")),
        "Expected test-smoke to include $(MAKE) tui-accessibility"
    );
}

fn load_makefile_contract_view(workspace_root: &std::path::Path) -> String {
    let root_makefile = workspace_root.join("Makefile");
    let tests_makefile = workspace_root.join("mk/tests.mk");
    assert!(
        root_makefile.exists(),
        "Makefile not found at {:?}",
        root_makefile
    );
    assert!(
        tests_makefile.exists(),
        "mk/tests.mk not found at {:?}",
        tests_makefile
    );

    let root = fs::read_to_string(&root_makefile).expect("Failed to read workspace Makefile");
    let tests = fs::read_to_string(&tests_makefile).expect("Failed to read mk/tests.mk");

    format!("{root}\n{tests}")
}

fn extract_target_recipe<'a>(makefile_content: &'a str, target: &str) -> Vec<&'a str> {
    let mut in_target = false;
    let mut recipe = Vec::new();

    for line in makefile_content.lines() {
        let trimmed = line.trim();

        if trimmed == format!("{target}:")
            || (trimmed.starts_with(&format!("{target}:"))
                && !trimmed.starts_with(&(target.to_string() + "-")))
        {
            in_target = true;
            continue;
        }

        if in_target {
            if line.starts_with('\t') {
                recipe.push(line.trim_start_matches('\t'));
            } else if !trimmed.is_empty() && trimmed.contains(':') && !trimmed.starts_with('#') {
                break;
            }
        }
    }

    recipe
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
