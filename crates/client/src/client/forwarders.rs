//! Forwarder management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing deployment clients (forwarders)
//!
//! # What this module does NOT handle:
//! - Creating or modifying forwarders (not supported by Splunk REST API)
//! - Low-level forwarder endpoint HTTP calls (in [`crate::endpoints::forwarders`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::Forwarder;

impl SplunkClient {
    /// List all deployment clients (forwarders).
    ///
    /// Retrieves a list of forwarders that have checked in with the deployment server.
    /// Supports pagination via `count` and `offset` parameters.
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum number of results to return (default: 30)
    /// * `offset` - Offset for pagination
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `Forwarder` structs on success.
    ///
    /// # Errors
    ///
    /// Returns a `ClientError` if the request fails or the response cannot be parsed.
    pub async fn list_forwarders(
        &self,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<Forwarder>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_forwarders(
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
