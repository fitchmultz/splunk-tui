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
use crate::endpoints::search::SearchMode;
use crate::error::Result;
use crate::models::{SavedSearch, SearchJobResults, SearchJobStatus, ValidateSplResponse};
use splunk_config::constants::{
    DEFAULT_MAX_RESULTS, DEFAULT_MAX_WAIT_SECS, DEFAULT_POLL_INTERVAL_MS,
};

/// A request to execute a search job.
///
/// This struct bundles all search parameters to avoid the `too_many_arguments`
/// lint and provide a clean, builder-friendly API.
///
/// # What this struct handles:
/// - Query string and wait behavior
/// - Time bounds (earliest/latest)
/// - Result limits and search mode
/// - Real-time window configuration
///
/// # What this struct does NOT handle:
/// - Authentication or session management
/// - HTTP transport details
/// - Result parsing
///
/// # Invariants
/// - `realtime_window` is only meaningful when `search_mode` is `Some(SearchMode::Realtime)`
/// - `wait` controls client-side polling; the server may still process asynchronously
#[derive(Debug, Clone, Copy)]
pub struct SearchRequest<'a> {
    /// The SPL query to execute.
    pub query: &'a str,
    /// Whether to wait for the job to complete before returning.
    pub wait: bool,
    /// Optional earliest time bound (e.g., "-24h").
    pub earliest_time: Option<&'a str>,
    /// Optional latest time bound (e.g., "now").
    pub latest_time: Option<&'a str>,
    /// Maximum number of results to return.
    pub max_results: Option<u64>,
    /// Optional search mode (Normal or Realtime).
    pub search_mode: Option<SearchMode>,
    /// Optional real-time window in seconds (only for Realtime mode).
    pub realtime_window: Option<u64>,
}

impl<'a> SearchRequest<'a> {
    /// Create a new search request with the given query and wait flag.
    ///
    /// All optional fields default to `None`.
    pub fn new(query: &'a str, wait: bool) -> Self {
        Self {
            query,
            wait,
            earliest_time: None,
            latest_time: None,
            max_results: None,
            search_mode: None,
            realtime_window: None,
        }
    }

    /// Set the time bounds for the search.
    pub fn time_bounds(mut self, earliest: &'a str, latest: &'a str) -> Self {
        self.earliest_time = Some(earliest);
        self.latest_time = Some(latest);
        self
    }

    /// Set the maximum number of results to return.
    pub fn max_results(mut self, max: u64) -> Self {
        self.max_results = Some(max);
        self
    }

    /// Set the search mode.
    pub fn search_mode(mut self, mode: SearchMode) -> Self {
        self.search_mode = Some(mode);
        self
    }

    /// Set the real-time window in seconds.
    pub fn realtime_window(mut self, window: u64) -> Self {
        self.realtime_window = Some(window);
        self
    }

    /// Get the effective max results count, using the default if not set.
    fn effective_max_results(&self) -> u64 {
        self.max_results.unwrap_or(DEFAULT_MAX_RESULTS)
    }
}

/// Build `CreateJobOptions` from a search request.
///
/// When `force_wait_false` is true, the `wait` field is always set to `false`,
/// regardless of `request.wait`. This is used by `search_with_progress` to ensure
/// job creation is non-blocking so polling/progress remains a client concern.
fn build_create_job_options(
    request: &SearchRequest<'_>,
    force_wait_false: bool,
) -> endpoints::search::CreateJobOptions {
    endpoints::search::CreateJobOptions {
        wait: Some(if force_wait_false {
            false
        } else {
            request.wait
        }),
        earliest_time: request.earliest_time.map(|s| s.to_string()),
        latest_time: request.latest_time.map(|s| s.to_string()),
        max_count: request.max_results,
        search_mode: request.search_mode,
        realtime_window: request.realtime_window,
        ..Default::default()
    }
}

impl SplunkClient {
    /// Create and execute a search job, waiting for completion.
    ///
    /// # Arguments
    /// * `request` - The search request containing query, time bounds, and options
    pub async fn search(&self, request: SearchRequest<'_>) -> Result<Vec<serde_json::Value>> {
        let options = build_create_job_options(&request, false);
        let sid = self.create_search_job(request.query, &options).await?;

        if request.wait {
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
            .get_search_results(&sid, request.effective_max_results(), 0)
            .await?;

        Ok(results.results)
    }

    /// Create and execute a search job with optional progress reporting.
    ///
    /// When `request.wait` is true, this polls job status and can report `done_progress` (0.0–1.0).
    /// Progress reporting is a UI-layer concern; the callback is optional and may be `None`.
    ///
    /// This method is designed to allow the CLI to display progress bars without
    /// contaminating stdout, while keeping polling logic in the client library.
    ///
    /// Returns a tuple of (results, sid, total_count) where:
    /// - `results`: The search results as JSON values
    /// - `sid`: The search job ID for pagination or further operations
    /// - `total_count`: Optional total count of results (may be None if not available)
    ///
    /// # Arguments
    /// * `request` - The search request containing query, time bounds, and options
    /// * `progress_cb` - Optional callback for progress updates (0.0–1.0)
    ///
    /// # Invariants
    /// Job creation is always non-blocking (`CreateJobOptions.wait = Some(false)`),
    /// regardless of `request.wait`, so that polling/progress remains a client concern.
    /// `request.wait` only controls whether the client polls before fetching results.
    pub async fn search_with_progress(
        &self,
        request: SearchRequest<'_>,
        progress_cb: Option<&mut (dyn FnMut(f64) + Send)>,
    ) -> Result<(Vec<serde_json::Value>, String, Option<u64>)> {
        let options = build_create_job_options(&request, true);
        let sid = self.create_search_job(request.query, &options).await?;

        if request.wait {
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
            .get_search_results(&sid, request.effective_max_results(), 0)
            .await?;

        Ok((results.results, sid, results.total))
    }

    /// Create a search job without waiting for completion.
    pub async fn create_search_job(
        &self,
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
        &self,
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
    pub async fn get_job_status(&self, sid: &str) -> Result<SearchJobStatus> {
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
        &self,
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
    pub async fn create_saved_search(&self, name: &str, search: &str) -> Result<()> {
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
    pub async fn delete_saved_search(&self, name: &str) -> Result<()> {
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
    pub async fn get_saved_search(&self, name: &str) -> Result<SavedSearch> {
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
        &self,
        name: &str,
        search: Option<&str>,
        description: Option<&str>,
        disabled: Option<bool>,
    ) -> Result<()> {
        let params = endpoints::SavedSearchUpdateParams {
            search,
            description,
            disabled,
        };
        crate::retry_call!(
            self,
            __token,
            endpoints::update_saved_search(
                &self.http,
                &self.base_url,
                &__token,
                name,
                &params,
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
    pub async fn validate_spl(&self, search: &str) -> Result<ValidateSplResponse> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_search_request_new_defaults() {
        let req = SearchRequest::new("search index=main", true);
        assert_eq!(req.query, "search index=main");
        assert!(req.wait);
        assert_eq!(req.earliest_time, None);
        assert_eq!(req.latest_time, None);
        assert_eq!(req.max_results, None);
        assert_eq!(req.search_mode, None);
        assert_eq!(req.realtime_window, None);
    }

    #[test]
    fn test_search_request_builder_methods() {
        let req = SearchRequest::new("search index=main", false)
            .time_bounds("-24h", "now")
            .max_results(100)
            .search_mode(SearchMode::Realtime)
            .realtime_window(60);

        assert_eq!(req.earliest_time, Some("-24h"));
        assert_eq!(req.latest_time, Some("now"));
        assert_eq!(req.max_results, Some(100));
        assert_eq!(req.search_mode, Some(SearchMode::Realtime));
        assert_eq!(req.realtime_window, Some(60));
    }

    #[test]
    fn test_effective_max_results_default() {
        let req = SearchRequest::new("search index=main", true);
        assert_eq!(req.effective_max_results(), DEFAULT_MAX_RESULTS);
    }

    #[test]
    fn test_effective_max_results_explicit() {
        let req = SearchRequest::new("search index=main", true).max_results(500);
        assert_eq!(req.effective_max_results(), 500);
    }

    #[test]
    fn test_effective_max_results_zero() {
        let req = SearchRequest::new("search index=main", true).max_results(0);
        assert_eq!(req.effective_max_results(), 0);
    }

    #[test]
    fn test_build_create_job_options_respects_request_wait() {
        let req = SearchRequest::new("search index=main", true);
        let opts = build_create_job_options(&req, false);
        assert_eq!(opts.wait, Some(true));

        let req = SearchRequest::new("search index=main", false);
        let opts = build_create_job_options(&req, false);
        assert_eq!(opts.wait, Some(false));
    }

    #[test]
    fn test_build_create_job_options_forces_wait_false_when_flag_set() {
        // Even when request.wait is true, forcing wait false should override
        let req = SearchRequest::new("search index=main", true);
        let opts = build_create_job_options(&req, true);
        assert_eq!(opts.wait, Some(false));

        // Also verify when request.wait is false
        let req = SearchRequest::new("search index=main", false);
        let opts = build_create_job_options(&req, true);
        assert_eq!(opts.wait, Some(false));
    }

    #[test]
    fn test_build_create_job_options_maps_time_bounds() {
        let req = SearchRequest::new("search index=main", true).time_bounds("-24h", "now");
        let opts = build_create_job_options(&req, false);
        assert_eq!(opts.earliest_time, Some("-24h".to_string()));
        assert_eq!(opts.latest_time, Some("now".to_string()));
    }

    #[test]
    fn test_build_create_job_options_maps_other_fields() {
        let req = SearchRequest::new("search index=main", true)
            .max_results(100)
            .search_mode(SearchMode::Realtime)
            .realtime_window(60);

        let opts = build_create_job_options(&req, false);
        assert_eq!(opts.max_count, Some(100));
        assert_eq!(opts.search_mode, Some(SearchMode::Realtime));
        assert_eq!(opts.realtime_window, Some(60));
    }
}
