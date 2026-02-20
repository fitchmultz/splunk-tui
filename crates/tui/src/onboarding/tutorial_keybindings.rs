//! Tutorial keybinding rendering from centralized keymap.
//!
//! Responsibilities:
//! - Generate human-readable keybinding descriptions for tutorial content
//! - Respect user keybinding overrides when displaying keys
//!
//! Does NOT handle:
//! - Tutorial state management (handled by state.rs)
//! - UI rendering (handled by UI layer)

use crate::action::Action;
use crate::input::keymap::overrides::get_effective_key_display;

/// Get the effective key display for an action, respecting overrides.
/// Returns the override key if set, otherwise returns the default key from keymap.
pub fn get_key_for_action(action: Action, default_key: &'static str) -> String {
    get_effective_key_display(action, default_key)
}

/// Generate screen navigation keybinding text for tutorial.
/// Returns formatted string like "Tab / Shift+Tab" or override equivalents.
pub fn screen_navigation_keys_text() -> String {
    let next_screen = get_key_for_action(Action::NextScreen, "Tab");
    let prev_screen = get_key_for_action(Action::PreviousScreen, "Shift+Tab");
    format!("{} / {}", next_screen, prev_screen)
}

/// Generate focus navigation keybinding text for tutorial.
pub fn focus_navigation_keys_text() -> String {
    let next_focus = get_key_for_action(Action::NextFocus, "Ctrl+Tab");
    let prev_focus = get_key_for_action(Action::PreviousFocus, "Ctrl+Shift+Tab");
    format!("{} / {}", next_focus, prev_focus)
}

/// Generate quit key text, respecting overrides.
pub fn quit_key_text() -> String {
    get_key_for_action(Action::Quit, "q")
}

/// Generate help key text, respecting overrides.
pub fn help_key_text() -> String {
    get_key_for_action(Action::OpenHelpPopup, "?")
}

/// Generate list navigation keybinding text for tutorial.
/// Returns the keys for GoToTop and GoToBottom actions.
pub fn list_navigation_keys_text() -> (String, String) {
    // These default to "Home" and "End" based on global_search.rs bindings
    let go_top = get_key_for_action(Action::GoToTop, "Home");
    let go_bottom = get_key_for_action(Action::GoToBottom, "End");
    (go_top, go_bottom)
}

/// Generate the keybinding tutorial content section.
/// This is the main entry point for generating tutorial keybinding text.
pub fn generate_keybinding_section() -> String {
    let screen_nav_keys = screen_navigation_keys_text();
    let focus_nav_keys = focus_navigation_keys_text();
    let (go_top_key, go_bottom_key) = list_navigation_keys_text();
    let quit_key = quit_key_text();
    let help_key = help_key_text();

    format!(
        r#"Step 4: Learn the Keybindings

Splunk TUI is designed to be keyboard-driven for efficiency. Here are the essential shortcuts:

Navigation:
  • {screen_nav_keys}  Switch between screens
  • {focus_nav_keys}  Cycle focus between elements
  • {go_top_key:<14} Go to top of list
  • {go_bottom_key:<14} Go to bottom of list

Search:
  • Enter           Execute search
  • Esc             Return to query input
  • Ctrl+c          Cancel running search

Actions:
  • r               Refresh current view
  • e               Export results
  • {help_key:<14} Show help
  • {quit_key:<14} Quit or go back

Screen-specific:
  • p               Profile manager
  • s               Saved searches
  • j               Jobs screen
  • i               Indexes screen

Use ↑/↓ to scroll this help text. Press → to continue."#
    )
}

/// Generate footer hint for tutorial welcome step.
pub fn welcome_footer_hint() -> String {
    let quit_key = quit_key_text();
    format!(
        "Press → or Enter to continue | Press {} to skip tutorial",
        quit_key
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_screen_navigation_keys_format() {
        let text = screen_navigation_keys_text();
        assert!(
            text.contains('/') || text.contains("Tab"),
            "Should show key combination"
        );
    }

    #[test]
    fn test_focus_navigation_keys_format() {
        let text = focus_navigation_keys_text();
        assert!(
            text.contains('/') || text.contains("Tab"),
            "Should show key combination"
        );
    }

    #[test]
    fn test_generate_keybinding_section_contains_expected_keys() {
        let section = generate_keybinding_section();
        assert!(
            section.contains("Switch between screens"),
            "Should contain screen navigation description"
        );
        assert!(
            section.contains("Cycle focus"),
            "Should contain focus navigation description"
        );
    }

    #[test]
    fn test_generate_keybinding_section_does_not_contain_incorrect_claims() {
        let section = generate_keybinding_section();
        assert!(
            !section.contains("Cycle between screen elements"),
            "Should NOT claim Tab cycles elements (it switches screens)"
        );
        assert!(
            !section.contains("←/→ or h/l"),
            "Should NOT claim arrow keys navigate screens (Tab does that)"
        );
        assert!(
            !section.contains("Navigate between screens") || !section.contains("h/l"),
            "Should NOT associate h/l with screen navigation"
        );
        // Check for g/G drift - these should NOT appear for navigation
        assert!(
            !section.contains("g               Go to top"),
            "Should NOT claim 'g' for go to top (Home is bound)"
        );
        assert!(
            !section.contains("G               Go to bottom"),
            "Should NOT claim 'G' for go to bottom (End is bound)"
        );
        // Check for / drift - no / binding exists for search focus
        assert!(
            !section.contains("/               Focus search"),
            "Should NOT claim '/' for search focus (no binding exists)"
        );
    }

    #[test]
    fn test_generate_keybinding_section_is_deterministic() {
        let first = generate_keybinding_section();
        let second = generate_keybinding_section();
        assert_eq!(first, second, "Generated content must be deterministic");
    }

    #[test]
    fn test_quit_key_default() {
        let key = quit_key_text();
        assert_eq!(key, "q", "Default quit key should be 'q'");
    }

    #[test]
    fn test_help_key_default() {
        let key = help_key_text();
        assert_eq!(key, "?", "Default help key should be '?'");
    }

    #[test]
    fn test_welcome_footer_hint_contains_quit_key() {
        let hint = welcome_footer_hint();
        assert!(hint.contains('q'), "Welcome footer should mention quit key");
    }
}
