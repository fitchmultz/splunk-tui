//! Keybinding configuration types for Splunk TUI.
//!
//! Responsibilities:
//! - Define overridable keybinding action identifiers (`KeybindAction`).
//! - Define `KeybindOverrides` for user-defined keybinding customizations.
//!
//! Does NOT handle:
//! - Keybinding parsing or validation (see `keybind` module at crate root).
//! - Runtime key event matching (see TUI crate).
//!
//! Invariants:
//! - `KeybindAction` uses snake_case serialization for config file consistency.
//! - `KeybindOverrides` uses `BTreeMap` for deterministic serialization.
//! - Only actions explicitly listed in overrides override the defaults.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;

/// An overridable keybinding action identifier.
///
/// This enum represents the subset of actions that users can customize.
/// Starting with global navigation only; may expand in the future.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Hash, PartialOrd, Ord)]
#[serde(rename_all = "snake_case")]
pub enum KeybindAction {
    /// Quit the application
    Quit,
    /// Open the help popup
    Help,
    /// Navigate to the next screen
    NextScreen,
    /// Navigate to the previous screen
    PreviousScreen,
}

impl fmt::Display for KeybindAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Quit => write!(f, "quit"),
            Self::Help => write!(f, "help"),
            Self::NextScreen => write!(f, "next_screen"),
            Self::PreviousScreen => write!(f, "previous_screen"),
        }
    }
}

/// User-defined keybinding overrides.
///
/// Maps action identifiers to key combinations. Only actions explicitly
/// listed here override the defaults; all others use built-in bindings.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct KeybindOverrides {
    /// Map of action -> key combination string.
    /// Using BTreeMap for deterministic serialization.
    #[serde(default)]
    pub overrides: BTreeMap<KeybindAction, String>,
}

impl KeybindOverrides {
    /// Returns true if there are no overrides configured.
    pub fn is_empty(&self) -> bool {
        self.overrides.is_empty()
    }

    /// Get the override for a specific action, if any.
    pub fn get(&self, action: KeybindAction) -> Option<&str> {
        self.overrides.get(&action).map(|s| s.as_str())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_keybind_overrides_is_empty() {
        let empty = KeybindOverrides::default();
        assert!(empty.is_empty());

        let mut with_override = KeybindOverrides::default();
        with_override
            .overrides
            .insert(KeybindAction::Quit, "q".to_string());
        assert!(!with_override.is_empty());
    }

    #[test]
    fn test_keybind_overrides_get() {
        let mut overrides = KeybindOverrides::default();
        overrides
            .overrides
            .insert(KeybindAction::Quit, "F1".to_string());
        overrides
            .overrides
            .insert(KeybindAction::Help, "?".to_string());

        assert_eq!(overrides.get(KeybindAction::Quit), Some("F1"));
        assert_eq!(overrides.get(KeybindAction::Help), Some("?"));
        assert_eq!(overrides.get(KeybindAction::NextScreen), None);
    }

    #[test]
    fn test_keybind_action_serde_round_trip() {
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
    fn test_keybind_overrides_serde_round_trip() {
        let mut overrides = KeybindOverrides::default();
        overrides
            .overrides
            .insert(KeybindAction::Quit, "F1".to_string());
        overrides
            .overrides
            .insert(KeybindAction::Help, "?".to_string());

        let json = serde_json::to_string(&overrides).unwrap();
        let deserialized: KeybindOverrides = serde_json::from_str(&json).unwrap();

        assert_eq!(overrides.overrides.len(), deserialized.overrides.len());
        assert_eq!(
            deserialized.overrides.get(&KeybindAction::Quit),
            Some(&"F1".to_string())
        );
    }

    #[test]
    fn test_keybind_action_ordering() {
        // Verify deterministic ordering for BTreeMap (uses declaration order with Ord derive)
        let mut actions = [
            KeybindAction::Quit,
            KeybindAction::Help,
            KeybindAction::NextScreen,
            KeybindAction::PreviousScreen,
        ];
        actions.sort();

        // Should be ordered by declaration order: Quit, Help, NextScreen, PreviousScreen
        assert!(actions[0] == KeybindAction::Quit);
        assert!(actions[1] == KeybindAction::Help);
        assert!(actions[2] == KeybindAction::NextScreen);
        assert!(actions[3] == KeybindAction::PreviousScreen);
    }
}
