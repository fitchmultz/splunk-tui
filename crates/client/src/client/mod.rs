//! Main Splunk REST API client and API methods.
//!
//! This module provides the primary [`SplunkClient`] for interacting with the
//! Splunk Enterprise REST API. It automatically handles authentication and
//! session management.
//!
//! # Submodules
//! - [`builder`]: Client construction and configuration
//! - `session`: Session token management helpers (private module)
//! - `search`: Search-related methods
//! - `jobs`: Job management methods
//! - `indexes`: Index management methods
//! - `apps`: App management methods
//! - `users`: User management methods
//! - `server`: Server info and health methods
//! - `cluster`: Cluster info methods
//! - `license`: License methods
//! - `kvstore`: KVStore methods
//! - `logs`: Log parsing and internal logs methods
//!
//! # What this module does NOT handle:
//! - Direct HTTP request implementation (delegated to [`crate::endpoints`])
//! - Low-level session token storage (delegated to [`crate::auth::SessionManager`])
//! - Authentication strategy configuration (handled by [`builder::SplunkClientBuilder`])
//!
//! # Invariants
//! - All API methods handle 401/403 authentication errors by refreshing the session
//!   and retrying once (for session-based authentication only; API tokens do not trigger retries)
//! - The `retry_call!` macro centralizes this retry pattern across all API methods

pub mod builder;
mod session;

// API method submodules
mod apps;
mod cluster;
mod configs;
mod forwarders;
mod indexes;
mod inputs;
mod jobs;
mod kvstore;
mod license;
mod logs;
mod search;
mod search_peers;
mod server;
mod users;

use crate::auth::SessionManager;
use crate::metrics::MetricsCollector;

/// Macro to wrap an async API call with automatic session retry on 401/403 errors.
///
/// This macro centralizes the authentication retry pattern used across all API methods.
/// When a 401 or 403 error is received and the client is using session-based auth
/// (not API token auth), it clears the session, re-authenticates, and retries the call once.
///
/// # Usage
///
/// ```ignore
/// retry_call!(self, __token, endpoints::some_endpoint(&self.http, &self.base_url, __token, arg1, arg2).await)
/// ```
///
/// The placeholder `__token` will be replaced with the actual auth token.
#[macro_export]
macro_rules! retry_call {
    ($self:expr, $token:ident, $call:expr) => {{
        let $token = $self.get_auth_token().await?;
        let result = $call;

        match result {
            Ok(data) => Ok(data),
            Err($crate::error::ClientError::ApiError { status, .. })
                if (status == 401 || status == 403) && !$self.is_api_token_auth() =>
            {
                ::tracing::debug!(
                    "Session expired (status {}), clearing and re-authenticating...",
                    status
                );
                $self.session_manager.clear_session();
                let $token = $self.get_auth_token().await?;
                $call
            }
            Err(e) => Err(e),
        }
    }};
}

/// Splunk REST API client.
///
/// This client provides methods for interacting with the Splunk Enterprise
/// REST API. It automatically handles authentication and session management.
///
/// # Creating a Client
///
/// Use [`SplunkClient::builder()`] to create a new client:
///
/// ```rust,ignore
/// use splunk_client::{SplunkClient, AuthStrategy};
/// use secrecy::SecretString;
///
/// let client = SplunkClient::builder()
///     .base_url("https://localhost:8089".to_string())
///     .auth_strategy(AuthStrategy::ApiToken {
///         token: SecretString::new("my-token".to_string().into()),
///     })
///     .build()?;
/// ```
///
/// # Authentication
///
/// The client supports two authentication strategies:
/// - `AuthStrategy::SessionToken`: Username/password with automatic session management
/// - `AuthStrategy::ApiToken`: Static API token (no session management needed)
#[derive(Debug)]
pub struct SplunkClient {
    pub(crate) http: reqwest::Client,
    pub(crate) base_url: String,
    pub(crate) session_manager: SessionManager,
    pub(crate) max_retries: usize,
    pub(crate) session_ttl_seconds: u64,
    pub(crate) session_expiry_buffer_seconds: u64,
    pub(crate) metrics: Option<MetricsCollector>,
}

impl SplunkClient {
    /// Create a new client builder.
    ///
    /// This is the entry point for constructing a [`SplunkClient`].
    pub fn builder() -> builder::SplunkClientBuilder {
        builder::SplunkClientBuilder::new()
    }

    /// Get the base URL.
    pub fn base_url(&self) -> &str {
        &self.base_url
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthStrategy;
    use crate::error::ClientError;
    use secrecy::SecretString;

    #[test]
    fn test_client_builder_with_api_token() {
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new("test-token".to_string().into()),
        };

        let client = SplunkClient::builder()
            .base_url("https://localhost:8089".to_string())
            .auth_strategy(strategy)
            .build();

        assert!(client.is_ok());
        let client = client.unwrap();
        assert_eq!(client.base_url(), "https://localhost:8089");
        assert!(client.is_api_token_auth());
    }

    #[test]
    fn test_client_builder_missing_base_url() {
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new("test-token".to_string().into()),
        };

        let client = SplunkClient::builder().auth_strategy(strategy).build();

        assert!(matches!(client.unwrap_err(), ClientError::InvalidUrl(_)));
    }

    #[test]
    fn test_client_builder_normalizes_base_url() {
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new("test-token".to_string().into()),
        };

        let client = SplunkClient::builder()
            .base_url("https://localhost:8089/".to_string())
            .auth_strategy(strategy)
            .build()
            .unwrap();

        assert_eq!(client.base_url(), "https://localhost:8089");
    }

    #[test]
    fn test_skip_verify_with_https_url() {
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new("test-token".to_string().into()),
        };

        // Should succeed with HTTPS URL
        let client = SplunkClient::builder()
            .base_url("https://localhost:8089".to_string())
            .auth_strategy(strategy)
            .skip_verify(true)
            .build();

        assert!(client.is_ok());
    }

    #[test]
    fn test_skip_verify_with_http_url() {
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new("test-token".to_string().into()),
        };

        // Should succeed but log warning about ineffective skip_verify
        let client = SplunkClient::builder()
            .base_url("http://localhost:8089".to_string())
            .auth_strategy(strategy)
            .skip_verify(true)
            .build();

        assert!(client.is_ok());
    }
}
