//! Cluster management API methods for [`SplunkClient`].
//!
//! # What this module handles:
//! - Getting cluster information
//! - Listing cluster peers
//! - Cluster configuration and management operations
//!
//! # What this module does NOT handle:
//! - Low-level cluster endpoint HTTP calls (in [`crate::endpoints::cluster`])

use crate::client::SplunkClient;
use crate::endpoints;
use crate::error::Result;
use crate::models::{
    ClusterInfo, ClusterManagementResponse, ClusterPeer, DecommissionPeerParams,
    MaintenanceModeParams, RemovePeersParams,
};

impl SplunkClient {
    /// Get cluster information.
    pub async fn get_cluster_info(&self) -> Result<ClusterInfo> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_cluster_info(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Get cluster peer information.
    pub async fn get_cluster_peers(&self) -> Result<Vec<ClusterPeer>> {
        crate::retry_call!(
            self,
            __token,
            endpoints::get_cluster_peers(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Set maintenance mode on the cluster manager.
    ///
    /// # Arguments
    ///
    /// * `enable` - true to enable maintenance mode, false to disable
    pub async fn set_maintenance_mode(&self, enable: bool) -> Result<ClusterManagementResponse> {
        let params = MaintenanceModeParams { mode: enable };
        crate::retry_call!(
            self,
            __token,
            endpoints::set_maintenance_mode(
                &self.http,
                &self.base_url,
                &__token,
                &params,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Enable maintenance mode on the cluster manager.
    pub async fn enable_maintenance_mode(&self) -> Result<ClusterManagementResponse> {
        self.set_maintenance_mode(true).await
    }

    /// Disable maintenance mode on the cluster manager.
    pub async fn disable_maintenance_mode(&self) -> Result<ClusterManagementResponse> {
        self.set_maintenance_mode(false).await
    }

    /// Rebalance primary buckets across all peers.
    pub async fn rebalance_cluster(&self) -> Result<ClusterManagementResponse> {
        crate::retry_call!(
            self,
            __token,
            endpoints::rebalance_cluster(
                &self.http,
                &self.base_url,
                &__token,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Remove peers from the cluster by their GUIDs.
    ///
    /// # Arguments
    ///
    /// * `peer_guids` - Slice of peer GUIDs to remove
    pub async fn remove_peers(&self, peer_guids: &[String]) -> Result<ClusterManagementResponse> {
        let params = RemovePeersParams {
            peers: peer_guids.join(","),
        };
        crate::retry_call!(
            self,
            __token,
            endpoints::remove_peers(
                &self.http,
                &self.base_url,
                &__token,
                &params,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }

    /// Decommission a peer by its name/GUID.
    ///
    /// # Arguments
    ///
    /// * `peer_name` - The peer name or GUID to decommission
    pub async fn decommission_peer(&self, peer_name: &str) -> Result<ClusterPeer> {
        let params = DecommissionPeerParams { decommission: true };
        crate::retry_call!(
            self,
            __token,
            endpoints::decommission_peer(
                &self.http,
                &self.base_url,
                &__token,
                peer_name,
                &params,
                self.max_retries,
                self.metrics.as_ref(),
                self.circuit_breaker.as_deref(),
            )
            .await
        )
    }
}
