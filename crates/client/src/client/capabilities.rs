//! Capability management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Listing capabilities
//!
//! # What this module does NOT handle:
//! - Authentication and session management (in [`crate::client::session`])
//! - Low-level capability endpoint HTTP calls (in [`crate::endpoints::capabilities`])
//! - Creating/modifying/deleting capabilities (capabilities are read-only in Splunk)

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::Capability;

impl SplunkClient {
    /// List all capabilities.
    ///
    /// Capabilities are read-only in Splunk. They represent the set of
    /// permissions that can be assigned to roles.
    pub async fn list_capabilities(&mut self) -> Result<Vec<Capability>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::list_capabilities(
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
