//! Integration tests for keybinding override functionality.
//!
//! These tests verify that user-defined keybinding overrides work correctly
//! and take precedence over default bindings.

use std::collections::BTreeMap;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use splunk_config::{KeybindAction, KeybindOverrides};
use splunk_tui::action::Action;
use splunk_tui::app::CurrentScreen;
use splunk_tui::input::keymap::{overrides, resolve_action};

fn char_key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

fn ctrl_key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

fn f_key(n: u8) -> KeyEvent {
    KeyEvent::new(KeyCode::F(n), KeyModifiers::NONE)
}

fn shift_tab_key() -> KeyEvent {
    KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE)
}

/// Helper to reset the override table between tests.
/// Note: This only works in tests because we can re-initialize the OnceLock
/// by using std::mem::take pattern (not actually possible with OnceLock).
/// Instead, we test the table logic directly without using the global state.

#[test]
fn test_default_quit_key_without_overrides() {
    // Without any overrides initialized, 'q' should resolve to Quit
    let action = resolve_action(CurrentScreen::Jobs, char_key('q'));
    assert!(
        matches!(action, Some(Action::Quit)),
        "'q' should resolve to Quit without overrides"
    );
}

#[test]
fn test_default_help_key_without_overrides() {
    // Without any overrides initialized, '?' should resolve to OpenHelpPopup
    let action = resolve_action(CurrentScreen::Jobs, char_key('?'));
    assert!(
        matches!(action, Some(Action::OpenHelpPopup)),
        "'?' should resolve to OpenHelpPopup without overrides"
    );
}

#[test]
fn test_default_next_screen_key_without_overrides() {
    // Without any overrides initialized, Tab should resolve to NextScreen
    let action = resolve_action(
        CurrentScreen::Jobs,
        KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE),
    );
    assert!(
        matches!(action, Some(Action::NextScreen)),
        "Tab should resolve to NextScreen without overrides"
    );
}

#[test]
fn test_default_previous_screen_key_without_overrides() {
    // Without any overrides initialized, Shift+Tab should resolve to PreviousScreen
    let action = resolve_action(CurrentScreen::Jobs, shift_tab_key());
    assert!(
        matches!(action, Some(Action::PreviousScreen)),
        "Shift+Tab should resolve to PreviousScreen without overrides"
    );
}

#[test]
fn test_override_table_quit_key() {
    let mut map = BTreeMap::new();
    map.insert(KeybindAction::Quit, "F10".to_string());
    let overrides = KeybindOverrides { overrides: map };

    let table = overrides::KeybindOverrideTable::from_overrides(&overrides).unwrap();

    // F10 should now resolve to Quit
    let action = table.resolve(f_key(10));
    assert!(
        matches!(action, Some(Action::Quit)),
        "F10 should resolve to Quit with override"
    );

    // 'q' should NOT resolve to Quit anymore (it's not in the override table)
    assert!(
        table.resolve(char_key('q')).is_none(),
        "'q' should not be in override table"
    );
}

#[test]
fn test_override_table_help_key() {
    let mut map = BTreeMap::new();
    map.insert(KeybindAction::Help, "F1".to_string());
    let overrides = KeybindOverrides { overrides: map };

    let table = overrides::KeybindOverrideTable::from_overrides(&overrides).unwrap();

    // F1 should resolve to OpenHelpPopup
    let action = table.resolve(f_key(1));
    assert!(
        matches!(action, Some(Action::OpenHelpPopup)),
        "F1 should resolve to OpenHelpPopup with override"
    );
}

#[test]
fn test_override_table_next_screen_key() {
    let mut map = BTreeMap::new();
    map.insert(KeybindAction::NextScreen, "Ctrl+n".to_string());
    let overrides = KeybindOverrides { overrides: map };

    let table = overrides::KeybindOverrideTable::from_overrides(&overrides).unwrap();

    // Ctrl+n should resolve to NextScreen
    let action = table.resolve(ctrl_key('n'));
    assert!(
        matches!(action, Some(Action::NextScreen)),
        "Ctrl+n should resolve to NextScreen with override"
    );
}

#[test]
fn test_override_table_previous_screen_key() {
    let mut map = BTreeMap::new();
    map.insert(KeybindAction::PreviousScreen, "Ctrl+p".to_string());
    let overrides = KeybindOverrides { overrides: map };

    let table = overrides::KeybindOverrideTable::from_overrides(&overrides).unwrap();

    // Ctrl+p should resolve to PreviousScreen
    let action = table.resolve(ctrl_key('p'));
    assert!(
        matches!(action, Some(Action::PreviousScreen)),
        "Ctrl+p should resolve to PreviousScreen with override"
    );
}

#[test]
fn test_override_table_multiple_bindings() {
    let mut map = BTreeMap::new();
    map.insert(KeybindAction::Quit, "Ctrl+x".to_string());
    map.insert(KeybindAction::Help, "F1".to_string());
    map.insert(KeybindAction::NextScreen, "Ctrl+n".to_string());
    map.insert(KeybindAction::PreviousScreen, "Ctrl+p".to_string());
    let overrides = KeybindOverrides { overrides: map };

    let table = overrides::KeybindOverrideTable::from_overrides(&overrides).unwrap();

    // All overrides should work
    assert!(matches!(table.resolve(ctrl_key('x')), Some(Action::Quit)));
    assert!(matches!(
        table.resolve(f_key(1)),
        Some(Action::OpenHelpPopup)
    ));
    assert!(matches!(
        table.resolve(ctrl_key('n')),
        Some(Action::NextScreen)
    ));
    assert!(matches!(
        table.resolve(ctrl_key('p')),
        Some(Action::PreviousScreen)
    ));
}

#[test]
fn test_override_table_empty() {
    let overrides = KeybindOverrides::default();
    let table = overrides::KeybindOverrideTable::from_overrides(&overrides).unwrap();

    assert!(table.is_empty());
    assert_eq!(table.len(), 0);
}

#[test]
fn test_override_table_invalid_key_returns_error() {
    let mut map = BTreeMap::new();
    map.insert(KeybindAction::Quit, "InvalidKey".to_string());
    let overrides = KeybindOverrides { overrides: map };

    let result = overrides::KeybindOverrideTable::from_overrides(&overrides);
    assert!(result.is_err(), "Invalid key should return an error");
}

#[test]
fn test_keybind_overrides_serialization() {
    let mut map = BTreeMap::new();
    map.insert(KeybindAction::Quit, "Ctrl+x".to_string());
    map.insert(KeybindAction::Help, "F1".to_string());
    let overrides = KeybindOverrides { overrides: map };

    let json = serde_json::to_string(&overrides).unwrap();

    // Should contain the overrides
    assert!(json.contains("quit"));
    assert!(json.contains("Ctrl+x"));
    assert!(json.contains("help"));
    assert!(json.contains("F1"));

    // Deserialize and verify
    let deserialized: KeybindOverrides = serde_json::from_str(&json).unwrap();
    assert_eq!(deserialized.get(KeybindAction::Quit), Some("Ctrl+x"));
    assert_eq!(deserialized.get(KeybindAction::Help), Some("F1"));
}

#[test]
fn test_keybind_overrides_deserialization_from_json() {
    let json = r#"{"overrides":{"quit":"Ctrl+x","help":"F1"}}"#;
    let overrides: KeybindOverrides = serde_json::from_str(json).unwrap();

    assert_eq!(overrides.get(KeybindAction::Quit), Some("Ctrl+x"));
    assert_eq!(overrides.get(KeybindAction::Help), Some("F1"));
}

#[test]
fn test_keybind_overrides_default_is_empty() {
    let overrides = KeybindOverrides::default();
    assert!(overrides.is_empty());
    assert_eq!(overrides.get(KeybindAction::Quit), None);
    assert_eq!(overrides.get(KeybindAction::Help), None);
}

#[test]
fn test_keybind_action_display() {
    assert_eq!(format!("{}", KeybindAction::Quit), "quit");
    assert_eq!(format!("{}", KeybindAction::Help), "help");
    assert_eq!(format!("{}", KeybindAction::NextScreen), "next_screen");
    assert_eq!(
        format!("{}", KeybindAction::PreviousScreen),
        "previous_screen"
    );
}

#[test]
fn test_keybind_action_serde_round_trip() {
    // Test that KeybindAction serializes and deserializes correctly
    let actions = vec![
        KeybindAction::Quit,
        KeybindAction::Help,
        KeybindAction::NextScreen,
        KeybindAction::PreviousScreen,
    ];

    for action in actions {
        let json = serde_json::to_string(&action).unwrap();
        let deserialized: KeybindAction = serde_json::from_str(&json).unwrap();
        assert_eq!(action, deserialized);
    }
}

#[test]
fn test_keybind_action_snake_case_serialization() {
    // Verify snake_case serialization
    assert_eq!(
        serde_json::to_string(&KeybindAction::Quit).unwrap(),
        "\"quit\""
    );
    assert_eq!(
        serde_json::to_string(&KeybindAction::Help).unwrap(),
        "\"help\""
    );
    assert_eq!(
        serde_json::to_string(&KeybindAction::NextScreen).unwrap(),
        "\"next_screen\""
    );
    assert_eq!(
        serde_json::to_string(&KeybindAction::PreviousScreen).unwrap(),
        "\"previous_screen\""
    );
}

// Note: Override display in footer tests would require initializing the global
// override table, which can only be done once. These tests verify the table
// logic directly without using the global state.

#[test]
fn test_override_table_provides_display_key_for_action() {
    // Verify that the override table can provide the key string for an action
    let mut map = BTreeMap::new();
    map.insert(KeybindAction::Quit, "Ctrl+x".to_string());
    map.insert(KeybindAction::NextScreen, "Ctrl+n".to_string());
    let overrides = KeybindOverrides { overrides: map };

    let table = overrides::KeybindOverrideTable::from_overrides(&overrides).unwrap();

    // The table should resolve the overridden keys to actions
    assert!(matches!(table.resolve(ctrl_key('x')), Some(Action::Quit)));
    assert!(matches!(
        table.resolve(ctrl_key('n')),
        Some(Action::NextScreen)
    ));

    // Non-overridden keys should not resolve
    assert!(table.resolve(char_key('q')).is_none());
    assert!(
        table
            .resolve(KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE))
            .is_none()
    );
}

#[test]
fn test_override_table_empty_returns_none_for_display() {
    let overrides = KeybindOverrides::default();
    let table = overrides::KeybindOverrideTable::from_overrides(&overrides).unwrap();

    assert!(table.is_empty());
}
