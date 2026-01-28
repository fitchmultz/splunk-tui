//! Splunk TUI Library
//!
//! This library provides the core application logic, state management,
//! and UI components for the Splunk terminal user interface.
//!
//! # Example
//!
//! ```rust
//! use splunk_tui::{App, Action};
//! use crossterm::event::KeyEvent;
//!
//! let mut app = App::default();
//! if let Some(action) = app.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('q'))) {
//!     // Handle action
//! }
//! ```

pub mod action;
pub mod app;
pub mod error_details;
pub mod export;
pub mod ui;

pub mod input;

// Re-export commonly used types at the crate root
pub use action::Action;
pub use app::{
    App, ConnectionContext, CurrentScreen, FOOTER_HEIGHT, HEADER_HEIGHT, SearchInputMode,
};
pub use error_details::ErrorDetails;
pub use ui::popup::{Popup, PopupType};
pub use ui::toast::{Toast, ToastLevel};

/// Render the TUI keybinding documentation block for docs/usage.md.
///
/// This is a thin wrapper over the internal keymap renderer so binary tools
/// can regenerate documentation without accessing private modules.
pub fn render_tui_keybinding_docs() -> String {
    input::docs::render_markdown()
}
