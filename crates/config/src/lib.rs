//! Configuration management for Splunk TUI.
//!
//! This crate provides types and loaders for managing Splunk connection
//! configuration from environment variables and files.

mod loader;
pub mod persistence;
pub mod types;

pub use loader::ConfigLoader;
pub use persistence::{ConfigManager, PersistedState};
pub use types::{
    AuthConfig, AuthStrategy, ColorTheme, Config, ConnectionConfig, ProfileConfig, SecureValue,
    Theme,
};
