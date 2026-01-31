//! KVStore models for Splunk KVStore API.
//!
//! This module contains types for KVStore member, replication status,
//! collection management, and collection data access.

use serde::{Deserialize, Serialize};

/// KVStore member information.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct KvStoreMember {
    pub guid: String,
    pub host: String,
    pub port: u32,
    #[serde(rename = "replicaSet")]
    pub replica_set: String,
    pub status: String,
}

/// KVStore replication status.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct KvStoreReplicationStatus {
    #[serde(rename = "oplogSize")]
    #[serde(deserialize_with = "crate::serde_helpers::u64_from_string_or_number")]
    pub oplog_size: u64,
    #[serde(rename = "oplogUsed")]
    pub oplog_used: f64,
}

/// KVStore status information.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct KvStoreStatus {
    #[serde(rename = "currentMember")]
    pub current_member: KvStoreMember,
    #[serde(rename = "replicationStatus")]
    pub replication_status: KvStoreReplicationStatus,
}

/// Parameters for creating a new KVStore collection.
#[derive(Debug, Clone, Default)]
pub struct CreateCollectionParams {
    /// Collection name (required)
    pub name: String,
    /// App context (default: "search")
    pub app: Option<String>,
    /// Owner context (default: "nobody")
    pub owner: Option<String>,
    /// Field schema as JSON string
    pub fields: Option<String>,
    /// Accelerated fields as JSON string
    pub accelerated_fields: Option<String>,
}

/// Parameters for modifying a KVStore collection.
#[derive(Debug, Clone, Default)]
pub struct ModifyCollectionParams {
    /// Field schema as JSON string
    pub fields: Option<String>,
    /// Accelerated fields as JSON string
    pub accelerated_fields: Option<String>,
    /// Disable/enable the collection
    pub disabled: Option<bool>,
}

/// KVStore collection information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KvStoreCollection {
    #[serde(default)]
    pub name: String,
    #[serde(default)]
    pub app: String,
    #[serde(default)]
    pub owner: String,
    #[serde(default)]
    pub sharing: String,
    /// Collection fields schema (JSON object as serde_json::Value)
    pub fields: Option<serde_json::Value>,
    /// Accelerated fields for indexing
    #[serde(rename = "acceleratedFields")]
    pub accelerated_fields: Option<serde_json::Value>,
    /// Whether the collection is disabled
    pub disabled: Option<bool>,
}

/// KVStore collection list response entry.
#[derive(Debug, Deserialize, Clone)]
pub struct CollectionEntry {
    pub name: String,
    pub content: KvStoreCollection,
}

/// KVStore collection list response.
#[derive(Debug, Deserialize, Clone)]
pub struct CollectionListResponse {
    pub entry: Vec<CollectionEntry>,
}

/// KVStore record (document) in a collection.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct KvStoreRecord {
    /// Record ID (_key field in Splunk)
    #[serde(rename = "_key")]
    pub key: Option<String>,
    /// Record owner
    #[serde(rename = "_owner")]
    pub owner: Option<String>,
    /// Record user (additional metadata)
    #[serde(rename = "_user")]
    pub user: Option<String>,
    /// All other fields are dynamic
    #[serde(flatten)]
    pub data: serde_json::Value,
}
