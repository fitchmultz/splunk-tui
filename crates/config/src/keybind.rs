//! Keybinding parsing and validation.
//!
//! Responsibilities:
//! - Parse human-readable key strings into structured representations.
//! - Validate key combinations for conflicts and invalid syntax.
//!
//! Does NOT handle:
//! - Integration with crossterm (that's in the TUI crate).
//! - Runtime key event matching.

use std::collections::HashMap;

use thiserror::Error;

use crate::types::keybind::KeybindAction;

/// Errors that can occur when parsing or validating keybindings.
#[derive(Debug, Error, PartialEq)]
pub enum KeybindError {
    /// Invalid key syntax
    #[error("Invalid key syntax: '{key}'. Expected format like 'q', 'Ctrl+x', 'Shift+Tab', 'F1'")]
    InvalidSyntax {
        /// The invalid key string
        key: String,
    },

    /// Unknown key name
    #[error("Unknown key name: '{name}'")]
    UnknownKey {
        /// The unknown key name
        name: String,
    },

    /// Conflicting keybindings
    #[error("Conflicting keybindings: '{key}' is assigned to both {action1} and {action2}")]
    Conflict {
        /// The conflicting key
        key: String,
        /// First action using this key
        action1: String,
        /// Second action using this key
        action2: String,
    },

    /// Reserved keybinding
    #[error("Reserved keybinding: '{key}' cannot be overridden")]
    ReservedKey {
        /// The reserved key
        key: String,
    },
}

/// A parsed key combination.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ParsedKey {
    /// The key code name (for cross-crate compatibility, we use strings)
    pub code: KeyCodeName,
    /// Modifier flags
    pub modifiers: ModifierFlags,
}

/// Key code names that can be parsed from config strings.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum KeyCodeName {
    /// A character key (e.g., 'a', '1', '?')
    Char(char),
    /// Function key F1-F20
    F(u8),
    /// Escape key
    Esc,
    /// Enter/Return key
    Enter,
    /// Space key
    Space,
    /// Tab key
    Tab,
    /// BackTab (Shift+Tab) key
    BackTab,
    /// Backspace key
    Backspace,
    /// Delete key
    Delete,
    /// Insert key
    Insert,
    /// Home key
    Home,
    /// End key
    End,
    /// Page Up key
    PageUp,
    /// Page Down key
    PageDown,
    /// Up arrow key
    Up,
    /// Down arrow key
    Down,
    /// Left arrow key
    Left,
    /// Right arrow key
    Right,
}

impl fmt::Display for KeyCodeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Char(c) => write!(f, "{}", c),
            Self::F(n) => write!(f, "F{}", n),
            Self::Esc => write!(f, "Esc"),
            Self::Enter => write!(f, "Enter"),
            Self::Space => write!(f, "Space"),
            Self::Tab => write!(f, "Tab"),
            Self::BackTab => write!(f, "BackTab"),
            Self::Backspace => write!(f, "Backspace"),
            Self::Delete => write!(f, "Delete"),
            Self::Insert => write!(f, "Insert"),
            Self::Home => write!(f, "Home"),
            Self::End => write!(f, "End"),
            Self::PageUp => write!(f, "PageUp"),
            Self::PageDown => write!(f, "PageDown"),
            Self::Up => write!(f, "Up"),
            Self::Down => write!(f, "Down"),
            Self::Left => write!(f, "Left"),
            Self::Right => write!(f, "Right"),
        }
    }
}

/// Modifier flags for key combinations.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Hash)]
pub struct ModifierFlags {
    /// Control key pressed
    pub ctrl: bool,
    /// Shift key pressed
    pub shift: bool,
    /// Alt/Option key pressed
    pub alt: bool,
}

impl fmt::Display for ModifierFlags {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut parts = Vec::new();
        if self.ctrl {
            parts.push("Ctrl");
        }
        if self.shift {
            parts.push("Shift");
        }
        if self.alt {
            parts.push("Alt");
        }
        if parts.is_empty() {
            write!(f, "None")
        } else {
            write!(f, "{}", parts.join("+"))
        }
    }
}

use std::fmt;

/// Parse a key string like "Ctrl+x", "F1", "Shift+Tab" into structured form.
///
/// # Examples
///
/// ```
/// use splunk_config::keybind::{parse_key, KeyCodeName, ModifierFlags};
///
/// let key = parse_key("Ctrl+x").unwrap();
/// assert!(matches!(key.code, KeyCodeName::Char('x')));
/// assert!(key.modifiers.ctrl);
///
/// let key = parse_key("F1").unwrap();
/// assert!(matches!(key.code, KeyCodeName::F(1)));
/// ```
pub fn parse_key(key_str: &str) -> Result<ParsedKey, KeybindError> {
    let key_str = key_str.trim();

    if key_str.is_empty() {
        return Err(KeybindError::InvalidSyntax {
            key: key_str.to_string(),
        });
    }

    // Split by '+' to handle modifiers
    let parts: Vec<&str> = key_str.split('+').map(|s| s.trim()).collect();

    let mut modifiers = ModifierFlags::default();
    let mut key_name = "";

    // Parse modifiers and find the key name
    for part in &parts {
        match part.to_ascii_lowercase().as_str() {
            "ctrl" => modifiers.ctrl = true,
            "shift" => modifiers.shift = true,
            "alt" => modifiers.alt = true,
            _ => {
                if key_name.is_empty() {
                    key_name = part;
                } else {
                    // Multiple non-modifier parts is invalid
                    return Err(KeybindError::InvalidSyntax {
                        key: key_str.to_string(),
                    });
                }
            }
        }
    }

    if key_name.is_empty() {
        return Err(KeybindError::InvalidSyntax {
            key: key_str.to_string(),
        });
    }

    // Parse the key code
    let code = parse_key_code(key_name)?;

    // Handle special case: Shift+Tab should be BackTab
    let code = if matches!(code, KeyCodeName::Tab) && modifiers.shift {
        KeyCodeName::BackTab
    } else {
        code
    };

    Ok(ParsedKey { code, modifiers })
}

/// Parse a key code name (without modifiers).
fn parse_key_code(name: &str) -> Result<KeyCodeName, KeybindError> {
    let name_lower = name.to_ascii_lowercase();

    // Check for special keys
    match name_lower.as_str() {
        "esc" | "escape" => return Ok(KeyCodeName::Esc),
        "enter" | "return" => return Ok(KeyCodeName::Enter),
        "space" => return Ok(KeyCodeName::Space),
        "tab" => return Ok(KeyCodeName::Tab),
        "backtab" => return Ok(KeyCodeName::BackTab),
        "backspace" => return Ok(KeyCodeName::Backspace),
        "delete" | "del" => return Ok(KeyCodeName::Delete),
        "insert" | "ins" => return Ok(KeyCodeName::Insert),
        "home" => return Ok(KeyCodeName::Home),
        "end" => return Ok(KeyCodeName::End),
        "pageup" | "page_up" | "pgup" => return Ok(KeyCodeName::PageUp),
        "pagedown" | "page_down" | "pgdn" => return Ok(KeyCodeName::PageDown),
        "up" => return Ok(KeyCodeName::Up),
        "down" => return Ok(KeyCodeName::Down),
        "left" => return Ok(KeyCodeName::Left),
        "right" => return Ok(KeyCodeName::Right),
        _ => {}
    }

    // Check for function keys (F1-F20)
    if let Some(num_str) = name_lower.strip_prefix('f')
        && let Ok(num) = num_str.parse::<u8>()
        && (1..=20).contains(&num)
    {
        return Ok(KeyCodeName::F(num));
    }

    // Check for single character
    let chars: Vec<char> = name.chars().collect();
    if chars.len() == 1 {
        return Ok(KeyCodeName::Char(chars[0]));
    }

    // Unknown key
    Err(KeybindError::UnknownKey {
        name: name.to_string(),
    })
}

/// List of keys that should not be allowed for override
/// (e.g., Ctrl+C for copy, Ctrl+Z for suspend).
pub const RESERVED_KEYS: &[&str] = &["Ctrl+c", "Ctrl+z", "Ctrl+C", "Ctrl+Z"];

/// Validate a set of keybinding overrides for conflicts.
///
/// # Examples
///
/// ```
/// use std::collections::BTreeMap;
/// use splunk_config::keybind::{validate_overrides, KeybindError};
/// use splunk_config::types::KeybindAction;
///
/// let mut overrides = BTreeMap::new();
/// overrides.insert(KeybindAction::Quit, "F1".to_string());
/// overrides.insert(KeybindAction::Help, "F2".to_string());
///
/// assert!(validate_overrides(&overrides).is_ok());
/// ```
pub fn validate_overrides(
    overrides: &std::collections::BTreeMap<KeybindAction, String>,
) -> Result<(), KeybindError> {
    // Check for reserved keys
    for (action, key_str) in overrides {
        let normalized = key_str.replace(' ', "");
        if RESERVED_KEYS.contains(&normalized.as_str()) {
            return Err(KeybindError::ReservedKey {
                key: key_str.clone(),
            });
        }

        // Validate the key can be parsed
        if let Err(e) = parse_key(key_str) {
            return Err(KeybindError::InvalidSyntax {
                key: format!("{} for action '{}': {}", key_str, action, e),
            });
        }
    }

    // Check for conflicts (same key assigned to multiple actions)
    let mut key_to_action: HashMap<String, KeybindAction> = HashMap::new();

    for (action, key_str) in overrides {
        // Normalize the key string for comparison
        let normalized = normalize_key(key_str);

        if let Some(existing_action) = key_to_action.get(&normalized) {
            return Err(KeybindError::Conflict {
                key: key_str.clone(),
                action1: existing_action.to_string(),
                action2: action.to_string(),
            });
        }
        key_to_action.insert(normalized, *action);
    }

    Ok(())
}

/// Normalize a key string for comparison.
/// Converts to lowercase and removes spaces around '+'.
fn normalize_key(key_str: &str) -> String {
    key_str
        .split('+')
        .map(|s| s.trim().to_ascii_lowercase())
        .collect::<Vec<_>>()
        .join("+")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeMap;

    #[test]
    fn test_parse_simple_char() {
        let key = parse_key("q").unwrap();
        assert_eq!(key.code, KeyCodeName::Char('q'));
        assert!(!key.modifiers.ctrl);
        assert!(!key.modifiers.shift);
        assert!(!key.modifiers.alt);
    }

    #[test]
    fn test_parse_uppercase_char() {
        let key = parse_key("Q").unwrap();
        assert_eq!(key.code, KeyCodeName::Char('Q'));
    }

    #[test]
    fn test_parse_ctrl_combo() {
        let key = parse_key("Ctrl+x").unwrap();
        assert_eq!(key.code, KeyCodeName::Char('x'));
        assert!(key.modifiers.ctrl);
        assert!(!key.modifiers.shift);
        assert!(!key.modifiers.alt);
    }

    #[test]
    fn test_parse_ctrl_alt_combo() {
        let key = parse_key("Ctrl+Alt+x").unwrap();
        assert_eq!(key.code, KeyCodeName::Char('x'));
        assert!(key.modifiers.ctrl);
        assert!(!key.modifiers.shift);
        assert!(key.modifiers.alt);
    }

    #[test]
    fn test_parse_all_modifiers() {
        let key = parse_key("Ctrl+Shift+Alt+x").unwrap();
        assert_eq!(key.code, KeyCodeName::Char('x'));
        assert!(key.modifiers.ctrl);
        assert!(key.modifiers.shift);
        assert!(key.modifiers.alt);
    }

    #[test]
    fn test_parse_function_key() {
        let key = parse_key("F1").unwrap();
        assert_eq!(key.code, KeyCodeName::F(1));

        let key = parse_key("f12").unwrap();
        assert_eq!(key.code, KeyCodeName::F(12));

        let key = parse_key("F20").unwrap();
        assert_eq!(key.code, KeyCodeName::F(20));
    }

    #[test]
    fn test_parse_invalid_function_key() {
        // F0 is invalid
        assert!(parse_key("F0").is_err());
        // F21 is invalid (only F1-F20 supported)
        assert!(parse_key("F21").is_err());
        // F100 is invalid
        assert!(parse_key("F100").is_err());
    }

    #[test]
    fn test_parse_special_keys() {
        assert_eq!(parse_key("Esc").unwrap().code, KeyCodeName::Esc);
        assert_eq!(parse_key("escape").unwrap().code, KeyCodeName::Esc);
        assert_eq!(parse_key("Enter").unwrap().code, KeyCodeName::Enter);
        assert_eq!(parse_key("return").unwrap().code, KeyCodeName::Enter);
        assert_eq!(parse_key("Space").unwrap().code, KeyCodeName::Space);
        assert_eq!(parse_key("Tab").unwrap().code, KeyCodeName::Tab);
        assert_eq!(parse_key("BackTab").unwrap().code, KeyCodeName::BackTab);
        assert_eq!(parse_key("Backspace").unwrap().code, KeyCodeName::Backspace);
        assert_eq!(parse_key("Delete").unwrap().code, KeyCodeName::Delete);
        assert_eq!(parse_key("del").unwrap().code, KeyCodeName::Delete);
        assert_eq!(parse_key("Insert").unwrap().code, KeyCodeName::Insert);
        assert_eq!(parse_key("ins").unwrap().code, KeyCodeName::Insert);
        assert_eq!(parse_key("Home").unwrap().code, KeyCodeName::Home);
        assert_eq!(parse_key("End").unwrap().code, KeyCodeName::End);
        assert_eq!(parse_key("PageUp").unwrap().code, KeyCodeName::PageUp);
        assert_eq!(parse_key("pageup").unwrap().code, KeyCodeName::PageUp);
        assert_eq!(parse_key("pgup").unwrap().code, KeyCodeName::PageUp);
        assert_eq!(parse_key("PageDown").unwrap().code, KeyCodeName::PageDown);
        assert_eq!(parse_key("Up").unwrap().code, KeyCodeName::Up);
        assert_eq!(parse_key("Down").unwrap().code, KeyCodeName::Down);
        assert_eq!(parse_key("Left").unwrap().code, KeyCodeName::Left);
        assert_eq!(parse_key("Right").unwrap().code, KeyCodeName::Right);
    }

    #[test]
    fn test_parse_shift_tab() {
        let key = parse_key("Shift+Tab").unwrap();
        assert_eq!(key.code, KeyCodeName::BackTab);
        assert!(key.modifiers.shift);
    }

    #[test]
    fn test_parse_with_spaces() {
        let key = parse_key("Ctrl + x").unwrap();
        assert_eq!(key.code, KeyCodeName::Char('x'));
        assert!(key.modifiers.ctrl);
    }

    #[test]
    fn test_invalid_syntax_empty() {
        let result = parse_key("");
        assert!(matches!(result, Err(KeybindError::InvalidSyntax { .. })));
    }

    #[test]
    fn test_invalid_syntax_only_modifiers() {
        let result = parse_key("Ctrl+Shift");
        assert!(matches!(result, Err(KeybindError::InvalidSyntax { .. })));
    }

    #[test]
    fn test_unknown_key() {
        let result = parse_key("Ctrl+Unknown");
        assert!(matches!(result, Err(KeybindError::UnknownKey { .. })));
    }

    #[test]
    fn test_validate_conflicts() {
        let mut overrides = BTreeMap::new();
        overrides.insert(KeybindAction::Quit, "F1".to_string());
        overrides.insert(KeybindAction::Help, "F1".to_string()); // Conflict!

        let result = validate_overrides(&overrides);
        assert!(matches!(result, Err(KeybindError::Conflict { .. })));
    }

    #[test]
    fn test_validate_no_conflicts() {
        let mut overrides = BTreeMap::new();
        overrides.insert(KeybindAction::Quit, "F1".to_string());
        overrides.insert(KeybindAction::Help, "F2".to_string());
        overrides.insert(KeybindAction::NextScreen, "Ctrl+n".to_string());
        overrides.insert(KeybindAction::PreviousScreen, "Ctrl+p".to_string());

        assert!(validate_overrides(&overrides).is_ok());
    }

    #[test]
    fn test_validate_reserved_key() {
        let mut overrides = BTreeMap::new();
        overrides.insert(KeybindAction::Quit, "Ctrl+c".to_string());

        let result = validate_overrides(&overrides);
        assert!(matches!(result, Err(KeybindError::ReservedKey { .. })));
    }

    #[test]
    fn test_validate_reserved_key_uppercase() {
        let mut overrides = BTreeMap::new();
        overrides.insert(KeybindAction::Quit, "Ctrl+C".to_string());

        let result = validate_overrides(&overrides);
        assert!(matches!(result, Err(KeybindError::ReservedKey { .. })));
    }

    #[test]
    fn test_validate_invalid_syntax() {
        let mut overrides = BTreeMap::new();
        overrides.insert(KeybindAction::Quit, "".to_string());

        let result = validate_overrides(&overrides);
        assert!(matches!(result, Err(KeybindError::InvalidSyntax { .. })));
    }

    #[test]
    fn test_normalize_key() {
        assert_eq!(normalize_key("Ctrl+x"), "ctrl+x");
        assert_eq!(normalize_key("CTRL + X"), "ctrl+x");
        assert_eq!(normalize_key("  Ctrl  +  x  "), "ctrl+x");
        assert_eq!(normalize_key("F1"), "f1");
    }

    #[test]
    fn test_case_insensitive_modifiers() {
        // Modifiers are case-insensitive, but key characters are case-sensitive
        let key1 = parse_key("ctrl+x").unwrap();
        let key2 = parse_key("CTRL+x").unwrap();
        let key3 = parse_key("Ctrl+x").unwrap();

        // All should have the same code (lowercase 'x')
        assert_eq!(key1.code, key2.code);
        assert_eq!(key1.code, key3.code);
        // All should have ctrl modifier
        assert!(key1.modifiers.ctrl);
        assert!(key2.modifiers.ctrl);
        assert!(key3.modifiers.ctrl);
        // All should be equal
        assert_eq!(key1, key2);
        assert_eq!(key1, key3);
    }

    #[test]
    fn test_case_sensitive_char_keys() {
        // Character keys are case-sensitive: 'x' and 'X' are different
        let lower = parse_key("x").unwrap();
        let upper = parse_key("X").unwrap();

        assert_eq!(lower.code, KeyCodeName::Char('x'));
        assert_eq!(upper.code, KeyCodeName::Char('X'));
        assert_ne!(lower.code, upper.code);
    }

    #[test]
    fn test_display_key_code_name() {
        assert_eq!(format!("{}", KeyCodeName::Char('a')), "a");
        assert_eq!(format!("{}", KeyCodeName::F(1)), "F1");
        assert_eq!(format!("{}", KeyCodeName::Esc), "Esc");
        assert_eq!(format!("{}", KeyCodeName::Enter), "Enter");
    }

    #[test]
    fn test_display_modifier_flags() {
        assert_eq!(format!("{}", ModifierFlags::default()), "None");
        assert_eq!(
            format!(
                "{}",
                ModifierFlags {
                    ctrl: true,
                    ..Default::default()
                }
            ),
            "Ctrl"
        );
        assert_eq!(
            format!(
                "{}",
                ModifierFlags {
                    ctrl: true,
                    shift: true,
                    alt: true,
                }
            ),
            "Ctrl+Shift+Alt"
        );
    }
}
