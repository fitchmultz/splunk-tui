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
        crate::retry_call!(
            self,
            __token,
            endpoints::get_server_info(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get system-wide health information.
    pub async fn get_health(&self) -> Result<SplunkHealth> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_health(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }
}
