//! Drift detection tests for tutorial keybinding content.
//!
//! These tests verify that tutorial content matches the keymap catalog,
//! preventing guidance drift where tutorial teaches different keys than runtime.

use splunk_tui::action::Action;
use splunk_tui::input::keymap::overrides::get_effective_key_display;
use splunk_tui::onboarding::generate_keybinding_section;

/// Test that tutorial mentions screen navigation with correct keys
#[test]
fn tutorial_screen_navigation_matches_keymap() {
    let tutorial = generate_keybinding_section();

    let next_key = get_effective_key_display(Action::NextScreen, "Tab");
    let prev_key = get_effective_key_display(Action::PreviousScreen, "Shift+Tab");

    assert!(
        tutorial.contains(&next_key) || tutorial.contains("Switch between screens"),
        "Tutorial should mention '{}' or explain screen navigation",
        next_key
    );
    assert!(
        tutorial.contains(&prev_key),
        "Tutorial should mention previous screen key '{}'",
        prev_key
    );
}

/// Test that tutorial mentions focus navigation with correct keys
#[test]
fn tutorial_focus_navigation_matches_keymap() {
    let tutorial = generate_keybinding_section();

    let next_focus = get_effective_key_display(Action::NextFocus, "Ctrl+Tab");
    let prev_focus = get_effective_key_display(Action::PreviousFocus, "Ctrl+Shift+Tab");

    assert!(
        tutorial.contains(&next_focus) || tutorial.contains("Cycle focus"),
        "Tutorial should mention '{}' or explain focus navigation",
        next_focus
    );
    assert!(
        tutorial.contains(&prev_focus),
        "Tutorial should mention previous focus key '{}'",
        prev_focus
    );
}

/// Test that tutorial does NOT claim Tab cycles elements (the old incorrect claim)
#[test]
fn tutorial_does_not_claim_tab_cycles_elements() {
    let tutorial = generate_keybinding_section();

    assert!(
        !tutorial.contains("Cycle between screen elements"),
        "Tutorial should NOT claim Tab cycles elements (it switches screens)"
    );
}

/// Test that tutorial does NOT claim arrow/h/l navigate screens
#[test]
fn tutorial_does_not_claim_arrows_navigate_screens() {
    let tutorial = generate_keybinding_section();

    assert!(
        !tutorial.contains("←/→ or h/l"),
        "Tutorial should NOT claim arrow keys navigate screens (Tab does that)"
    );

    assert!(
        !tutorial.contains("h/l     Navigate between screens"),
        "Tutorial should NOT associate h/l with screen navigation"
    );
}

/// Test that tutorial quit key matches keymap
#[test]
fn tutorial_quit_key_matches_keymap() {
    let tutorial = generate_keybinding_section();
    let quit_key = get_effective_key_display(Action::Quit, "q");

    assert!(
        tutorial.contains(&quit_key),
        "Tutorial should show correct quit key '{}'",
        quit_key
    );
}

/// Test that tutorial help key matches keymap
#[test]
fn tutorial_help_key_matches_keymap() {
    let tutorial = generate_keybinding_section();
    let help_key = get_effective_key_display(Action::OpenHelpPopup, "?");

    assert!(
        tutorial.contains(&help_key),
        "Tutorial should show correct help key '{}'",
        help_key
    );
}

/// Test that tutorial content is deterministically generated
#[test]
fn tutorial_content_is_deterministic() {
    let first = generate_keybinding_section();
    let second = generate_keybinding_section();
    assert_eq!(first, second, "Tutorial content must be deterministic");
}

/// Test that tutorial correctly describes what Tab does
#[test]
fn tutorial_correctly_describes_tab_function() {
    let tutorial = generate_keybinding_section();

    assert!(
        tutorial.contains("Switch between screens"),
        "Tutorial should describe Tab as switching screens, not cycling elements"
    );
}

/// Test that tutorial correctly describes focus navigation
#[test]
fn tutorial_correctly_describes_focus_navigation() {
    let tutorial = generate_keybinding_section();

    assert!(
        tutorial.contains("Cycle focus between elements"),
        "Tutorial should describe Ctrl+Tab as cycling focus between elements"
    );
}
