//! Event handling for the TUI.

use crossterm::event::KeyEvent;

/// TUI events.
#[derive(Debug, Clone, Copy)]
pub enum Event {
    Input(KeyEvent),
    Tick,
}
