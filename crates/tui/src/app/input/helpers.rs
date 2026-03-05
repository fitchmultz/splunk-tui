//! Input handling helper functions.
//!
//! Responsibilities:
//! - Classify key events for input handling
//! - Determine if keys are printable, mode switches, or cursor editing
//!
//! Does NOT handle:
//! - Does NOT handle App state
//! - Does NOT dispatch actions

use crossterm::event::{KeyCode, KeyEvent};

/// Check if a key event represents a printable character that should be inserted
/// into text input during QueryFocused mode.
///
/// A key is considered printable only if:
/// - It's a character key (KeyCode::Char)
/// - The character is not a control character
/// - No modifier keys (Ctrl, Alt, etc.) are pressed
pub fn is_printable_char(key: KeyEvent) -> bool {
    matches!(key.code, KeyCode::Char(c) if !c.is_control() && key.modifiers.is_empty())
}

/// Check if a key event is used for mode switching in the search screen.
/// These keys should bypass global bindings when in QueryFocused mode.
pub fn is_mode_switch_key(_key: KeyEvent) -> bool {
    // Tab/BackTab are now handled by global keymap for screen navigation
    // Focus switching is done via Ctrl+Tab/Ctrl+Shift+Tab (NextFocus/PreviousFocus)
    false
}

/// Check if a key event is used for cursor movement/editing in the search query.
/// These keys should bypass global bindings when in QueryFocused mode (RQ-0110).
pub fn is_cursor_editing_key(key: KeyEvent) -> bool {
    matches!(
        key.code,
        KeyCode::Left
            | KeyCode::Right
            | KeyCode::Home
            | KeyCode::End
            | KeyCode::Delete
            | KeyCode::Backspace
    )
}
