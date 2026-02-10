//! CLI command implementations.
//!
//! Responsibilities:
//! - Implement CLI subcommand handlers for all Splunk operations
//! - Coordinate between CLI argument parsing and client crate APIs
//! - Handle command-specific output formatting via formatters
//!
//! Does NOT handle:
//! - Argument parsing (see args module)
//! - Direct REST API implementation (see client crate)
//! - Output format implementation details (see formatters module)
//!
//! Invariants:
//! - All commands use build_client_from_config for client construction
//! - All async commands respect cancellation tokens
//! - All output goes through shared formatters for consistency

pub mod alerts;
pub mod apps;
pub mod audit;
pub mod cluster;
pub mod completions;
pub mod config;
pub mod configs;
pub mod dashboards;
pub mod datamodels;
pub mod doctor;
pub mod forwarders;
pub mod health;
pub mod hec;
pub mod indexes;
pub mod inputs;
pub mod jobs;
pub mod kvstore;
pub mod license;
pub mod list_all;
pub mod logs;
pub mod lookups;
pub mod macros;
pub mod manpage;
pub mod roles;
pub mod saved_searches;
pub mod search;
pub mod search_peers;
pub mod shc;
pub mod transaction;
pub mod users;
pub mod workload;

use anyhow::{Context, Result};
use splunk_client::SplunkClient;

/// Build a SplunkClient from configuration.
///
/// This is a thin wrapper around the shared client builder in the client crate.
/// The shared implementation centralizes client construction to avoid duplication
/// between CLI and TUI.
///
/// # Arguments
/// * `config` - The loaded configuration containing connection and auth settings
/// * `no_cache` - Optional flag to disable client-side response caching
///
/// # Returns
/// A configured SplunkClient ready for API calls
///
/// # Errors
/// Returns an error if the client builder fails (e.g., invalid base_url)
pub fn build_client_from_config(
    config: &splunk_config::Config,
    no_cache: Option<bool>,
) -> Result<SplunkClient> {
    let mut builder = SplunkClient::builder().from_config(config);

    if no_cache == Some(true) {
        builder = builder.no_cache();
    }

    builder.build().map_err(|e| e.into())
}

/// Get the transaction manager for the current environment.
pub fn get_transaction_manager() -> Result<splunk_client::transaction::TransactionManager> {
    let proj_dirs = directories::ProjectDirs::from("", "", "splunk-tui")
        .context("Failed to determine project directories")?;
    let log_dir = proj_dirs.config_dir().join("transactions");
    Ok(splunk_client::transaction::TransactionManager::new(log_dir))
}

#[cfg(test)]
mod tests {
    use super::*;
    use secrecy::SecretString;

    #[test]
    fn test_build_client_from_config_with_api_token() {
        let config = splunk_config::Config::with_api_token(
            "https://splunk.example.com:8089".to_string(),
            SecretString::new("test-token".to_string().into()),
        );

        let client = build_client_from_config(&config, None);

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url(), "https://splunk.example.com:8089");
        assert!(client.is_api_token_auth());
    }

    #[test]
    fn test_build_client_from_config_with_session_token() {
        let config = splunk_config::Config::with_session_token(
            "https://splunk.example.com:8089".to_string(),
            "admin".to_string(),
            SecretString::new("test-password".to_string().into()),
        );

        let client = build_client_from_config(&config, None);

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url(), "https://splunk.example.com:8089");
        assert!(!client.is_api_token_auth());
    }
}
