//! Search peer management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing distributed search peers
//!
//! # What this module does NOT handle:
//! - Creating or modifying search peers (not supported by this API)
//! - Low-level search peer endpoint HTTP calls (in [`crate::endpoints::search_peers`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::SearchPeer;

impl SplunkClient {
    /// List all distributed search peers.
    ///
    /// Retrieves a list of search peers configured on the search head.
    /// Supports pagination via `count` and `offset` parameters.
    ///
    /// # Arguments
    ///
    /// * `count` - Maximum number of results to return (default: 30)
    /// * `offset` - Offset for pagination
    ///
    /// # Returns
    ///
    /// A `Result` containing a vector of `SearchPeer` structs on success.
    ///
    /// # Errors
    ///
    /// Returns a `ClientError` if the request fails or the response cannot be parsed.
    pub async fn list_search_peers(
        &self,
        count: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<SearchPeer>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_search_peers(
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
