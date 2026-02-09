//! Data model API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing data models
//! - Getting individual data model details (including JSON)
//!
//! # What this module does NOT handle:
//! - Low-level data model endpoint HTTP calls (in [`crate::endpoints::datamodels`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::DataModel;

impl SplunkClient {
    /// List all data models.
    pub async fn list_datamodels(
        &self,
        count: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<DataModel>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_datamodels(
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

    /// Get a data model by name, including its JSON definition.
    pub async fn get_datamodel(&self, name: &str) -> Result<DataModel> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_datamodel(
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
