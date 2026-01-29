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
