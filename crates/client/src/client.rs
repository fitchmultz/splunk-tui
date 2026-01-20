//! Main Splunk REST API client.

use secrecy::ExposeSecret;
use std::time::Duration;

use crate::auth::SessionManager;
use crate::endpoints;
use crate::error::{ClientError, Result};
use crate::models::{ClusterInfo, ClusterPeer, Index, SearchJobResults, SearchJobStatus};

/// Builder for creating a new SplunkClient.
pub struct SplunkClientBuilder {
    base_url: Option<String>,
    auth_strategy: Option<crate::auth::AuthStrategy>,
    skip_verify: bool,
    timeout: Duration,
    max_retries: usize,
}

impl Default for SplunkClientBuilder {
    fn default() -> Self {
        Self {
            base_url: None,
            auth_strategy: None,
            skip_verify: false,
            timeout: Duration::from_secs(30),
            max_retries: 3,
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

    /// Build the client.
    pub fn build(self) -> Result<SplunkClient> {
        let base_url = self
            .base_url
            .ok_or_else(|| ClientError::InvalidUrl("base_url is required".to_string()))?;

        let auth_strategy = self
            .auth_strategy
            .ok_or_else(|| ClientError::AuthFailed("auth_strategy is required".to_string()))?;

        let mut http_builder = reqwest::Client::builder()
            .timeout(self.timeout)
            .redirect(reqwest::redirect::Policy::limited(5));

        if self.skip_verify {
            let https_only = base_url.starts_with("https://");
            if https_only {
                http_builder = http_builder.danger_accept_invalid_certs(true);
            }
        }

        let http = http_builder.build()?;

        Ok(SplunkClient {
            http,
            base_url,
            session_manager: SessionManager::new(auth_strategy),
            max_retries: self.max_retries,
        })
    }
}

/// Splunk REST API client.
///
/// This client provides methods for interacting with the Splunk Enterprise
/// REST API. It automatically handles authentication and session management.
#[derive(Debug)]
#[allow(dead_code)]
pub struct SplunkClient {
    http: reqwest::Client,
    base_url: String,
    session_manager: SessionManager,
    max_retries: usize,
}

impl SplunkClient {
    /// Create a new client builder.
    pub fn builder() -> SplunkClientBuilder {
        SplunkClientBuilder::new()
    }

    /// Ensure we have an active authentication token.
    /// For API token auth, this is a no-op.
    /// For session auth, this will login if needed or if the session is expired.
    #[allow(dead_code)]
    async fn ensure_authenticated(&self) -> Result<String> {
        // If using API token, we don't need to manage sessions
        if self.session_manager.is_api_token()
            && let Some(token) = self.session_manager.get_bearer_token()
        {
            return Ok(token.to_string());
        }

        // Check if we have a valid session token
        if let Some(token) = self.session_manager.get_bearer_token()
            && !self.session_manager.is_session_expired()
        {
            return Ok(token.to_string());
        }

        // Need to login - this requires mutable access to the session manager
        // Since we can't have &mut self in async methods easily, we need to handle this differently
        // For now, return an error indicating authentication is needed
        Err(ClientError::SessionExpired)
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

            // Default session TTL is 1 hour
            let token_clone = token.clone();
            self.session_manager
                .set_session_token(token_clone, Some(3600));

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

    /// Create a search job without waiting for completion.
    pub async fn create_search_job(
        &mut self,
        query: &str,
        options: &endpoints::search::CreateJobOptions,
    ) -> Result<String> {
        let auth_token = self.get_auth_token().await?;

        endpoints::search::create_job(
            &self.http,
            &self.base_url,
            &auth_token,
            query,
            options,
            self.max_retries,
        )
        .await
    }

    /// Get results from a search job.
    pub async fn get_search_results(
        &mut self,
        sid: &str,
        count: u64,
        offset: u64,
    ) -> Result<SearchJobResults> {
        let auth_token = self.get_auth_token().await?;

        endpoints::search::get_results(
            &self.http,
            &self.base_url,
            &auth_token,
            sid,
            Some(count),
            Some(offset),
            endpoints::search::OutputMode::Json,
            self.max_retries,
        )
        .await
    }

    /// Get the status of a search job.
    pub async fn get_job_status(&mut self, sid: &str) -> Result<SearchJobStatus> {
        let auth_token = self.get_auth_token().await?;

        endpoints::search::get_job_status(
            &self.http,
            &self.base_url,
            &auth_token,
            sid,
            self.max_retries,
        )
        .await
    }

    /// List all search jobs.
    pub async fn list_jobs(
        &mut self,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<SearchJobStatus>> {
        let auth_token = self.get_auth_token().await?;

        endpoints::list_jobs(
            &self.http,
            &self.base_url,
            &auth_token,
            count,
            offset,
            self.max_retries,
        )
        .await
    }

    /// Cancel a search job.
    pub async fn cancel_job(&mut self, sid: &str) -> Result<()> {
        let auth_token = self.get_auth_token().await?;

        endpoints::cancel_job(
            &self.http,
            &self.base_url,
            &auth_token,
            sid,
            self.max_retries,
        )
        .await
    }

    /// Delete a search job.
    pub async fn delete_job(&mut self, sid: &str) -> Result<()> {
        let auth_token = self.get_auth_token().await?;

        endpoints::delete_job(
            &self.http,
            &self.base_url,
            &auth_token,
            sid,
            self.max_retries,
        )
        .await
    }

    /// List all indexes.
    pub async fn list_indexes(
        &mut self,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<Index>> {
        let auth_token = self.get_auth_token().await?;

        endpoints::list_indexes(
            &self.http,
            &self.base_url,
            &auth_token,
            count,
            offset,
            self.max_retries,
        )
        .await
    }

    /// Get cluster information.
    pub async fn get_cluster_info(&mut self) -> Result<ClusterInfo> {
        let auth_token = self.get_auth_token().await?;

        endpoints::get_cluster_info(&self.http, &self.base_url, &auth_token, self.max_retries).await
    }

    /// Get cluster peer information.
    pub async fn get_cluster_peers(&mut self) -> Result<Vec<ClusterPeer>> {
        let auth_token = self.get_auth_token().await?;

        endpoints::get_cluster_peers(&self.http, &self.base_url, &auth_token, self.max_retries)
            .await
    }

    /// Get the current authentication token, logging in if necessary.
    async fn get_auth_token(&mut self) -> Result<String> {
        // For API token auth, just return the token
        if self.session_manager.is_api_token()
            && let Some(token) = self.session_manager.get_bearer_token()
        {
            return Ok(token.to_string());
        }

        // For session auth, check if we need to login
        if self.session_manager.is_session_expired() {
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
}
