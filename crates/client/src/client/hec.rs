//! HEC (HTTP Event Collector) API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Sending single events via HEC
//! - Sending batches of events (JSON array or NDJSON format)
//! - Checking HEC health status
//! - Querying acknowledgment status for guaranteed delivery
//!
//! # What this module does NOT handle:
//! - Low-level HEC endpoint HTTP calls (see [`crate::endpoints::hec`])
//! - HEC token management (handled by CLI/config)
//!
//! # Important Notes
//! - HEC uses a separate URL (typically port 8088) from the REST API (port 8089)
//! - HEC uses a separate token with "Splunk" auth prefix (not "Bearer")
//! - These methods accept HEC-specific URL/token as parameters rather than
//!   using the client's configured base_url and auth

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::hec::{HecAckStatus, HecBatchResponse, HecEvent, HecHealth, HecResponse};

impl SplunkClient {
    /// Send a single event via HEC.
    ///
    /// # Arguments
    /// * `hec_url` - The HEC endpoint URL (e.g., "https://localhost:8088")
    /// * `hec_token` - The HEC authentication token
    /// * `event` - The event to send
    ///
    /// # Returns
    /// The HEC response containing status and optional acknowledgment ID
    ///
    /// # Errors
    /// Returns `ClientError` if the request fails
    ///
    /// # Example
    /// ```ignore
    /// use splunk_client::models::HecEvent;
    ///
    /// let event = HecEvent::new(serde_json::json!({"message": "Hello Splunk"}));
    /// let response = client.hec_send_event("https://localhost:8088", "token", &event).await?;
    /// ```
    pub async fn hec_send_event(
        &self,
        hec_url: &str,
        hec_token: &str,
        event: &HecEvent,
    ) -> Result<HecResponse> {
        endpoints::hec::send_event(
            &self.http,
            hec_url,
            hec_token,
            event,
            self.max_retries,
            self.metrics.as_ref(),
        )
        .await
    }

    /// Send a batch of events via HEC.
    ///
    /// # Arguments
    /// * `hec_url` - The HEC endpoint URL (e.g., "https://localhost:8088")
    /// * `hec_token` - The HEC authentication token
    /// * `events` - The events to send
    /// * `use_ndjson` - Use NDJSON format instead of JSON array
    ///
    /// # Returns
    /// The HEC batch response containing status and optional acknowledgment IDs
    ///
    /// # Errors
    /// Returns `ClientError` if the request fails
    ///
    /// # Example
    /// ```ignore
    /// use splunk_client::models::HecEvent;
    ///
    /// let events = vec![
    ///     HecEvent::new(serde_json::json!({"message": "Event 1"})),
    ///     HecEvent::new(serde_json::json!({"message": "Event 2"})),
    /// ];
    /// let response = client.hec_send_batch("https://localhost:8088", "token", &events, false).await?;
    /// ```
    pub async fn hec_send_batch(
        &self,
        hec_url: &str,
        hec_token: &str,
        events: &[HecEvent],
        use_ndjson: bool,
    ) -> Result<HecBatchResponse> {
        endpoints::hec::send_batch(
            &self.http,
            hec_url,
            hec_token,
            events,
            use_ndjson,
            self.max_retries,
            self.metrics.as_ref(),
        )
        .await
    }

    /// Check HEC health status.
    ///
    /// # Arguments
    /// * `hec_url` - The HEC endpoint URL (e.g., "https://localhost:8088")
    /// * `hec_token` - The HEC authentication token
    ///
    /// # Returns
    /// The HEC health status
    ///
    /// # Errors
    /// Returns `ClientError` if the request fails
    ///
    /// # Example
    /// ```ignore
    /// let health = client.hec_health_check("https://localhost:8088", "token").await?;
    /// println!("HEC is healthy: {}", health.is_healthy());
    /// ```
    pub async fn hec_health_check(&self, hec_url: &str, hec_token: &str) -> Result<HecHealth> {
        endpoints::hec::health_check(
            &self.http,
            hec_url,
            hec_token,
            self.max_retries,
            self.metrics.as_ref(),
        )
        .await
    }

    /// Check HEC acknowledgment status for guaranteed delivery.
    ///
    /// When HEC acknowledgments are enabled, this method checks whether
    /// events have been successfully indexed.
    ///
    /// # Arguments
    /// * `hec_url` - The HEC endpoint URL (e.g., "https://localhost:8088")
    /// * `hec_token` - The HEC authentication token
    /// * `ack_ids` - List of acknowledgment IDs to check
    ///
    /// # Returns
    /// The acknowledgment status for each ID
    ///
    /// # Errors
    /// Returns `ClientError` if the request fails or acknowledgments are disabled
    ///
    /// # Example
    /// ```ignore
    /// let ack_ids = vec![123, 124, 125];
    /// let status = client.hec_check_acks("https://localhost:8088", "token", &ack_ids).await?;
    /// println!("All indexed: {}", status.all_indexed());
    /// ```
    pub async fn hec_check_acks(
        &self,
        hec_url: &str,
        hec_token: &str,
        ack_ids: &[u64],
    ) -> Result<HecAckStatus> {
        endpoints::hec::check_ack_status(
            &self.http,
            hec_url,
            hec_token,
            ack_ids,
            self.max_retries,
            self.metrics.as_ref(),
        )
        .await
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::auth::AuthStrategy;
    use secrecy::SecretString;

    fn create_test_client() -> SplunkClient {
        let strategy = AuthStrategy::ApiToken {
            token: SecretString::new("test-token".to_string().into()),
        };

        SplunkClient::builder()
            .base_url("https://localhost:8089".to_string())
            .auth_strategy(strategy)
            .build()
            .unwrap()
    }

    #[test]
    fn test_hec_methods_exist() {
        // This test just verifies the methods compile and are callable
        // Actual functionality is tested in the endpoints module
        let client = create_test_client();

        // Verify the client has the HEC methods
        // We can't call them without a mock server, but we can verify they exist
        assert_eq!(client.max_retries, 3); // Default value
    }
}
