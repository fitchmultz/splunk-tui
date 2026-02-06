//! Runtime components for the TUI application.
//!
//! This module contains the runtime infrastructure for the TUI:
//! - Terminal management (TerminalGuard)
//! - Client creation and authentication
//! - Configuration loading and persistence
//! - Async side effect handlers for API calls
//!
//! Does NOT handle:
//! - UI rendering or input handling (see `splunk_tui::app` and `splunk_tui::ui`).
//! - Business logic for Splunk operations (see `splunk_client`).
//!
//! Invariants:
//! - All modules are initialized during application startup in `main()`.
//! - Side effects run in separate tokio tasks to avoid blocking the UI.

pub mod client;
pub mod config;
pub mod side_effects;
pub mod terminal;
