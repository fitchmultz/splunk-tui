//! Configuration type definitions for Splunk TUI.
//!
//! Responsibilities:
//! - Define configuration types for authentication, connections, themes, profiles, and keybindings.
//! - Provide serialization helpers for sensitive types (secrets, durations).
//! - Ensure consistent defaults and type safety across the configuration system.
//!
//! Does NOT handle:
//! - Configuration loading from files or environment variables (see `loader` module).
//! - Configuration persistence or state management (see `persistence` module).
//! - Keybinding parsing or validation (see `keybind` module at crate root).
//! - Actual network connections or authentication flows (see client crate).
//!
//! Invariants:
//! - All secret types use `secrecy::SecretString` to prevent accidental logging.
//! - Serialization helpers (`secret_string`, `duration_seconds`) are private modules.
//! - `ColorTheme` is the persisted representation; `Theme` is the runtime representation.
//! - `KEYRING_SERVICE` is the canonical service name for all keyring operations.

mod auth;
pub(crate) mod connection;
pub mod keybind;
mod profile;
mod theme;

pub use auth::{AuthConfig, AuthStrategy, KEYRING_SERVICE, SecureValue};
pub use connection::{
    Config, ConnectionConfig, default_circuit_breaker_enabled, default_circuit_failure_threshold,
    default_circuit_failure_window, default_circuit_half_open_requests,
    default_circuit_reset_timeout,
};
pub use keybind::{KeybindAction, KeybindOverrides};
pub use profile::ProfileConfig;
pub use theme::{ColorTheme, Theme};
