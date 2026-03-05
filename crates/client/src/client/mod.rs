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
//! - `dashboards`: Dashboard management methods
//! - `datamodels`: Data model management methods
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
pub mod cache;
pub mod circuit_breaker;
mod session;

// API method submodules
mod alerts;
mod apps;
mod audit;
mod capabilities;
mod cluster;
mod configs;
mod dashboards;
mod datamodels;
mod forwarders;
pub mod health;
mod hec;
mod indexes;
mod inputs;
mod jobs;
mod kvstore;
mod license;
mod logs;
mod lookups;
pub mod macros;
mod roles;
pub mod search;
mod search_peers;
mod server;
mod shc;
mod users;
mod workload;

use crate::auth::SessionManager;
use crate::client::circuit_breaker::CircuitBreaker;
use crate::metrics::MetricsCollector;
use std::sync::Arc;

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
            // Handle classified auth errors (SessionExpired, Unauthorized, AuthFailed)
            // as well as legacy ApiError with 401/403 status codes
            Err(ref e) if e.is_auth_error() && !$self.is_api_token_auth() => {
                ::tracing::debug!(
                    "Auth error ({}), clearing session and re-authenticating...",
                    e
                );
                $self.session_manager.clear_session().await;
                let $token = $self.get_auth_token().await?;
                let retry_result = $call;
                // Enrich error with username if retry also failed
                retry_result.map_err(|err| $self.enrich_auth_error(err))
            }
            Err(e) => Err($self.enrich_auth_error(e)),
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
    pub(crate) metrics: Option<MetricsCollector>,
    /// Response cache for GET requests.
    pub(crate) cache: cache::ResponseCache,
    /// Circuit breaker for resilient API calls.
    pub(crate) circuit_breaker: Option<Arc<CircuitBreaker>>,
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

    /// Create a client from configuration with optional auto-login.
    ///
    /// This is a convenience method that builds a client from the provided configuration
    /// and automatically logs in if using session token authentication.
    ///
    /// For session token auth, this will perform the initial login.
    /// For API token auth, no login is needed.
    ///
    /// # Arguments
    ///
    /// * `config` - The loaded configuration containing connection and auth settings
    ///
    /// # Returns
    ///
    /// A configured and authenticated `SplunkClient` ready for API calls
    ///
    /// # Errors
    ///
    /// Returns an error if client construction fails or if login fails for
    /// session token authentication.
    ///
    /// # Example
    ///
    /// ```rust,ignore
    /// use splunk_client::SplunkClient;
    /// use splunk_config::Config;
    ///
    /// let config = Config::default();
    /// let client = SplunkClient::from_config(&config).await?;
    /// ```
    pub async fn from_config(config: &splunk_config::Config) -> crate::error::Result<Self> {
        let client = Self::builder().from_config(config).build()?;

        if !client.is_api_token_auth() {
            client.login().await?;
        }
        Ok(client)
    }

    /// Get a reference to the response cache.
    pub fn cache(&self) -> &cache::ResponseCache {
        &self.cache
    }

    /// Get a mutable reference to the response cache.
    pub fn cache_mut(&mut self) -> &mut cache::ResponseCache {
        &mut self.cache
    }

    /// Disable response caching for this client.
    pub fn disable_cache(&mut self) {
        self.cache.disable();
    }

    /// Enable response caching for this client.
    pub fn enable_cache(&mut self) {
        self.cache.enable();
    }

    /// Get cache statistics.
    pub fn cache_stats(&self) -> cache::CacheStats {
        self.cache.stats()
    }

    /// Clear all cached responses.
    pub async fn clear_cache(&self) {
        self.cache.invalidate_all().await;
    }

    /// Get the transaction manager for the current environment.
    pub fn transaction_manager(
        &self,
    ) -> crate::error::Result<crate::transaction::TransactionManager> {
        let proj_dirs = directories::ProjectDirs::from("", "", "splunk-tui").ok_or_else(|| {
            crate::error::ClientError::ValidationError(
                "Failed to determine project directories".into(),
            )
        })?;
        let log_dir = proj_dirs.config_dir().join("transactions");
        Ok(crate::transaction::TransactionManager::new(log_dir))
    }

    /// Begin a new transaction for multi-step operations.
    pub fn begin_transaction(&self) -> crate::transaction::Transaction {
        crate::transaction::Transaction::new()
    }

    /// Commit a transaction, executing all staged operations.
    ///
    /// If any operation fails, the transaction is automatically rolled back.
    pub async fn commit_transaction(
        &self,
        transaction: &crate::transaction::Transaction,
    ) -> crate::error::Result<()> {
        let manager = self.transaction_manager()?;
        manager.validate(self, transaction).await?;
        manager.commit(self, transaction).await
    }

    /// Enrich auth-related errors with username from the auth strategy.
    ///
    /// This replaces "unknown" username in SessionExpired errors with the actual
    /// username, making error messages more actionable for operators with multiple accounts.
    fn enrich_auth_error(&self, error: crate::error::ClientError) -> crate::error::ClientError {
        if matches!(error, crate::error::ClientError::SessionExpired { .. }) {
            let username = match self.session_manager.strategy() {
                crate::auth::AuthStrategy::SessionToken { username, .. } => username.clone(),
                crate::auth::AuthStrategy::ApiToken { .. } => {
                    crate::auth::API_TOKEN_USERNAME.to_string()
                }
            };
            error.with_username(&username)
        } else {
            error
        }
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
