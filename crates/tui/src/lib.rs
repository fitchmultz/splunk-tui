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
//! let mut app = App::new();
//! if let Some(action) = app.handle_input(KeyEvent::from(crossterm::event::KeyCode::Char('q'))) {
//!     // Handle action
//! }
//! ```

pub mod action;
pub mod app;
pub mod ui;

// Re-export commonly used types at the crate root
pub use action::Action;
pub use app::{App, CurrentScreen, FOOTER_HEIGHT, HEADER_HEIGHT};
pub use ui::popup::{Popup, PopupType};
pub use ui::toast::{Toast, ToastLevel};
