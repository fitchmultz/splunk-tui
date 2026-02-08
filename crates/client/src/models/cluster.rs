//! Cluster models for Splunk cluster management API.
//!
//! This module contains types for cluster configuration and peer status.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Cluster mode (manager or peer).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ClusterMode {
    Manager,
    Peer,
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for ClusterMode {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClusterMode::Manager => write!(f, "Manager"),
            ClusterMode::Peer => write!(f, "Peer"),
            ClusterMode::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Cluster status (enabled or disabled).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ClusterStatus {
    Enabled,
    Disabled,
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for ClusterStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ClusterStatus::Enabled => write!(f, "Enabled"),
            ClusterStatus::Disabled => write!(f, "Disabled"),
            ClusterStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Peer status (up or down).
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum PeerStatus {
    #[serde(rename = "Up")]
    Up,
    #[serde(rename = "Down")]
    Down,
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for PeerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeerStatus::Up => write!(f, "Up"),
            PeerStatus::Down => write!(f, "Down"),
            PeerStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Peer state in the cluster.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum PeerState {
    Searchable,
    Unsearchable,
    Streaming,
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for PeerState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            PeerState::Searchable => write!(f, "Searchable"),
            PeerState::Unsearchable => write!(f, "Unsearchable"),
            PeerState::Streaming => write!(f, "Streaming"),
            PeerState::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Replication status of a peer.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum ReplicationStatus {
    Complete,
    Pending,
    Failed,
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for ReplicationStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ReplicationStatus::Complete => write!(f, "Complete"),
            ReplicationStatus::Pending => write!(f, "Pending"),
            ReplicationStatus::Failed => write!(f, "Failed"),
            ReplicationStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Cluster information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClusterInfo {
    pub id: String,
    pub label: Option<String>,
    pub mode: ClusterMode,
    pub manager_uri: Option<String>,
    pub replication_factor: Option<u32>,
    pub search_factor: Option<u32>,
    pub status: Option<ClusterStatus>,
    pub maintenance_mode: Option<bool>,
}

/// Cluster peer information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClusterPeer {
    pub id: String,
    pub label: Option<String>,
    pub status: PeerStatus,
    pub peer_state: PeerState,
    pub site: Option<String>,
    pub guid: String,
    pub host: String,
    pub port: u32,
    pub replication_count: Option<u32>,
    pub replication_status: Option<ReplicationStatus>,
    pub bundle_replication_count: Option<u32>,
    #[serde(rename = "isCaptain")]
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

#[cfg(test)]
mod tests {
    use super::*;

    // ClusterMode tests
    #[test]
    fn cluster_mode_default_is_unknown() {
        assert_eq!(ClusterMode::default(), ClusterMode::Unknown);
    }

    #[test]
    fn cluster_mode_display_formats_correctly() {
        assert_eq!(ClusterMode::Manager.to_string(), "Manager");
        assert_eq!(ClusterMode::Peer.to_string(), "Peer");
        assert_eq!(ClusterMode::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn cluster_mode_deserializes_from_snake_case() {
        let manager: ClusterMode = serde_json::from_str("\"manager\"").unwrap();
        assert_eq!(manager, ClusterMode::Manager);

        let peer: ClusterMode = serde_json::from_str("\"peer\"").unwrap();
        assert_eq!(peer, ClusterMode::Peer);
    }

    #[test]
    fn cluster_mode_deserializes_unknown_variant() {
        let unknown: ClusterMode = serde_json::from_str("\"invalid_mode\"").unwrap();
        assert_eq!(unknown, ClusterMode::Unknown);
    }

    #[test]
    fn cluster_mode_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&ClusterMode::Manager).unwrap(),
            "\"manager\""
        );
        assert_eq!(
            serde_json::to_string(&ClusterMode::Peer).unwrap(),
            "\"peer\""
        );
    }

    // ClusterStatus tests
    #[test]
    fn cluster_status_default_is_unknown() {
        assert_eq!(ClusterStatus::default(), ClusterStatus::Unknown);
    }

    #[test]
    fn cluster_status_display_formats_correctly() {
        assert_eq!(ClusterStatus::Enabled.to_string(), "Enabled");
        assert_eq!(ClusterStatus::Disabled.to_string(), "Disabled");
        assert_eq!(ClusterStatus::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn cluster_status_deserializes_from_snake_case() {
        let enabled: ClusterStatus = serde_json::from_str("\"enabled\"").unwrap();
        assert_eq!(enabled, ClusterStatus::Enabled);

        let disabled: ClusterStatus = serde_json::from_str("\"disabled\"").unwrap();
        assert_eq!(disabled, ClusterStatus::Disabled);
    }

    #[test]
    fn cluster_status_deserializes_unknown_variant() {
        let unknown: ClusterStatus = serde_json::from_str("\"invalid_status\"").unwrap();
        assert_eq!(unknown, ClusterStatus::Unknown);
    }

    #[test]
    fn cluster_status_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&ClusterStatus::Enabled).unwrap(),
            "\"enabled\""
        );
        assert_eq!(
            serde_json::to_string(&ClusterStatus::Disabled).unwrap(),
            "\"disabled\""
        );
    }

    // PeerStatus tests
    #[test]
    fn peer_status_default_is_unknown() {
        assert_eq!(PeerStatus::default(), PeerStatus::Unknown);
    }

    #[test]
    fn peer_status_display_formats_correctly() {
        assert_eq!(PeerStatus::Up.to_string(), "Up");
        assert_eq!(PeerStatus::Down.to_string(), "Down");
        assert_eq!(PeerStatus::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn peer_status_deserializes_from_explicit_rename() {
        let up: PeerStatus = serde_json::from_str("\"Up\"").unwrap();
        assert_eq!(up, PeerStatus::Up);

        let down: PeerStatus = serde_json::from_str("\"Down\"").unwrap();
        assert_eq!(down, PeerStatus::Down);
    }

    #[test]
    fn peer_status_deserializes_unknown_variant() {
        let unknown: PeerStatus = serde_json::from_str("\"invalid_status\"").unwrap();
        assert_eq!(unknown, PeerStatus::Unknown);

        let unknown_lowercase: PeerStatus = serde_json::from_str("\"up\"").unwrap();
        assert_eq!(unknown_lowercase, PeerStatus::Unknown);
    }

    #[test]
    fn peer_status_serializes_with_explicit_rename() {
        assert_eq!(serde_json::to_string(&PeerStatus::Up).unwrap(), "\"Up\"");
        assert_eq!(
            serde_json::to_string(&PeerStatus::Down).unwrap(),
            "\"Down\""
        );
    }

    // PeerState tests
    #[test]
    fn peer_state_default_is_unknown() {
        assert_eq!(PeerState::default(), PeerState::Unknown);
    }

    #[test]
    fn peer_state_display_formats_correctly() {
        assert_eq!(PeerState::Searchable.to_string(), "Searchable");
        assert_eq!(PeerState::Unsearchable.to_string(), "Unsearchable");
        assert_eq!(PeerState::Streaming.to_string(), "Streaming");
        assert_eq!(PeerState::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn peer_state_deserializes_from_snake_case() {
        let searchable: PeerState = serde_json::from_str("\"searchable\"").unwrap();
        assert_eq!(searchable, PeerState::Searchable);

        let unsearchable: PeerState = serde_json::from_str("\"unsearchable\"").unwrap();
        assert_eq!(unsearchable, PeerState::Unsearchable);

        let streaming: PeerState = serde_json::from_str("\"streaming\"").unwrap();
        assert_eq!(streaming, PeerState::Streaming);
    }

    #[test]
    fn peer_state_deserializes_unknown_variant() {
        let unknown: PeerState = serde_json::from_str("\"invalid_state\"").unwrap();
        assert_eq!(unknown, PeerState::Unknown);
    }

    #[test]
    fn peer_state_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&PeerState::Searchable).unwrap(),
            "\"searchable\""
        );
        assert_eq!(
            serde_json::to_string(&PeerState::Unsearchable).unwrap(),
            "\"unsearchable\""
        );
        assert_eq!(
            serde_json::to_string(&PeerState::Streaming).unwrap(),
            "\"streaming\""
        );
    }

    // ReplicationStatus tests
    #[test]
    fn replication_status_default_is_unknown() {
        assert_eq!(ReplicationStatus::default(), ReplicationStatus::Unknown);
    }

    #[test]
    fn replication_status_display_formats_correctly() {
        assert_eq!(ReplicationStatus::Complete.to_string(), "Complete");
        assert_eq!(ReplicationStatus::Pending.to_string(), "Pending");
        assert_eq!(ReplicationStatus::Failed.to_string(), "Failed");
        assert_eq!(ReplicationStatus::Unknown.to_string(), "Unknown");
    }

    #[test]
    fn replication_status_deserializes_from_snake_case() {
        let complete: ReplicationStatus = serde_json::from_str("\"complete\"").unwrap();
        assert_eq!(complete, ReplicationStatus::Complete);

        let pending: ReplicationStatus = serde_json::from_str("\"pending\"").unwrap();
        assert_eq!(pending, ReplicationStatus::Pending);

        let failed: ReplicationStatus = serde_json::from_str("\"failed\"").unwrap();
        assert_eq!(failed, ReplicationStatus::Failed);
    }

    #[test]
    fn replication_status_deserializes_unknown_variant() {
        let unknown: ReplicationStatus = serde_json::from_str("\"invalid_status\"").unwrap();
        assert_eq!(unknown, ReplicationStatus::Unknown);
    }

    #[test]
    fn replication_status_serializes_to_snake_case() {
        assert_eq!(
            serde_json::to_string(&ReplicationStatus::Complete).unwrap(),
            "\"complete\""
        );
        assert_eq!(
            serde_json::to_string(&ReplicationStatus::Pending).unwrap(),
            "\"pending\""
        );
        assert_eq!(
            serde_json::to_string(&ReplicationStatus::Failed).unwrap(),
            "\"failed\""
        );
    }

    // Struct tests with new enum fields
    #[test]
    fn cluster_info_deserializes_with_enum_fields() {
        let json = r#"{
            "id": "cluster-1",
            "label": "Test Cluster",
            "mode": "manager",
            "manager_uri": "https://manager:8089",
            "replication_factor": 3,
            "search_factor": 2,
            "status": "enabled",
            "maintenance_mode": false
        }"#;

        let info: ClusterInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.id, "cluster-1");
        assert_eq!(info.mode, ClusterMode::Manager);
        assert_eq!(info.status, Some(ClusterStatus::Enabled));
    }

    #[test]
    fn cluster_info_handles_unknown_mode() {
        let json = r#"{
            "id": "cluster-1",
            "mode": "unknown_mode"
        }"#;

        let info: ClusterInfo = serde_json::from_str(json).unwrap();
        assert_eq!(info.mode, ClusterMode::Unknown);
        assert!(info.status.is_none());
    }

    #[test]
    fn cluster_peer_deserializes_with_enum_fields() {
        let json = r#"{
            "id": "peer-1",
            "label": "Peer One",
            "status": "Up",
            "peer_state": "searchable",
            "site": "site1",
            "guid": "abc123",
            "host": "peer1.example.com",
            "port": 8089,
            "replication_count": 5,
            "replication_status": "complete"
        }"#;

        let peer: ClusterPeer = serde_json::from_str(json).unwrap();
        assert_eq!(peer.id, "peer-1");
        assert_eq!(peer.status, PeerStatus::Up);
        assert_eq!(peer.peer_state, PeerState::Searchable);
        assert_eq!(peer.replication_status, Some(ReplicationStatus::Complete));
    }

    #[test]
    fn cluster_peer_handles_unknown_status() {
        let json = r#"{
            "id": "peer-1",
            "status": "Unknown",
            "peer_state": "unknown_state",
            "guid": "abc123",
            "host": "peer1.example.com",
            "port": 8089
        }"#;

        let peer: ClusterPeer = serde_json::from_str(json).unwrap();
        assert_eq!(peer.status, PeerStatus::Unknown);
        assert_eq!(peer.peer_state, PeerState::Unknown);
    }

    #[test]
    fn cluster_peer_handles_missing_replication_status() {
        let json = r#"{
            "id": "peer-1",
            "status": "Up",
            "peer_state": "streaming",
            "guid": "abc123",
            "host": "peer1.example.com",
            "port": 8089
        }"#;

        let peer: ClusterPeer = serde_json::from_str(json).unwrap();
        assert_eq!(peer.peer_state, PeerState::Streaming);
        assert!(peer.replication_status.is_none());
    }

    #[test]
    fn enums_are_cloneable() {
        let mode = ClusterMode::Manager;
        let cloned = mode.clone();
        assert_eq!(mode, cloned);

        let status = PeerStatus::Up;
        let cloned = status.clone();
        assert_eq!(status, cloned);
    }

    #[test]
    fn enums_implement_partial_eq() {
        assert_eq!(ClusterMode::Manager, ClusterMode::Manager);
        assert_ne!(ClusterMode::Manager, ClusterMode::Peer);

        assert_eq!(PeerStatus::Up, PeerStatus::Up);
        assert_ne!(PeerStatus::Up, PeerStatus::Down);
    }
}
