//! Cluster management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Getting cluster information
//! - Listing cluster peers
//!
//! # What this module does NOT handle:
//! - Cluster configuration or management operations (not yet implemented)
//! - Low-level cluster endpoint HTTP calls (in [`crate::endpoints::cluster`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{ClusterInfo, ClusterPeer};

impl SplunkClient {
    /// Get cluster information.
    pub async fn get_cluster_info(&mut self) -> Result<ClusterInfo> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_cluster_info(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
            )
            .await
        )
    }

    /// Get cluster peer information.
    pub async fn get_cluster_peers(&mut self) -> Result<Vec<ClusterPeer>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_cluster_peers(
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
