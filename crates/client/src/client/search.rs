//! Search-related API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Creating and executing search jobs
//! - Retrieving search results
//! - Managing saved searches
//! - SPL syntax validation
//!
//! # What this module does NOT handle:
//! - Low-level search endpoint HTTP calls (in [`crate::endpoints::search`])
//! - Search result parsing (handled by endpoint functions)

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{SavedSearch, SearchJobResults, SearchJobStatus, ValidateSplResponse};
use splunk_config::constants::{
    DEFAULT_MAX_RESULTS, DEFAULT_MAX_WAIT_SECS, DEFAULT_POLL_INTERVAL_MS,
};

impl SplunkClient {
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
        crate::retry_call!(
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
        crate::retry_call!(
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
        crate::retry_call!(
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

    /// List all saved searches.
    pub async fn list_saved_searches(
        &mut self,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<SavedSearch>> {
        crate::retry_call!(
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
        crate::retry_call!(
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
        crate::retry_call!(
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
        crate::retry_call!(
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

    /// Update an existing saved search.
    ///
    /// Only provided fields are updated; omitted fields retain their current values.
    ///
    /// # Arguments
    /// * `name` - The name of the saved search to update
    /// * `search` - Optional new search query (SPL)
    /// * `description` - Optional new description
    /// * `disabled` - Optional enable/disable flag
    ///
    /// # Returns
    /// Ok(()) on success, or `ClientError::NotFound` if the saved search doesn't exist.
    pub async fn update_saved_search(
        &mut self,
        name: &str,
        search: Option<&str>,
        description: Option<&str>,
        disabled: Option<bool>,
    ) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::update_saved_search(
                &self.http,
                &self.base_url,
                &__token,
                name,
                search,
                description,
                disabled,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Validate SPL syntax without executing the search.
    ///
    /// Sends the query to Splunk's search parser endpoint to check for
    /// syntax errors and warnings before running the search.
    ///
    /// # Arguments
    /// * `search` - The SPL query to validate
    ///
    /// # Returns
    /// * `Ok(ValidateSplResponse)` - Validation result with errors/warnings
    /// * `Err(ClientError)` - Transport or API error
    ///
    /// # Example
    /// ```rust,no_run
    /// # use splunk_client::SplunkClient;
    /// # async fn example(client: &mut SplunkClient) -> Result<(), Box<dyn std::error::Error>> {
    /// let result = client.validate_spl("search index=main | stats count").await?;
    /// if result.valid {
    ///     println!("SPL is valid!");
    /// } else {
    ///     for error in result.errors {
    ///         println!("Error: {}", error.message);
    ///     }
    /// }
    /// # Ok(())
    /// # }
    /// ```
    pub async fn validate_spl(&mut self, search: &str) -> Result<ValidateSplResponse> {
        crate::retry_call!(
            self,
            __token,
            endpoints::search::validate_spl(
                &self.http,
                &self.base_url,
                &__token,
                search,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }
}
