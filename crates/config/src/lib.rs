//! Configuration management for Splunk TUI.
//!
//! This crate provides types and loaders for managing Splunk connection
//! configuration from environment variables and files.

mod loader;
mod persistence;
mod types;

pub use loader::ConfigLoader;
pub use persistence::{ConfigManager, PersistedState};
pub use types::{AuthConfig, AuthStrategy, Config, ConnectionConfig, ProfileConfig};
