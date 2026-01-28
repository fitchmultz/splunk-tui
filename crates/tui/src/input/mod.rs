//! Centralized input/keybinding definitions for the TUI.
//!
//! Responsibilities:
//! - Define the shared keybinding catalog used by input resolution, help popup, and docs.
//! - Provide deterministic rendering helpers for help and documentation output.
//!
//! Non-responsibilities:
//! - Mutating application state directly (handled by App via Actions).
//! - Performing I/O or file writes (handled by generator binaries).
//!
//! Invariants:
//! - Keybinding metadata must remain the single source of truth for help/docs.
//! - Input resolution must return Actions only and never mutate App state.

pub mod docs;
pub mod help;
pub mod keymap;
