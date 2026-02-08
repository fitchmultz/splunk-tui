//! Search peer models for Splunk distributed search API.
//!
//! This module contains types for listing and managing Splunk distributed search peers.
//! Search peers are searchable targets configured on search heads, distinct from
//! indexer cluster peers which manage data replication.
//!
//! # What this module handles:
//! - Deserialization of search peer data from Splunk REST API
//! - Type-safe representation of search peer metadata
//!
//! # What this module does NOT handle:
//! - Direct HTTP API calls (see [`crate::endpoints::search_peers`])
//! - Client-side filtering or searching of peers
//! - Modifying search peer configuration

use serde::{Deserialize, Serialize};
use std::fmt;

/// The connection status of a search peer.
///
/// Represents whether a Splunk search peer is currently reachable
/// and functioning correctly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "PascalCase")]
#[derive(Default)]
pub enum SearchPeerStatus {
    /// The peer is connected and responding normally.
    Up,
    /// The peer is unreachable or not responding.
    Down,
    /// The peer status is unknown or unrecognized.
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for SearchPeerStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            SearchPeerStatus::Up => write!(f, "Up"),
            SearchPeerStatus::Down => write!(f, "Down"),
            SearchPeerStatus::Unknown => write!(f, "Unknown"),
        }
    }
}

/// Distributed search peer information.
///
/// Represents a Splunk search peer configured on a search head.
/// Search peers are targets for distributed searches, distinct from
/// indexer cluster peers which handle data replication.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchPeer {
    /// The peer name (entry name from the API).
    #[serde(default)]
    pub name: String,
    /// The hostname or IP address of the peer.
    #[serde(rename = "host")]
    pub host: String,
    /// The management port of the peer (usually 8089).
    #[serde(rename = "port")]
    pub port: u32,
    /// The connection status of the peer.
    #[serde(rename = "status")]
    pub status: SearchPeerStatus,
    /// The Splunk version running on the peer.
    #[serde(rename = "version")]
    pub version: Option<String>,
    /// The unique identifier (GUID) of the peer.
    #[serde(rename = "guid")]
    pub guid: Option<String>,
    /// The last time the peer was connected.
    #[serde(rename = "last_connected")]
    pub last_connected: Option<String>,
    /// Whether the peer is disabled.
    #[serde(rename = "disabled")]
    pub disabled: Option<bool>,
}

/// Search peer list response.
///
/// Wrapper struct for deserializing the Splunk API response when listing search peers.
#[derive(Debug, Deserialize, Clone)]
pub struct SearchPeerListResponse {
    /// The list of search peer entries returned by the API.
    pub entry: Vec<SearchPeerEntry>,
}

/// A single search peer entry in the list response.
///
/// Splunk's REST API wraps each resource in an entry structure containing
/// metadata and the actual content.
#[derive(Debug, Deserialize, Clone)]
pub struct SearchPeerEntry {
    /// The entry name (peer identifier).
    pub name: String,
    /// The search peer content/data.
    pub content: SearchPeer,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod search_peer_status_tests {
        use super::*;

        #[test]
        fn test_deserialize_up() {
            let json = r#""Up""#;
            let status: SearchPeerStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, SearchPeerStatus::Up);
        }

        #[test]
        fn test_deserialize_down() {
            let json = r#""Down""#;
            let status: SearchPeerStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, SearchPeerStatus::Down);
        }

        #[test]
        fn test_deserialize_unknown_variants() {
            let unknown_values = vec![
                r#""Maintenance""#,
                r#""Disconnected""#,
                r#""Pending""#,
                r#""offline""#,
                r#""Unknown""#,
            ];

            for json in unknown_values {
                let status: SearchPeerStatus = serde_json::from_str(json).unwrap();
                assert_eq!(
                    status,
                    SearchPeerStatus::Unknown,
                    "Expected Unknown for {}",
                    json
                );
            }
        }

        #[test]
        fn test_display_up() {
            assert_eq!(SearchPeerStatus::Up.to_string(), "Up");
        }

        #[test]
        fn test_display_down() {
            assert_eq!(SearchPeerStatus::Down.to_string(), "Down");
        }

        #[test]
        fn test_display_unknown() {
            assert_eq!(SearchPeerStatus::Unknown.to_string(), "Unknown");
        }

        #[test]
        fn test_default() {
            assert_eq!(SearchPeerStatus::default(), SearchPeerStatus::Unknown);
        }

        #[test]
        fn test_serialize_up() {
            let status = SearchPeerStatus::Up;
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, r#""Up""#);
        }

        #[test]
        fn test_serialize_down() {
            let status = SearchPeerStatus::Down;
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, r#""Down""#);
        }

        #[test]
        fn test_serialize_unknown() {
            let status = SearchPeerStatus::Unknown;
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, r#""Unknown""#);
        }

        #[test]
        fn test_clone_and_copy() {
            let status = SearchPeerStatus::Up;
            let copied = status;
            let cloned = status;

            assert_eq!(status, copied);
            assert_eq!(status, cloned);
        }

        #[test]
        fn test_equality() {
            assert_eq!(SearchPeerStatus::Up, SearchPeerStatus::Up);
            assert_eq!(SearchPeerStatus::Down, SearchPeerStatus::Down);
            assert_eq!(SearchPeerStatus::Unknown, SearchPeerStatus::Unknown);

            assert_ne!(SearchPeerStatus::Up, SearchPeerStatus::Down);
            assert_ne!(SearchPeerStatus::Up, SearchPeerStatus::Unknown);
            assert_ne!(SearchPeerStatus::Down, SearchPeerStatus::Unknown);
        }
    }

    mod search_peer_integration_tests {
        use super::*;

        #[test]
        fn test_deserialize_search_peer_with_up_status() {
            let json = r#"{
                "name": "peer1",
                "host": "10.0.0.1",
                "port": 8089,
                "status": "Up",
                "version": "9.1.0",
                "guid": "abc-123",
                "last_connected": "2024-01-01T00:00:00Z",
                "disabled": false
            }"#;

            let peer: SearchPeer = serde_json::from_str(json).unwrap();
            assert_eq!(peer.name, "peer1");
            assert_eq!(peer.host, "10.0.0.1");
            assert_eq!(peer.port, 8089);
            assert_eq!(peer.status, SearchPeerStatus::Up);
            assert_eq!(peer.version, Some("9.1.0".to_string()));
            assert_eq!(peer.guid, Some("abc-123".to_string()));
            assert_eq!(
                peer.last_connected,
                Some("2024-01-01T00:00:00Z".to_string())
            );
            assert_eq!(peer.disabled, Some(false));
        }

        #[test]
        fn test_deserialize_search_peer_with_down_status() {
            let json = r#"{
                "host": "10.0.0.2",
                "port": 8089,
                "status": "Down"
            }"#;

            let peer: SearchPeer = serde_json::from_str(json).unwrap();
            assert_eq!(peer.host, "10.0.0.2");
            assert_eq!(peer.status, SearchPeerStatus::Down);
        }

        #[test]
        fn test_deserialize_search_peer_with_unknown_status() {
            let json = r#"{
                "host": "10.0.0.3",
                "port": 8089,
                "status": "Maintenance"
            }"#;

            let peer: SearchPeer = serde_json::from_str(json).unwrap();
            assert_eq!(peer.status, SearchPeerStatus::Unknown);
        }

        #[test]
        fn test_serialize_search_peer() {
            let peer = SearchPeer {
                name: "peer1".to_string(),
                host: "10.0.0.1".to_string(),
                port: 8089,
                status: SearchPeerStatus::Up,
                version: Some("9.1.0".to_string()),
                guid: Some("abc-123".to_string()),
                last_connected: Some("2024-01-01T00:00:00Z".to_string()),
                disabled: Some(false),
            };

            let json = serde_json::to_string(&peer).unwrap();
            assert!(json.contains(r#""status":"Up""#));
            assert!(json.contains(r#""host":"10.0.0.1""#));
        }

        #[test]
        fn test_search_peer_list_response() {
            let json = r#"{
                "entry": [
                    {
                        "name": "peer1",
                        "content": {
                            "host": "10.0.0.1",
                            "port": 8089,
                            "status": "Up"
                        }
                    },
                    {
                        "name": "peer2",
                        "content": {
                            "host": "10.0.0.2",
                            "port": 8089,
                            "status": "Down"
                        }
                    }
                ]
            }"#;

            let response: SearchPeerListResponse = serde_json::from_str(json).unwrap();
            assert_eq!(response.entry.len(), 2);
            assert_eq!(response.entry[0].name, "peer1");
            assert_eq!(response.entry[0].content.status, SearchPeerStatus::Up);
            assert_eq!(response.entry[1].name, "peer2");
            assert_eq!(response.entry[1].content.status, SearchPeerStatus::Down);
        }
    }
}
