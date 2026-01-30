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
    /// The connection status (e.g., "Up", "Down").
    #[serde(rename = "status")]
    pub status: String,
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
