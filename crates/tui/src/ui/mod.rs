//! UI rendering modules for the TUI.
//!
//! This module contains screen-specific rendering logic that is separated
//! from the main app state management.
//!
//! # Component Library
//!
//! The `components` module provides reusable, theme-aware components:
//! - [`SelectList<T>`]: Generic selectable list
//! - [`ScrollableContainer`]: Scrollable content container
//!
//! # Theme Helpers
//!
//! The [`theme`] module extends `splunk_config::Theme` with ergonomic
//! style builders via the [`ThemeExt`] trait.
//!
//! # Example
//!
//! ```rust,ignore
//! use splunk_tui::ui::{ThemeExt, components::SelectList};
//! use splunk_config::Theme;
//!
//! let theme = Theme::default();
//! let style = theme.title(); // Uses ThemeExt trait
//! ```

pub mod components;
pub mod error_details;
pub mod index_details;
pub mod popup;
pub mod screens;
pub mod syntax;
pub mod theme;
pub mod toast;

// Layout module with flexbox support via taffy
pub mod layout {
    pub mod flex;
}

pub use theme::ThemeExt;
pub use toast::{Toast, ToastLevel};
