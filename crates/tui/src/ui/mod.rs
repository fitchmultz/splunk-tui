//! UI rendering modules for the TUI.
//!
//! This module contains screen-specific rendering logic that is separated
//! from the main app state management.
//!
pub mod error_details;
pub mod popup;
pub mod screens;
pub mod syntax;
pub mod toast;

pub use toast::{Toast, ToastLevel};
