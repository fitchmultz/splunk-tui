//! Error types for configuration loading.
//!
//! Responsibilities:
//! - Define error variants for all configuration loading failures.
//! - Provide conversion from lower-level errors (e.g., ConfigFileError).
//!
//! Does NOT handle:
//! - Error handling for persistence operations (see persistence.rs).
//! - Error handling for type conversions (see types.rs).
//!
//! Invariants / Assumptions:
//! - All error variants include context for debugging (variable names, paths, etc.).
//! - ConfigFileError is converted to ConfigError for unified error handling.

use std::path::PathBuf;
use thiserror::Error;

use crate::persistence::ConfigFileError;

/// Errors that can occur during configuration loading.
#[derive(Error, Debug)]
pub enum ConfigError {
    #[error("Missing required environment variable: {0}")]
    MissingEnvVar(String),

    #[error("Invalid value for {var}: {message}")]
    InvalidValue { var: String, message: String },

    #[error("Base URL is required")]
    MissingBaseUrl,

    #[error("Authentication configuration is required (either username/password or API token)")]
    MissingAuth,

    #[error("Unable to determine config directory: {0}")]
    ConfigDirUnavailable(String),

    #[error("Failed to read config file at {path}")]
    ConfigFileRead { path: PathBuf },

    #[error("Failed to parse config file at {path}")]
    ConfigFileParse { path: PathBuf },

    #[error("Profile '{0}' not found in config file")]
    ProfileNotFound(String),

    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}

impl From<ConfigFileError> for ConfigError {
    fn from(error: ConfigFileError) -> Self {
        match error {
            ConfigFileError::Read { path, .. } => ConfigError::ConfigFileRead { path },
            ConfigFileError::Parse { path, .. } => ConfigError::ConfigFileParse { path },
        }
    }
}
