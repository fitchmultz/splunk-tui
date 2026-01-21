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

/// Create a Ctrl+char key event.
pub fn ctrl_key(c: char) -> KeyEvent {
    KeyEvent::new(KeyCode::Char(c), KeyModifiers::CONTROL)
}
