//! Index management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing indexes
//!
//! # What this module does NOT handle:
//! - Creating or modifying indexes (not yet implemented)
//! - Low-level index endpoint HTTP calls (in [`crate::endpoints::indexes`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::Index;

impl SplunkClient {
    /// List all indexes.
    pub async fn list_indexes(
        &mut self,
        count: Option<u64>,
        offset: Option<u64>,
    ) -> Result<Vec<Index>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_indexes(
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
