//! Cluster models for Splunk cluster management API.
//!
//! This module contains types for cluster configuration and peer status.

use serde::{Deserialize, Serialize};

/// Cluster information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClusterInfo {
    pub id: String,
    pub label: Option<String>,
    pub mode: String,
    pub manager_uri: Option<String>,
    pub replication_factor: Option<u32>,
    pub search_factor: Option<u32>,
    pub status: Option<String>,
    pub maintenance_mode: Option<bool>,
}

/// Cluster peer information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClusterPeer {
    pub id: String,
    pub label: Option<String>,
    pub status: String,
    pub peer_state: String,
    pub site: Option<String>,
    pub guid: String,
    pub host: String,
    pub port: u32,
    pub replication_count: Option<u32>,
    pub replication_status: Option<String>,
    pub bundle_replication_count: Option<u32>,
    #[serde(rename = "is_captain")]
    pub is_captain: Option<bool>,
}

/// Parameters for setting maintenance mode.
#[derive(Debug, Serialize)]
pub struct MaintenanceModeParams {
    /// Enable or disable maintenance mode.
    pub mode: bool,
}

/// Parameters for removing peers from the cluster.
#[derive(Debug, Serialize)]
pub struct RemovePeersParams {
    /// Comma-separated list of peer GUIDs to remove.
    pub peers: String,
}

/// Parameters for decommissioning a peer.
#[derive(Debug, Serialize)]
pub struct DecommissionPeerParams {
    /// Set to true to decommission the peer.
    pub decommission: bool,
}

/// Response from a cluster management operation.
#[derive(Debug, Deserialize)]
pub struct ClusterManagementResponse {
    /// Whether the operation was successful.
    pub success: bool,
    /// Optional message from the server.
    pub message: Option<String>,
}
