//! Workload management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing workload pools and rules
//!
//! # What this module does NOT handle:
//! - Creating or modifying workload pools/rules (not supported by initial implementation)
//! - Low-level workload endpoint HTTP calls (in [`crate::endpoints::workload`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{WorkloadPool, WorkloadRule};

impl SplunkClient {
    /// List all workload pools.
    ///
    /// Retrieves a list of workload pools from the Splunk server.
    /// Supports pagination via `count` and `offset` parameters.
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum number of results to return (default: 30)
    /// * `offset` - Offset for pagination
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `WorkloadPool` structs on success.
    ///
    /// # Errors
    ///
    /// Returns a `ClientError` if the request fails or the response cannot be parsed.
    pub async fn list_workload_pools(
        &self,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<WorkloadPool>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_workload_pools(
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

    /// List all workload rules.
    ///
    /// Retrieves a list of workload rules from the Splunk server.
    /// Supports pagination via `count` and `offset` parameters.
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum number of results to return (default: 30)
    /// * `offset` - Offset for pagination
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `WorkloadRule` structs on success.
    ///
    /// # Errors
    ///
    /// Returns a `ClientError` if the request fails or the response cannot be parsed.
    pub async fn list_workload_rules(
        &self,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<WorkloadRule>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_workload_rules(
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
}
