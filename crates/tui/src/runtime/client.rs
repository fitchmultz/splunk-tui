//! Splunk client creation and authentication.
//!
//! Responsibilities:
//! - Create and authenticate Splunk client instances from configuration.
//!
//! Does NOT handle:
//! - Configuration loading (see `runtime::config`).
//! - Terminal state management (see `runtime::terminal`).
//!
//! Invariants / Assumptions:
//! - The provided config has valid base_url and auth credentials.
//! - Session token auth requires calling `login()` before use.
//!
//! Note: The actual implementation has been moved to the shared client crate
//! (`splunk_client::SplunkClient::from_config`) to avoid duplication between
//! CLI and TUI. This module now provides a thin wrapper for TUI-specific use.

use anyhow::Result;
use splunk_client::SplunkClient;
use splunk_config::Config;

/// Create and authenticate a new Splunk client.
///
/// This function delegates to the shared client creation logic in the client crate.
/// For session token authentication, it performs the initial login automatically.
///
/// # Arguments
///
/// * `config` - The loaded configuration containing connection and auth settings
///
/// # Errors
///
/// Returns an error if client construction fails or if login fails for
/// session token authentication.
pub async fn create_client(config: &Config) -> Result<SplunkClient> {
    SplunkClient::from_config(config)
        .await
        .map_err(|e| e.into())
}
