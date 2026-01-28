//! Configuration persistence for user preferences.
//!
//! Responsibilities:
//! - Manage the standard and legacy configuration file paths.
//! - Handle automatic migration from legacy to standard paths.
//! - Read and write user preferences (`PersistedState`) to disk.
//! - Manage multiple configuration profiles and their secure values.
//!
//! Does NOT handle:
//! - Loading environment variables (see `loader.rs`).
//! - High-level configuration merging (see `loader.rs`).
//! - Direct REST API communication (see `crates/client`).
//!
//! Invariants / Assumptions:
//! - The standard configuration path is preferred over the legacy path.
//! - Migration is best-effort and atomic (using rename); it should not block startup.
//! - Profile names are unique within a configuration file.

mod migration;
mod path;
mod profiles;
mod state;

pub use profiles::ConfigManager;
pub use state::{ConfigFileError, PersistedState, SearchDefaults};

// Internal re-exports for use by loader module
pub(crate) use migration::migrate_config_file_if_needed;
pub(crate) use path::{default_config_path, legacy_config_path};
pub(crate) use state::read_config_file;
