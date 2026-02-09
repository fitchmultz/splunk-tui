//! Alert-related API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing fired alerts
//! - Getting fired alert details
//!
//! # What this module does NOT handle:
//! - Low-level alert endpoint HTTP calls (in [`crate::endpoints::alerts`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::FiredAlert;

impl SplunkClient {
    /// List all fired alerts.
    ///
    /// # Arguments
    /// * `count` - Maximum number of fired alerts to return
    /// * `offset` - Offset for pagination
    ///
    /// # Returns
    /// List of fired alerts
    pub async fn list_fired_alerts(
        &self,
        count: Option<usize>,
        offset: Option<usize>,
    ) -> Result<Vec<FiredAlert>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_fired_alerts(
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

    /// Get a specific fired alert by name.
    ///
    /// # Arguments
    /// * `name` - The name of the fired alert
    ///
    /// # Returns
    /// The `FiredAlert` if found, or `ClientError::NotFound` if it doesn't exist.
    pub async fn get_fired_alert(&self, name: &str) -> Result<FiredAlert> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_fired_alert(
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
