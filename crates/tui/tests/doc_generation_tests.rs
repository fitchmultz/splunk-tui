//! Integration tests for TUI documentation generation.
//!
//! These tests ensure that the documentation generated for TUI keybindings
//! remains deterministic and correctly reflects the current keymap.

use splunk_tui::render_tui_keybinding_docs;

#[test]
fn keybinding_docs_render_is_deterministic() {
    let first = render_tui_keybinding_docs();
    let second = render_tui_keybinding_docs();
    assert_eq!(
        first, second,
        "Documentation generation must be deterministic"
    );
}

#[test]
fn keybinding_docs_render_contains_expected_structure() {
    let docs = render_tui_keybinding_docs();

    // Check for major sections
    assert!(
        docs.contains("### Navigation"),
        "Docs should have a Navigation section"
    );
    assert!(
        docs.contains("### Screen Specific Shortcuts"),
        "Docs should have a Screen Specific Shortcuts section"
    );

    // Check for some common keybindings that are unlikely to change
    assert!(
        docs.contains("- `?`: Help"),
        "Docs should include help keybinding"
    );
    assert!(
        docs.contains("- `q`: Quit"),
        "Docs should include quit keybinding"
    );
    assert!(
        docs.contains("- `Enter`: Run search"),
        "Docs should include search execution"
    );

    // Check for formatting
    assert!(
        docs.starts_with("### Navigation"),
        "Docs should start with navigation section"
    );
    assert!(!docs.ends_with('\n'), "Docs should be trimmed at the end");
}

#[test]
fn keybinding_docs_render_contains_all_screens() {
    let docs = render_tui_keybinding_docs();

    let expected_screens = [
        "Search Screen",
        "Jobs Screen",
        "Indexes Screen",
        "Cluster Screen",
        "Health Screen",
        "Saved Searches Screen",
        "Internal Logs Screen",
        "Apps Screen",
        "Users Screen",
        "Settings Screen",
        "Data Models Screen",
        "Workload Management Screen",
        "SHC Screen",
    ];

    for screen in expected_screens {
        assert!(
            docs.contains(&format!("#### {}", screen)),
            "Docs should include section for {}",
            screen
        );
    }
}
