//! Configuration persistence for user preferences.
//!
//! Responsibilities:
//! - Manage the standard and legacy configuration file paths.
//! - Handle automatic migration from legacy to standard paths.
//! - Read and write user preferences (`PersistedState`) to disk.
//! - Manage multiple configuration profiles and their secure values.
//! - Backup corrupt config files before overwriting.
//!
//! Does NOT handle:
//! - Loading environment variables (see `loader.rs`).
//! - High-level configuration merging (see `loader.rs`).
//! - Direct REST API communication (see `crates/client`).
//!
//! Invariants:
//! - The standard configuration path is preferred over the legacy path.
//! - Migration is best-effort and atomic (using rename); it should not block startup.
//! - Profile names are unique within a configuration file.
//! - Corrupt config files are backed up before being overwritten.

use std::path::{Path, PathBuf};

mod migration;
mod path;
mod profiles;
mod state;

pub use profiles::ConfigManager;
pub use state::{
    ConfigFileError, InternalLogsDefaults, ListDefaults, ListType, PersistedState, ScrollPositions,
    SearchDefaults,
};

// Internal re-exports for use by loader module
pub(crate) use migration::migrate_config_file_if_needed;
pub(crate) use path::{default_config_path, legacy_config_path};
pub(crate) use state::read_config_file;

/// Creates a backup of a corrupt config file before it is overwritten.
///
/// The backup is created by renaming the original file to a path with a
/// `.corrupt.{timestamp}` extension. This preserves the original file contents
/// for potential recovery while preventing the corrupt file from blocking
/// application startup.
///
/// # Arguments
///
/// * `path` - The path to the corrupt config file.
///
/// # Returns
///
/// Returns the path to the backup file on success, or an IO error if the
/// backup could not be created.
///
/// # Example
///
/// ```rust,ignore
/// let backup_path = create_corrupt_backup(Path::new("/home/user/.config/splunk/config.json"))?;
/// println!("Config backed up to: {}", backup_path.display());
/// ```
pub(crate) fn create_corrupt_backup(path: &Path) -> Result<PathBuf, std::io::Error> {
    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();

    // Create backup path with .corrupt.{timestamp} extension
    // We use with_extension which replaces the last extension
    let backup_path = path.with_extension(format!("corrupt.{}", timestamp));

    std::fs::rename(path, &backup_path)?;

    Ok(backup_path)
}
