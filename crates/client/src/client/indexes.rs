//! Index management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing indexes
//! - Creating new indexes
//! - Modifying existing indexes
//! - Deleting indexes
//!
//! # What this module does NOT handle:
//! - Low-level index endpoint HTTP calls (in [`crate::endpoints::indexes`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{CreateIndexParams, Index, ModifyIndexParams};

impl SplunkClient {
    /// List all indexes.
    pub async fn list_indexes(
        &self,
        count: Option<usize>,
        offset: Option<usize>,
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

    /// Create a new index with the specified parameters.
    pub async fn create_index(&self, params: &CreateIndexParams) -> Result<Index> {
        crate::retry_call!(
            self,
            __token,
            endpoints::create_index(
                &self.http,
                &self.base_url,
                &__token,
                params,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Modify an existing index.
    pub async fn modify_index(&self, name: &str, params: &ModifyIndexParams) -> Result<Index> {
        crate::retry_call!(
            self,
            __token,
            endpoints::modify_index(
                &self.http,
                &self.base_url,
                &__token,
                name,
                params,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Delete an index by name.
    pub async fn delete_index(&self, name: &str) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::delete_index(
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
}
