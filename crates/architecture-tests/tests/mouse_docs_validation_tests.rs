//! Architecture tests for mouse documentation validation.
//!
//! Purpose: Prevent drift between mouse support documentation and actual
//! implementation in crates/tui/src/app/mouse.rs.
//!
//! What This Tests:
//! - Mouse Support section exists in docs/user-guide.md
//! - Documentation mentions correct features (selection, click-twice inspect, scrolling, quit)
//! - Documentation correctly states navigation is keyboard-only
//! - Documentation uses "click twice" not "double-click" (implementation has no timing)
//!
//! What This Does NOT Do:
//! - Does NOT run the TUI or test actual mouse behavior
//! - Does NOT validate implementation correctness (unit tests in mouse.rs do that)

use std::fs;
use std::path::PathBuf;

const EXPECTED_MOUSE_FEATURES: &[&str] = &["Selection", "Inspect", "Scrolling", "Quit"];

const KEYBOARD_ONLY_PHRASES: &[&str] = &["keyboard-only", "Tab"];

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

fn get_mouse_docs_section() -> String {
    let workspace_root = find_workspace_root();
    let user_guide_path = workspace_root.join("docs/user-guide.md");

    let content = fs::read_to_string(&user_guide_path).expect("Failed to read docs/user-guide.md");

    // Extract Mouse Support section (between "### Mouse Support" and next ## or ###)
    let mut in_mouse_section = false;
    let mut section_lines = Vec::new();

    for line in content.lines() {
        if line.starts_with("### Mouse Support") {
            in_mouse_section = true;
            continue;
        }
        if in_mouse_section {
            if line.starts_with("### ") || line.starts_with("## ") {
                break;
            }
            section_lines.push(line);
        }
    }

    section_lines.join("\n")
}

#[test]
fn mouse_docs_section_exists() {
    let section = get_mouse_docs_section();
    assert!(
        !section.is_empty(),
        "Mouse Support section not found in docs/user-guide.md"
    );
}

#[test]
fn mouse_docs_lists_expected_features() {
    let section = get_mouse_docs_section();

    for feature in EXPECTED_MOUSE_FEATURES {
        assert!(
            section.contains(feature),
            "Mouse Support section missing expected feature: {}\nSection content:\n{}",
            feature,
            section
        );
    }
}

#[test]
fn mouse_docs_states_keyboard_only_navigation() {
    let section = get_mouse_docs_section();

    let has_keyboard_note = KEYBOARD_ONLY_PHRASES
        .iter()
        .all(|phrase| section.to_lowercase().contains(&phrase.to_lowercase()));

    assert!(
        has_keyboard_note,
        "Mouse Support section should state that screen navigation is keyboard-only\n\
         Expected phrases: {:?}\n\
         Section content:\n{}",
        KEYBOARD_ONLY_PHRASES, section
    );
}

#[test]
fn mouse_docs_uses_click_twice_not_double_click() {
    let section = get_mouse_docs_section();

    // Should contain "click" and "twice" or "same" for inspect behavior
    let has_click_twice =
        section.to_lowercase().contains("twice") || section.to_lowercase().contains("same");

    // Should NOT contain "double-click" as implementation has no timing
    let has_double_click = section.to_lowercase().contains("double-click");

    assert!(
        has_click_twice && !has_double_click,
        "Mouse Support section should describe inspect as 'click twice' or 'same row', not 'double-click'\n\
         (Implementation has no double-click timing semantics)\n\
         Section content:\n{}",
        section
    );
}

#[test]
fn mouse_docs_does_not_claim_footer_navigation_clicks() {
    let section = get_mouse_docs_section();
    let section_lower = section.to_lowercase();

    // Should NOT claim footer clicks for navigation
    let claims_footer_nav = section_lower.contains("footer")
        && (section_lower.contains("tab") || section_lower.contains("switch"))
        && !section_lower.contains("quit");

    assert!(
        !claims_footer_nav,
        "Mouse Support section should NOT claim footer clicks for tab switching\n\
         (handle_footer_click only supports quit button)\n\
         Section content:\n{}",
        section
    );
}
