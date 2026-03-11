//! Dashboard API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing dashboards
//! - Getting individual dashboard details (including XML)
//!
//! # What this module does NOT handle:
//! - Low-level dashboard endpoint HTTP calls (in [`crate::endpoints::dashboards`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::Dashboard;

impl SplunkClient {
    /// List all dashboards.
    pub async fn list_dashboards(
        &self,
        count: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<Dashboard>> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("list_dashboards"),
            |__token| async move {
                endpoints::list_dashboards(
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

    /// Get a dashboard by name, including its XML definition.
    pub async fn get_dashboard(&self, name: &str) -> Result<Dashboard> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("get_dashboard"),
            |__token| async move {
                endpoints::get_dashboard(
                    &self.http,
                    &self.base_url,
                    &__token,
                    name,
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
