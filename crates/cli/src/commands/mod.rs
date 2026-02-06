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
pub mod config;
pub mod configs;
pub mod dashboards;
pub mod datamodels;
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
pub mod roles;
pub mod saved_searches;
pub mod search;
pub mod search_peers;
pub mod shc;
pub mod users;
pub mod workload;

use anyhow::Result;
use splunk_client::SplunkClient;

/// Build a SplunkClient from configuration.
///
/// This is a thin wrapper around the shared client builder in the client crate.
/// The shared implementation centralizes client construction to avoid duplication
/// between CLI and TUI.
///
/// # Arguments
/// * `config` - The loaded configuration containing connection and auth settings
///
/// # Returns
/// A configured SplunkClient ready for API calls
///
/// # Errors
/// Returns an error if the client builder fails (e.g., invalid base_url)
pub fn build_client_from_config(config: &splunk_config::Config) -> Result<SplunkClient> {
    SplunkClient::builder()
        .from_config(config)
        .build()
        .map_err(|e| e.into())
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

        let client = build_client_from_config(&config);

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

        let client = build_client_from_config(&config);

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url(), "https://splunk.example.com:8089");
        assert!(!client.is_api_token_auth());
    }
}
