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
//! Invariants:
//! - All error variants include context for debugging (variable names, paths, etc.).
//! - ConfigFileError is converted to ConfigError for unified error handling.
//! - Dotenv errors NEVER include raw .env line contents to prevent secret leakage.

use std::io::ErrorKind;
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

    #[error("Base URL is required. Set SPLUNK_BASE_URL or configure a profile.")]
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

    #[error("Failed to decrypt configuration: {0}")]
    DecryptionFailed(String),

    #[error("Keyring error: {0}")]
    Keyring(#[from] keyring::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("invalid timeout: {message}")]
    InvalidTimeout { message: String },

    #[error("invalid session TTL configuration: {message}")]
    InvalidSessionTtl { message: String },

    #[error("invalid health check interval: {message}")]
    InvalidHealthCheckInterval { message: String },

    /// Failed to parse the `.env` file due to invalid syntax.
    ///
    /// SAFETY: This error only includes the byte index of the parse failure,
    /// NOT the offending line content, to prevent leaking secrets.
    #[error(
        "Failed to parse .env file at position {error_index}. Hint: set DOTENV_DISABLED=1 to skip .env loading"
    )]
    DotenvParse { error_index: usize },

    /// Failed to read the `.env` file due to an I/O error.
    #[error("Failed to read .env file: {kind}")]
    DotenvIo { kind: ErrorKind },

    /// Unknown dotenv error (future variants from dotenvy crate).
    ///
    /// SAFETY: This error does not include any raw dotenv content.
    #[error("Failed to load .env file. Hint: set DOTENV_DISABLED=1 to skip .env loading")]
    DotenvUnknown,
}

impl From<ConfigFileError> for ConfigError {
    fn from(error: ConfigFileError) -> Self {
        match error {
            ConfigFileError::Read { path, .. } => ConfigError::ConfigFileRead { path },
            ConfigFileError::Parse { path, .. } => ConfigError::ConfigFileParse { path },
        }
    }
}
