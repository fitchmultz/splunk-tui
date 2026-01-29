//! Main Splunk REST API client and API methods.
//!
//! This module provides the primary [`SplunkClient`] for interacting with the
//! Splunk Enterprise REST API. It automatically handles authentication and
//! session management.
//!
//! # Submodules
//! - [`builder`]: Client construction and configuration
//! - `session`: Session token management helpers (private module)
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

use tracing::debug;

use crate::auth::SessionManager;
use crate::endpoints;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{
    App, ClusterInfo, ClusterPeer, Index, KvStoreStatus, LicensePool, LicenseStack, LicenseUsage,
    LogEntry, LogParsingHealth, SavedSearch, SearchJobResults, SearchJobStatus, ServerInfo,
    SplunkHealth, User,
};
use splunk_config::constants::{
    DEFAULT_MAX_RESULTS, DEFAULT_MAX_WAIT_SECS, DEFAULT_POLL_INTERVAL_MS,
};

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
macro_rules! retry_call {
    ($self:expr, $token:ident, $call:expr) => {{
        let $token = $self.get_auth_token().await?;
        let result = $call;

        match result {
            Ok(data) => Ok(data),
            Err(ClientError::ApiError { status, .. })
                if (status == 401 || status == 403) && !$self.is_api_token_auth() =>
            {
                debug!(
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

    /// Create and execute a search job, waiting for completion.
    ///
    /// # Arguments
    /// * `query` - The SPL query to execute
    /// * `wait` - Whether to wait for the job to complete before returning
    /// * `earliest_time` - Optional earliest time bound (e.g., "-24h")
    /// * `latest_time` - Optional latest time bound (e.g., "now")
    /// * `max_results` - Maximum number of results to return
    pub async fn search(
        &mut self,
        query: &str,
        wait: bool,
        earliest_time: Option<&str>,
        latest_time: Option<&str>,
        max_results: Option<u64>,
    ) -> Result<Vec<serde_json::Value>> {
        let options = endpoints::search::CreateJobOptions {
            wait: Some(wait),
            earliest_time: earliest_time.map(|s| s.to_string()),
            latest_time: latest_time.map(|s| s.to_string()),
            max_count: max_results,
            ..Default::default()
        };

        let sid = self.create_search_job(query, &options).await?;

        if wait {
            let auth_token = self.get_auth_token().await?;
            endpoints::wait_for_job(
                &self.http,
                &self.base_url,
                &auth_token,
                &sid,
                DEFAULT_POLL_INTERVAL_MS,
                DEFAULT_MAX_WAIT_SECS,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await?;
        }

        let results = self
            .get_search_results(&sid, max_results.unwrap_or(DEFAULT_MAX_RESULTS), 0)
            .await?;

        Ok(results.results)
    }

    /// Create and execute a search job with optional progress reporting.
    ///
    /// When `wait` is true, this polls job status and can report `done_progress` (0.0–1.0).
    /// Progress reporting is a UI-layer concern; the callback is optional and may be `None`.
    ///
    /// This method is designed to allow the CLI to display progress bars without
    /// contaminating stdout, while keeping polling logic in the client library.
    ///
    /// Returns a tuple of (results, sid, total_count) where:
    /// - `results`: The search results as JSON values
    /// - `sid`: The search job ID for pagination or further operations
    /// - `total_count`: Optional total count of results (may be None if not available)
    pub async fn search_with_progress(
        &mut self,
        query: &str,
        wait: bool,
        earliest_time: Option<&str>,
        latest_time: Option<&str>,
        max_results: Option<u64>,
        progress_cb: Option<&mut (dyn FnMut(f64) + Send)>,
    ) -> Result<(Vec<serde_json::Value>, String, Option<u64>)> {
        let options = endpoints::search::CreateJobOptions {
            // Always create the job in non-blocking mode so callers can poll and show progress.
            wait: Some(false),
            earliest_time: earliest_time.map(|s| s.to_string()),
            latest_time: latest_time.map(|s| s.to_string()),
            max_count: max_results,
            ..Default::default()
        };

        let sid = self.create_search_job(query, &options).await?;

        if wait {
            let auth_token = self.get_auth_token().await?;
            endpoints::search::wait_for_job_with_progress(
                &self.http,
                &self.base_url,
                &auth_token,
                &sid,
                DEFAULT_POLL_INTERVAL_MS,
                DEFAULT_MAX_WAIT_SECS,
                self.max_retries,
                progress_cb,
                self.metrics.as_ref(),
            )
            .await?;
        }

        let results = self
            .get_search_results(&sid, max_results.unwrap_or(DEFAULT_MAX_RESULTS), 0)
            .await?;

        Ok((results.results, sid, results.total))
    }

    /// Create a search job without waiting for completion.
    pub async fn create_search_job(
        &mut self,
        query: &str,
        options: &endpoints::search::CreateJobOptions,
    ) -> Result<String> {
        retry_call!(
            self,
            __token,
            endpoints::search::create_job(
                &self.http,
                &self.base_url,
                &__token,
                query,
                options,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get results from a search job.
    pub async fn get_search_results(
        &mut self,
        sid: &str,
        count: u64,
        offset: u64,
    ) -> Result<SearchJobResults> {
        retry_call!(
            self,
            __token,
            endpoints::search::get_results(
                &self.http,
                &self.base_url,
                &__token,
                sid,
                Some(count),
                Some(offset),
                endpoints::search::OutputMode::Json,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get the status of a search job.
    pub async fn get_job_status(&mut self, sid: &str) -> Result<SearchJobStatus> {
        retry_call!(
            self,
            __token,
            endpoints::search::get_job_status(
                &self.http,
                &self.base_url,
                &__token,
                sid,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// List all search jobs.
    pub async fn list_jobs(
        &mut self,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<SearchJobStatus>> {
        retry_call!(
            self,
            __token,
            endpoints::list_jobs(
                &self.http,
                &self.base_url,
                &__token,
                count,
                offset,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Cancel a search job.
    pub async fn cancel_job(&mut self, sid: &str) -> Result<()> {
        retry_call!(
            self,
            __token,
            endpoints::cancel_job(
                &self.http,
                &self.base_url,
                &__token,
                sid,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Delete a search job.
    pub async fn delete_job(&mut self, sid: &str) -> Result<()> {
        retry_call!(
            self,
            __token,
            endpoints::delete_job(
                &self.http,
                &self.base_url,
                &__token,
                sid,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// List all indexes.
    pub async fn list_indexes(
        &mut self,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<Index>> {
        retry_call!(
            self,
            __token,
            endpoints::list_indexes(
                &self.http,
                &self.base_url,
                &__token,
                count,
                offset,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// List all saved searches.
    pub async fn list_saved_searches(
        &mut self,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<SavedSearch>> {
        retry_call!(
            self,
            __token,
            endpoints::list_saved_searches(
                &self.http,
                &self.base_url,
                &__token,
                count,
                offset,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Create a saved search.
    pub async fn create_saved_search(&mut self, name: &str, search: &str) -> Result<()> {
        retry_call!(
            self,
            __token,
            endpoints::create_saved_search(
                &self.http,
                &self.base_url,
                &__token,
                name,
                search,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Delete a saved search by name.
    pub async fn delete_saved_search(&mut self, name: &str) -> Result<()> {
        retry_call!(
            self,
            __token,
            endpoints::delete_saved_search(
                &self.http,
                &self.base_url,
                &__token,
                name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get a single saved search by name.
    ///
    /// # Arguments
    /// * `name` - The name of the saved search
    ///
    /// # Returns
    /// The `SavedSearch` if found, or `ClientError::NotFound` if it doesn't exist.
    pub async fn get_saved_search(&mut self, name: &str) -> Result<SavedSearch> {
        retry_call!(
            self,
            __token,
            endpoints::get_saved_search(
                &self.http,
                &self.base_url,
                &__token,
                name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// List all installed apps.
    pub async fn list_apps(&mut self, count: Option<u64>, offset: Option<u64>) -> Result<Vec<App>> {
        retry_call!(
            self,
            __token,
            endpoints::list_apps(
                &self.http,
                &self.base_url,
                &__token,
                count,
                offset,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get specific app details by name.
    pub async fn get_app(&mut self, app_name: &str) -> Result<App> {
        retry_call!(
            self,
            __token,
            endpoints::get_app(
                &self.http,
                &self.base_url,
                &__token,
                app_name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Enable an app by name.
    pub async fn enable_app(&mut self, app_name: &str) -> Result<()> {
        retry_call!(
            self,
            __token,
            endpoints::enable_app(
                &self.http,
                &self.base_url,
                &__token,
                app_name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Disable an app by name.
    pub async fn disable_app(&mut self, app_name: &str) -> Result<()> {
        retry_call!(
            self,
            __token,
            endpoints::disable_app(
                &self.http,
                &self.base_url,
                &__token,
                app_name,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// List all users.
    pub async fn list_users(
        &mut self,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<User>> {
        retry_call!(
            self,
            __token,
            endpoints::list_users(
                &self.http,
                &self.base_url,
                &__token,
                count,
                offset,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get server information.
    pub async fn get_server_info(&mut self) -> Result<ServerInfo> {
        retry_call!(
            self,
            __token,
            endpoints::get_server_info(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get system-wide health information.
    pub async fn get_health(&mut self) -> Result<SplunkHealth> {
        retry_call!(
            self,
            __token,
            endpoints::get_health(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get cluster information.
    pub async fn get_cluster_info(&mut self) -> Result<ClusterInfo> {
        retry_call!(
            self,
            __token,
            endpoints::get_cluster_info(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get cluster peer information.
    pub async fn get_cluster_peers(&mut self) -> Result<Vec<ClusterPeer>> {
        retry_call!(
            self,
            __token,
            endpoints::get_cluster_peers(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get license usage information.
    pub async fn get_license_usage(&mut self) -> Result<Vec<LicenseUsage>> {
        retry_call!(
            self,
            __token,
            endpoints::get_license_usage(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// List all license pools.
    pub async fn list_license_pools(&mut self) -> Result<Vec<LicensePool>> {
        retry_call!(
            self,
            __token,
            endpoints::list_license_pools(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// List all license stacks.
    pub async fn list_license_stacks(&mut self) -> Result<Vec<LicenseStack>> {
        retry_call!(
            self,
            __token,
            endpoints::list_license_stacks(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get KVStore status information.
    pub async fn get_kvstore_status(&mut self) -> Result<KvStoreStatus> {
        retry_call!(
            self,
            __token,
            endpoints::get_kvstore_status(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Check log parsing health by searching for parsing errors in internal logs.
    ///
    /// This method searches the `_internal` index for parsing-related errors
    /// from specific components (TuningParser, DateParserVerbose, Parser) and
    /// returns structured results about any issues found.
    pub async fn check_log_parsing_health(&mut self) -> Result<LogParsingHealth> {
        retry_call!(
            self,
            __token,
            endpoints::check_log_parsing_health(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get internal logs from Splunk.
    pub async fn get_internal_logs(
        &mut self,
        count: u64,
        earliest: Option<&str>,
    ) -> Result<Vec<LogEntry>> {
        retry_call!(
            self,
            __token,
            endpoints::get_internal_logs(
                &self.http,
                &self.base_url,
                &__token,
                count,
                earliest,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthStrategy;
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
