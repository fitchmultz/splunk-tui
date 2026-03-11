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
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("get_cluster_info"),
            |__token| async move {
                endpoints::get_cluster_info(
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

    /// Get cluster peer information.
    pub async fn get_cluster_peers(&self) -> Result<Vec<ClusterPeer>> {
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("get_cluster_peers"),
            |__token| async move {
                endpoints::get_cluster_peers(
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

    /// Set maintenance mode on the cluster manager.
    ///
    /// # Arguments
    ///
    /// * `enable` - true to enable maintenance mode, false to disable
    pub async fn set_maintenance_mode(&self, enable: bool) -> Result<ClusterManagementResponse> {
        let params = MaintenanceModeParams { mode: enable };
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("set_maintenance_mode"),
            |__token| {
                let params = params.clone();
                async move {
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
                }
            },
        )
        .await
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
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("rebalance_cluster"),
            |__token| async move {
                endpoints::rebalance_cluster(
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

    /// Remove peers from the cluster by their GUIDs.
    ///
    /// # Arguments
    ///
    /// * `peer_guids` - Slice of peer GUIDs to remove
    pub async fn remove_peers(&self, peer_guids: &[String]) -> Result<ClusterManagementResponse> {
        let params = RemovePeersParams {
            peers: peer_guids.join(","),
        };
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("remove_peers"),
            |__token| {
                let params = params.clone();
                async move {
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
                }
            },
        )
        .await
    }

    /// Decommission a peer by its name/GUID.
    ///
    /// # Arguments
    ///
    /// * `peer_name` - The peer name or GUID to decommission
    pub async fn decommission_peer(&self, peer_name: &str) -> Result<ClusterPeer> {
        let params = DecommissionPeerParams { decommission: true };
        self.execute_request(
            crate::client::request_executor::RequestPolicy::for_operation("decommission_peer"),
            |__token| {
                let params = params.clone();
                async move {
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
                }
            },
        )
        .await
    }
}
