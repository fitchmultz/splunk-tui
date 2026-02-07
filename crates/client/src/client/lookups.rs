//! Lookup table management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing lookup table files
//! - Downloading lookup table files (CSV content)
//! - Uploading/replacing lookup table files
//! - Deleting lookup table files
//!
//! # What this module does NOT handle:
//! - KV store lookups (different endpoint)
//! - Low-level lookup endpoint HTTP calls (see [`crate::endpoints::lookups`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{LookupTable, UploadLookupParams};

impl SplunkClient {
    /// List all lookup table files.
    ///
    /// Returns CSV-based lookup files stored in Splunk.
    /// KV store lookups are managed via a different endpoint.
    pub async fn list_lookup_tables(
        &self,
        count: Option<usize>,
        offset: Option<usize>,
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

    /// Download a lookup table file as raw CSV content.
    ///
    /// # Arguments
    /// * `name` - The lookup name
    /// * `app` - Optional app namespace (defaults to "search")
    /// * `owner` - Optional owner namespace (defaults to "-" for all users)
    ///
    /// # Returns
    /// The raw CSV content as a string
    pub async fn download_lookup_table(
        &self,
        name: &str,
        app: Option<&str>,
        owner: Option<&str>,
    ) -> Result<String> {
        crate::retry_call!(
            self,
            __token,
            endpoints::download_lookup_table(
                &self.http,
                &self.base_url,
                &__token,
                name,
                app,
                owner,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Upload or replace a lookup table file.
    ///
    /// Note: Upload operations cannot be retried due to the request body
    /// being consumed on the first attempt.
    ///
    /// # Arguments
    /// * `params` - Upload parameters including name, filename, and content
    ///
    /// # Returns
    /// The created/updated lookup table metadata
    pub async fn upload_lookup_table(&self, params: &UploadLookupParams) -> Result<LookupTable> {
        // Get auth token (no retry for upload due to body consumption)
        let token = self.get_auth_token().await?;
        endpoints::upload_lookup_table(
            &self.http,
            &self.base_url,
            &token,
            params,
            self.max_retries,
            self.metrics.as_ref(),
        )
        .await
    }

    /// Delete a lookup table file.
    ///
    /// # Arguments
    /// * `name` - The lookup name to delete
    /// * `app` - Optional app namespace (defaults to "search")
    /// * `owner` - Optional owner namespace (defaults to "-" for all users)
    pub async fn delete_lookup_table(
        &self,
        name: &str,
        app: Option<&str>,
        owner: Option<&str>,
    ) -> Result<()> {
        crate::retry_call!(
            self,
            __token,
            endpoints::delete_lookup_table(
                &self.http,
                &self.base_url,
                &__token,
                name,
                app,
                owner,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }
}
