//! Main Splunk REST API client.

use secrecy::ExposeSecret;
use std::time::Duration;
use tracing::debug;

use crate::auth::SessionManager;
use crate::endpoints;
use crate::error::{ClientError, Result};

/// Macro to wrap an async API call with automatic session retry on 401/403 errors.
///
/// This macro centralizes the authentication retry pattern used across all API methods.
/// When a 401 or 403 error is received and the client is using session-based auth
/// (not API token auth), it clears the session, re-authenticates, and retries the call once.
///
/// Usage:
/// ```ignore
/// retry_call!(self, __token, endpoints::some_endpoint(&self.http, &self.base_url, __token, arg1, arg2).await)
/// ```
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

use crate::models::{
    App, ClusterInfo, ClusterPeer, Index, KvStoreStatus, LicensePool, LicenseStack, LicenseUsage,
    LogEntry, LogParsingHealth, SavedSearch, SearchJobResults, SearchJobStatus, ServerInfo,
    SplunkHealth, User,
};

/// Builder for creating a new SplunkClient.
pub struct SplunkClientBuilder {
    base_url: Option<String>,
    auth_strategy: Option<crate::auth::AuthStrategy>,
    skip_verify: bool,
    timeout: Duration,
    max_retries: usize,
    session_ttl_seconds: u64,
    session_expiry_buffer_seconds: u64,
}

impl Default for SplunkClientBuilder {
    fn default() -> Self {
        Self {
            base_url: None,
            auth_strategy: None,
            skip_verify: false,
            timeout: Duration::from_secs(30),
            max_retries: 3,
            session_ttl_seconds: 3600,
            session_expiry_buffer_seconds: 60,
        }
    }
}

impl SplunkClientBuilder {
    /// Create a new builder.
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the base URL of the Splunk server.
    pub fn base_url(mut self, url: String) -> Self {
        self.base_url = Some(url);
        self
    }

    /// Set the authentication strategy.
    pub fn auth_strategy(mut self, strategy: crate::auth::AuthStrategy) -> Self {
        self.auth_strategy = Some(strategy);
        self
    }

    /// Set whether to skip TLS verification.
    pub fn skip_verify(mut self, skip: bool) -> Self {
        self.skip_verify = skip;
        self
    }

    /// Set the request timeout.
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// Set the maximum number of retries.
    pub fn max_retries(mut self, retries: usize) -> Self {
        self.max_retries = retries;
        self
    }

    /// Set the session TTL in seconds.
    pub fn session_ttl_seconds(mut self, ttl: u64) -> Self {
        self.session_ttl_seconds = ttl;
        self
    }

    /// Set the session expiry buffer in seconds.
    pub fn session_expiry_buffer_seconds(mut self, buffer: u64) -> Self {
        self.session_expiry_buffer_seconds = buffer;
        self
    }

    /// Normalize a base URL by removing trailing slashes.
    ///
    /// This prevents double slashes when concatenating with endpoint paths.
    /// Examples:
    /// - "https://localhost:8089/" -> "https://localhost:8089"
    /// - "https://localhost:8089" -> "https://localhost:8089"
    /// - "https://example.com:8089//" -> "https://example.com:8089"
    fn normalize_base_url(url: String) -> String {
        url.trim_end_matches('/').to_string()
    }

    /// Build the client.
    pub fn build(self) -> Result<SplunkClient> {
        let base_url = self
            .base_url
            .ok_or_else(|| ClientError::InvalidUrl("base_url is required".to_string()))?;
        let base_url = Self::normalize_base_url(base_url);

        let auth_strategy = self
            .auth_strategy
            .ok_or_else(|| ClientError::AuthFailed("auth_strategy is required".to_string()))?;

        let mut http_builder = reqwest::Client::builder()
            .timeout(self.timeout)
            .redirect(reqwest::redirect::Policy::limited(5));

        if self.skip_verify {
            let is_https = base_url.starts_with("https://");
            if is_https {
                http_builder = http_builder.danger_accept_invalid_certs(true);
            } else {
                // skip_verify only affects TLS certificate verification.
                // It has no effect on HTTP connections since there is no TLS layer.
                tracing::warn!(
                    "skip_verify=true has no effect on HTTP URLs. TLS verification only applies to HTTPS connections."
                );
            }
        }

        let http = http_builder.build()?;

        Ok(SplunkClient {
            http,
            base_url,
            session_manager: SessionManager::new(auth_strategy),
            max_retries: self.max_retries,
            session_ttl_seconds: self.session_ttl_seconds,
            session_expiry_buffer_seconds: self.session_expiry_buffer_seconds,
        })
    }
}

/// Splunk REST API client.
///
/// This client provides methods for interacting with the Splunk Enterprise
/// REST API. It automatically handles authentication and session management.
///
/// All API methods handle 401/403 authentication errors by refreshing the session
/// and retrying once (for session-based authentication only; API tokens do not trigger retries).
#[derive(Debug)]
pub struct SplunkClient {
    http: reqwest::Client,
    base_url: String,
    session_manager: SessionManager,
    max_retries: usize,
    session_ttl_seconds: u64,
    session_expiry_buffer_seconds: u64,
}

impl SplunkClient {
    /// Create a new client builder.
    pub fn builder() -> SplunkClientBuilder {
        SplunkClientBuilder::new()
    }

    /// Login with username/password to get a session token.
    pub async fn login(&mut self) -> Result<String> {
        if let crate::auth::AuthStrategy::SessionToken { username, password } =
            self.session_manager.strategy()
        {
            let token = endpoints::login(
                &self.http,
                &self.base_url,
                username,
                password.expose_secret(),
                self.max_retries,
            )
            .await?;

            // Use configured session TTL and buffer for proactive refresh
            let token_clone = token.clone();
            self.session_manager.set_session_token(
                token_clone,
                Some(self.session_ttl_seconds),
                Some(self.session_expiry_buffer_seconds),
            );

            Ok(token)
        } else {
            Err(ClientError::AuthFailed(
                "Cannot login with API token auth strategy".to_string(),
            ))
        }
    }

    /// Create and execute a search job, waiting for completion.
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
                500,
                300,
                self.max_retries,
            )
            .await?;
        }

        let results = self
            .get_search_results(&sid, max_results.unwrap_or(1000), 0)
            .await?;

        Ok(results.results)
    }

    /// Create and execute a search job with optional progress reporting.
    ///
    /// - When `wait` is true, this polls job status and can report `done_progress` (0.0–1.0).
    /// - Progress reporting is UI-layer concern; the callback is optional and may be `None`.
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
                500,
                300,
                self.max_retries,
                progress_cb,
            )
            .await?;
        }

        let results = self
            .get_search_results(&sid, max_results.unwrap_or(1000), 0)
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
            )
            .await
        )
    }

    /// Cancel a search job.
    pub async fn cancel_job(&mut self, sid: &str) -> Result<()> {
        retry_call!(
            self,
            __token,
            endpoints::cancel_job(&self.http, &self.base_url, &__token, sid, self.max_retries,)
                .await
        )
    }

    /// Delete a search job.
    pub async fn delete_job(&mut self, sid: &str) -> Result<()> {
        retry_call!(
            self,
            __token,
            endpoints::delete_job(&self.http, &self.base_url, &__token, sid, self.max_retries,)
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
            )
            .await
        )
    }

    /// List all saved searches.
    pub async fn list_saved_searches(&mut self) -> Result<Vec<SavedSearch>> {
        retry_call!(
            self,
            __token,
            endpoints::list_saved_searches(&self.http, &self.base_url, &__token, self.max_retries,)
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
            )
            .await
        )
    }

    /// Enable an app by name.
    pub async fn enable_app(&mut self, app_name: &str) -> Result<()> {
        retry_call!(
            self,
            __token,
            endpoints::update_app(
                &self.http,
                &self.base_url,
                &__token,
                app_name,
                false,
                self.max_retries,
            )
            .await
        )
    }

    /// Disable an app by name.
    pub async fn disable_app(&mut self, app_name: &str) -> Result<()> {
        retry_call!(
            self,
            __token,
            endpoints::update_app(
                &self.http,
                &self.base_url,
                &__token,
                app_name,
                true,
                self.max_retries,
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
            )
            .await
        )
    }

    /// Get server information.
    pub async fn get_server_info(&mut self) -> Result<ServerInfo> {
        retry_call!(
            self,
            __token,
            endpoints::get_server_info(&self.http, &self.base_url, &__token, self.max_retries,)
                .await
        )
    }

    /// Get system-wide health information.
    pub async fn get_health(&mut self) -> Result<SplunkHealth> {
        retry_call!(
            self,
            __token,
            endpoints::get_health(&self.http, &self.base_url, &__token, self.max_retries,).await
        )
    }

    /// Get cluster information.
    pub async fn get_cluster_info(&mut self) -> Result<ClusterInfo> {
        retry_call!(
            self,
            __token,
            endpoints::get_cluster_info(&self.http, &self.base_url, &__token, self.max_retries,)
                .await
        )
    }

    /// Get cluster peer information.
    pub async fn get_cluster_peers(&mut self) -> Result<Vec<ClusterPeer>> {
        retry_call!(
            self,
            __token,
            endpoints::get_cluster_peers(&self.http, &self.base_url, &__token, self.max_retries,)
                .await
        )
    }

    /// Get license usage information.
    pub async fn get_license_usage(&mut self) -> Result<Vec<LicenseUsage>> {
        retry_call!(
            self,
            __token,
            endpoints::get_license_usage(&self.http, &self.base_url, &__token, self.max_retries,)
                .await
        )
    }

    /// List all license pools.
    pub async fn list_license_pools(&mut self) -> Result<Vec<LicensePool>> {
        retry_call!(
            self,
            __token,
            endpoints::list_license_pools(&self.http, &self.base_url, &__token, self.max_retries,)
                .await
        )
    }

    /// List all license stacks.
    pub async fn list_license_stacks(&mut self) -> Result<Vec<LicenseStack>> {
        retry_call!(
            self,
            __token,
            endpoints::list_license_stacks(&self.http, &self.base_url, &__token, self.max_retries,)
                .await
        )
    }

    /// Get KVStore status information.
    pub async fn get_kvstore_status(&mut self) -> Result<KvStoreStatus> {
        retry_call!(
            self,
            __token,
            endpoints::get_kvstore_status(&self.http, &self.base_url, &__token, self.max_retries,)
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
            )
            .await
        )
    }

    /// Get the current authentication token, logging in if necessary.
    ///
    /// Proactively refreshes the session if it will expire soon (within the buffer window)
    /// to prevent race conditions where the token expires during an API call.
    async fn get_auth_token(&mut self) -> Result<String> {
        // For API token auth, just return the token
        if self.session_manager.is_api_token()
            && let Some(token) = self.session_manager.get_bearer_token()
        {
            return Ok(token.to_string());
        }

        // For session auth, check if we need to login (expired OR will expire soon)
        if self.session_manager.is_session_expired() || self.session_manager.session_expires_soon()
        {
            self.login().await?;
        }

        self.session_manager
            .get_bearer_token()
            .map(|s| s.to_string())
            .ok_or_else(|| ClientError::SessionExpired)
    }

    /// Check if the client is using API token authentication.
    pub fn is_api_token_auth(&self) -> bool {
        self.session_manager.is_api_token()
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
    fn test_normalize_base_url_trailing_slash() {
        let input = "https://localhost:8089/".to_string();
        let expected = "https://localhost:8089";
        assert_eq!(SplunkClientBuilder::normalize_base_url(input), expected);
    }

    #[test]
    fn test_normalize_base_url_no_trailing_slash() {
        let input = "https://localhost:8089".to_string();
        let expected = "https://localhost:8089";
        assert_eq!(SplunkClientBuilder::normalize_base_url(input), expected);
    }

    #[test]
    fn test_normalize_base_url_multiple_trailing_slashes() {
        let input = "https://example.com:8089//".to_string();
        let expected = "https://example.com:8089";
        assert_eq!(SplunkClientBuilder::normalize_base_url(input), expected);
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
