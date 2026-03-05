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
        crate::retry_call!(
            self,
            __token,
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
        )
    }

    /// Get a dashboard by name, including its XML definition.
    pub async fn get_dashboard(&self, name: &str) -> Result<Dashboard> {
        crate::retry_call!(
            self,
            __token,
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
        )
    }
}
