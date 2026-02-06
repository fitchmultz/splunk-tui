//! Keybinding override resolution.
//!
//! Bridges the config crate's KeybindOverrides with the TUI's crossterm-based
//! keymap system. Converts parsed key strings into crossterm KeyEvents for matching.
//!
//! Responsibilities:
//! - Convert config keybinding strings into crossterm KeyEvents.
//! - Build a lookup table for fast override resolution at runtime.
//! - Provide validation feedback during initialization.
//!
//! Does NOT handle:
//! - Parsing key strings (handled by splunk_config::keybind).
//! - Persisting user preferences (handled by ConfigManager).
//! - Runtime keybinding changes (overrides are immutable after startup).

use std::collections::HashMap;
use std::sync::OnceLock;

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use splunk_config::keybind::{KeyCodeName, ModifierFlags, ParsedKey, parse_key};
use splunk_config::{KeybindAction, KeybindOverrides};
use tracing;

use crate::action::Action;

/// Runtime keybinding override storage.
///
/// Pre-computed lookup table for fast key event matching.
#[derive(Debug, Clone)]
pub struct KeybindOverrideTable {
    /// Maps (KeyCode, KeyModifiers) -> Action for overridden bindings
    overrides: HashMap<(KeyCode, KeyModifiers), Action>,
}

impl KeybindOverrideTable {
    /// Build the override table from config.
    ///
    /// # Errors
    ///
    /// Returns an error if any keybinding cannot be parsed.
    pub fn from_overrides(overrides: &KeybindOverrides) -> Result<Self, String> {
        let mut table = HashMap::new();

        for (action, key_str) in &overrides.overrides {
            match parse_key(key_str) {
                Ok(parsed) => {
                    let key_event = parsed_key_to_crossterm(&parsed);
                    let tui_action = action_for_keybind(*action);
                    table.insert((key_event.code, key_event.modifiers), tui_action);
                }
                Err(e) => {
                    return Err(format!(
                        "Failed to parse keybinding for '{}': {}",
                        action, e
                    ));
                }
            }
        }

        Ok(Self { overrides: table })
    }

    /// Check if a key event matches an override.
    pub fn resolve(&self, key: KeyEvent) -> Option<Action> {
        self.overrides.get(&(key.code, key.modifiers)).cloned()
    }

    /// Returns true if there are no active overrides.
    pub fn is_empty(&self) -> bool {
        self.overrides.is_empty()
    }

    /// Returns the number of active overrides.
    /// Note: Only used in tests, but kept for test convenience.
    #[allow(dead_code)]
    pub fn len(&self) -> usize {
        self.overrides.len()
    }
}

/// Converts a parsed key from config into a crossterm KeyEvent.
fn parsed_key_to_crossterm(parsed: &ParsedKey) -> KeyEvent {
    let code = match &parsed.code {
        KeyCodeName::Char(c) => KeyCode::Char(*c),
        KeyCodeName::F(n) => KeyCode::F(*n),
        KeyCodeName::Esc => KeyCode::Esc,
        KeyCodeName::Enter => KeyCode::Enter,
        KeyCodeName::Space => KeyCode::Char(' '),
        KeyCodeName::Tab => KeyCode::Tab,
        KeyCodeName::BackTab => KeyCode::BackTab,
        KeyCodeName::Backspace => KeyCode::Backspace,
        KeyCodeName::Delete => KeyCode::Delete,
        KeyCodeName::Insert => KeyCode::Insert,
        KeyCodeName::Home => KeyCode::Home,
        KeyCodeName::End => KeyCode::End,
        KeyCodeName::PageUp => KeyCode::PageUp,
        KeyCodeName::PageDown => KeyCode::PageDown,
        KeyCodeName::Up => KeyCode::Up,
        KeyCodeName::Down => KeyCode::Down,
        KeyCodeName::Left => KeyCode::Left,
        KeyCodeName::Right => KeyCode::Right,
    };

    let modifiers = modifier_flags_to_crossterm(&parsed.modifiers);

    KeyEvent::new(code, modifiers)
}

/// Converts ModifierFlags to crossterm KeyModifiers.
fn modifier_flags_to_crossterm(flags: &ModifierFlags) -> KeyModifiers {
    let mut modifiers = KeyModifiers::NONE;
    if flags.ctrl {
        modifiers |= KeyModifiers::CONTROL;
    }
    if flags.shift {
        modifiers |= KeyModifiers::SHIFT;
    }
    if flags.alt {
        modifiers |= KeyModifiers::ALT;
    }
    modifiers
}

/// Resolves a KeybindAction to the appropriate TUI Action.
fn action_for_keybind(action: KeybindAction) -> Action {
    match action {
        KeybindAction::Quit => Action::Quit,
        KeybindAction::Help => Action::OpenHelpPopup,
        KeybindAction::NextScreen => Action::NextScreen,
        KeybindAction::PreviousScreen => Action::PreviousScreen,
    }
}

// Global override table, initialized once at startup
static KEYBIND_OVERRIDES: OnceLock<KeybindOverrideTable> = OnceLock::new();

/// Initialize the keybinding override table from persisted state.
///
/// This should be called once at app startup, after loading persisted state.
/// If validation fails, a warning is logged and the app continues with defaults.
///
/// # Arguments
///
/// * `overrides` - The user's keybinding overrides from config
///
/// # Returns
///
/// Returns `Ok(())` if initialization succeeded (or if there are no overrides).
/// Returns `Err` only if the override table fails to build (should not happen
/// if validate_overrides was called first).
pub fn init_overrides(overrides: &KeybindOverrides) -> Result<(), String> {
    if overrides.is_empty() {
        tracing::debug!("No keybinding overrides configured");
        return Ok(());
    }

    // Validate overrides first
    if let Err(e) = splunk_config::keybind::validate_overrides(&overrides.overrides) {
        tracing::warn!(
            "Keybinding validation failed: {}. Using default keybindings.",
            e
        );
        return Ok(());
    }

    match KeybindOverrideTable::from_overrides(overrides) {
        Ok(table) => {
            let count = table.len();
            match KEYBIND_OVERRIDES.set(table) {
                Ok(_) => {
                    tracing::info!("Loaded {} keybinding override(s)", count);
                    Ok(())
                }
                Err(_) => {
                    tracing::warn!("Keybinding overrides already initialized");
                    Ok(())
                }
            }
        }
        Err(e) => {
            tracing::warn!(
                "Failed to build keybinding override table: {}. Using default keybindings.",
                e
            );
            Ok(())
        }
    }
}

/// Check if a key event matches a user-defined override.
///
/// This should be called by `resolve_action` before checking default bindings.
pub(crate) fn resolve_override(key: KeyEvent) -> Option<Action> {
    KEYBIND_OVERRIDES.get().and_then(|table| table.resolve(key))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    fn char_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
    }

    fn ctrl_key(c: char) -> KeyEvent {
        KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
    }

    fn f_key(n: u8) -> KeyEvent {
        KeyEvent::new(KeyCode::F(n), KeyModifiers::NONE)
    }

    #[test]
    fn test_override_table_empty() {
        let overrides = KeybindOverrides::default();
        let table = KeybindOverrideTable::from_overrides(&overrides).unwrap();
        assert!(table.is_empty());
        assert_eq!(table.len(), 0);
    }

    #[test]
    fn test_override_table_single_binding() {
        let mut map = BTreeMap::new();
        map.insert(KeybindAction::Quit, "F10".to_string());
        let overrides = KeybindOverrides { overrides: map };

        let table = KeybindOverrideTable::from_overrides(&overrides).unwrap();
        assert!(!table.is_empty());
        assert_eq!(table.len(), 1);

        // F10 should resolve to Quit
        let action = table.resolve(f_key(10));
        assert!(matches!(action, Some(Action::Quit)));

        // Other keys should not match
        assert!(table.resolve(char_key('q')).is_none());
    }

    #[test]
    fn test_override_table_multiple_bindings() {
        let mut map = BTreeMap::new();
        map.insert(KeybindAction::Quit, "Ctrl+x".to_string());
        map.insert(KeybindAction::Help, "F1".to_string());
        map.insert(KeybindAction::NextScreen, "Ctrl+n".to_string());
        map.insert(KeybindAction::PreviousScreen, "Ctrl+p".to_string());
        let overrides = KeybindOverrides { overrides: map };

        let table = KeybindOverrideTable::from_overrides(&overrides).unwrap();
        assert_eq!(table.len(), 4);

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
    fn test_override_table_invalid_key() {
        let mut map = BTreeMap::new();
        map.insert(KeybindAction::Quit, "InvalidKey".to_string());
        let overrides = KeybindOverrides { overrides: map };

        let result = KeybindOverrideTable::from_overrides(&overrides);
        assert!(result.is_err());
        assert!(result.unwrap_err().contains("Failed to parse keybinding"));
    }

    #[test]
    fn test_parsed_key_to_crossterm_char() {
        let parsed = ParsedKey {
            code: KeyCodeName::Char('a'),
            modifiers: ModifierFlags::default(),
        };
        let event = parsed_key_to_crossterm(&parsed);
        assert_eq!(event.code, KeyCode::Char('a'));
        assert_eq!(event.modifiers, KeyModifiers::NONE);
    }

    #[test]
    fn test_parsed_key_to_crossterm_ctrl() {
        let parsed = ParsedKey {
            code: KeyCodeName::Char('x'),
            modifiers: ModifierFlags {
                ctrl: true,
                ..Default::default()
            },
        };
        let event = parsed_key_to_crossterm(&parsed);
        assert_eq!(event.code, KeyCode::Char('x'));
        assert!(event.modifiers.contains(KeyModifiers::CONTROL));
    }

    #[test]
    fn test_parsed_key_to_crossterm_f_key() {
        let parsed = ParsedKey {
            code: KeyCodeName::F(5),
            modifiers: ModifierFlags::default(),
        };
        let event = parsed_key_to_crossterm(&parsed);
        assert_eq!(event.code, KeyCode::F(5));
    }

    #[test]
    fn test_action_for_keybind() {
        assert!(matches!(
            action_for_keybind(KeybindAction::Quit),
            Action::Quit
        ));
        assert!(matches!(
            action_for_keybind(KeybindAction::Help),
            Action::OpenHelpPopup
        ));
        assert!(matches!(
            action_for_keybind(KeybindAction::NextScreen),
            Action::NextScreen
        ));
        assert!(matches!(
            action_for_keybind(KeybindAction::PreviousScreen),
            Action::PreviousScreen
        ));
    }
}
