//! Job management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing search jobs
//! - Cancelling search jobs
//! - Deleting search jobs
//!
//! # What this module does NOT handle:
//! - Creating search jobs (in [`crate::client::search`])
//! - Getting search results (in [`crate::client::search`])
//! - Low-level job endpoint HTTP calls (in [`crate::endpoints::jobs`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::SearchJobStatus;

impl SplunkClient {
    /// List all search jobs.
    pub async fn list_jobs(
        &self,
        count: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<SearchJobStatus>> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("list_jobs"),
            |__token| async move {
                endpoints::list_jobs(
                    &self.http,
                    &self.base_url,
                    &__token,
                    count,
                    offset,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Cancel a search job.
    pub async fn cancel_job(&self, sid: &str) -> Result<()> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("cancel_job"),
            |__token| async move {
                endpoints::cancel_job(
                    &self.http,
                    &self.base_url,
                    &__token,
                    sid,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Delete a search job.
    pub async fn delete_job(&self, sid: &str) -> Result<()> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("delete_job"),
            |__token| async move {
                endpoints::delete_job(
                    &self.http,
                    &self.base_url,
                    &__token,
                    sid,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }
}
