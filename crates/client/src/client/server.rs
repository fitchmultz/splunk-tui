//! Server information API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Getting server information
//! - Getting system health status
//!
//! # What this module does NOT handle:
//! - Server configuration management (not yet implemented)
//! - Low-level server endpoint HTTP calls (in [`crate::endpoints::server`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{ServerInfo, SplunkHealth};

impl SplunkClient {
    /// Get server information.
    pub async fn get_server_info(&self) -> Result<ServerInfo> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("get_server_info"),
            |__token| async move {
                endpoints::get_server_info(
                    &self.http,
                    &self.base_url,
                    &__token,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }

    /// Get system-wide health information.
    pub async fn get_health(&self) -> Result<SplunkHealth> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("get_health"),
            |__token| async move {
                endpoints::get_health(
                    &self.http,
                    &self.base_url,
                    &__token,
                    self.max_retries,
                    self.metrics.as_ref(),
                    self.circuit_breaker.as_deref(),
                )
                .await
            },
        )
        .await
    }
}
