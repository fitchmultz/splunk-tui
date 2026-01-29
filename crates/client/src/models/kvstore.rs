//! KVStore models for Splunk KVStore API.
//!
//! This module contains types for KVStore member and replication status.

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
