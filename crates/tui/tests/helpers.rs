//! Test helpers for TUI testing.
//!
//! Provides utility functions for simulating keyboard input and creating
//! test fixtures for the TUI application.

#![allow(dead_code)]

use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};

/// Create a character key event.
pub fn key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::NONE)
}

/// Create an Enter key event.
pub fn enter_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Enter, KeyModifiers::NONE)
}

/// Create an Escape key event.
pub fn esc_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Esc, KeyModifiers::NONE)
}

/// Create a Down arrow key event.
pub fn down_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Down, KeyModifiers::NONE)
}

/// Create an Up arrow key event.
pub fn up_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Up, KeyModifiers::NONE)
}

/// Create a Page Down key event.
pub fn page_down_key() -> KeyEvent {
    KeyEvent::new(KeyCode::PageDown, KeyModifiers::NONE)
}

/// Create a Page Up key event.
pub fn page_up_key() -> KeyEvent {
    KeyEvent::new(KeyCode::PageUp, KeyModifiers::NONE)
}

/// Create a Home key event.
pub fn home_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Home, KeyModifiers::NONE)
}

/// Create an End key event.
pub fn end_key() -> KeyEvent {
    KeyEvent::new(KeyCode::End, KeyModifiers::NONE)
}

/// Create a Backspace key event.
pub fn backspace_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Backspace, KeyModifiers::NONE)
}

/// Create a Delete key event.
pub fn delete_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Delete, KeyModifiers::NONE)
}

/// Create a Left arrow key event.
pub fn left_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Left, KeyModifiers::NONE)
}

/// Create a Right arrow key event.
pub fn right_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Right, KeyModifiers::NONE)
}

/// Create a Tab key event.
pub fn tab_key() -> KeyEvent {
    KeyEvent::new(KeyCode::Tab, KeyModifiers::NONE)
}

/// Create a Shift+Tab (BackTab) key event.
pub fn shift_tab_key() -> KeyEvent {
    KeyEvent::new(KeyCode::BackTab, KeyModifiers::NONE)
}

/// Create a Ctrl+char key event.
pub fn ctrl_key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}

/// Create a key event with explicit KeyEventKind.
pub fn key_with_kind(code: KeyCode, kind: crossterm::event::KeyEventKind) -> KeyEvent {
    KeyEvent {
        code,
        modifiers: KeyModifiers::NONE,
        kind,
        state: crossterm::event::KeyEventState::NONE,
    }
}

/// Create a Release key event (used to test filtering).
pub fn release_key(c: char) -> KeyEvent {
    key_with_kind(KeyCode::Char(c), crossterm::event::KeyEventKind::Release)
}

/// Create a Repeat key event (used to test filtering).
pub fn repeat_key(c: char) -> KeyEvent {
    key_with_kind(KeyCode::Char(c), crossterm::event::KeyEventKind::Repeat)
}

/// Create a mouse click event.
pub fn mouse_click(col: u16, row: u16) -> crossterm::event::MouseEvent {
    use crossterm::event::{MouseButton, MouseEvent, MouseEventKind};
    MouseEvent {
        kind: MouseEventKind::Down(MouseButton::Left),
        column: col,
        row,
        modifiers: KeyModifiers::empty(),
    }
}
