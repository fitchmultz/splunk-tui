//! Configuration management for Splunk TUI.
//!
//! This crate provides types and loaders for managing Splunk connection
//! configuration from environment variables and files.

#![cfg_attr(not(test), warn(clippy::unwrap_used))]

pub mod constants;
pub mod keybind;
pub mod loader;
pub mod persistence;
pub mod types;

pub use loader::{ConfigError, ConfigLoader, SearchDefaultConfig, env_var_or_none};
pub use persistence::{ConfigManager, PersistedState, SearchDefaults};
pub use types::{
    AuthConfig, AuthStrategy, ColorTheme, Config, ConnectionConfig, KeybindAction,
    KeybindOverrides, ProfileConfig, SecureValue, Theme,
};

#[cfg(test)]
pub(crate) mod test_util {
    use std::sync::{Mutex, OnceLock};

    pub fn global_test_lock() -> &'static Mutex<()> {
        static LOCK: OnceLock<Mutex<()>> = OnceLock::new();
        LOCK.get_or_init(|| Mutex::new(()))
    }
}
