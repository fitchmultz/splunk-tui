//! Lookup table management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing lookup table files
//!
//! # What this module does NOT handle:
//! - Lookup file content upload/download
//! - KV store lookups (different endpoint)
//! - Low-level lookup endpoint HTTP calls (see [`crate::endpoints::lookups`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::LookupTable;

impl SplunkClient {
    /// List all lookup table files.
    ///
    /// Returns CSV-based lookup files stored in Splunk.
    /// KV store lookups are managed via a different endpoint.
    pub async fn list_lookup_tables(
        &mut self,
        count: Option<u32>,
        offset: Option<u32>,
    ) -> Result<Vec<LookupTable>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_lookup_tables(
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
