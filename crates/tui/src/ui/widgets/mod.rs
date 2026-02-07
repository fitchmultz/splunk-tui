//! Shared UI widgets for common rendering patterns.
//!
//! This module provides reusable widgets for loading states, empty states,
//! and other common UI patterns across TUI screens.

pub mod empty;
pub mod loading;

pub use empty::{render_empty_state, render_empty_state_custom};
pub use loading::{render_loading, render_loading_state};
