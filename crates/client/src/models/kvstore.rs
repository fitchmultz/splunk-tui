//! KVStore models for Splunk KVStore API.
//!
//! This module contains types for KVStore member, replication status,
//! collection management, and collection data access.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Status of a KVStore member.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum KvStoreMemberStatus {
    /// KVStore member is ready.
    Ready,
    /// KVStore member is initializing.
    Initializing,
    /// KVStore member encountered an error.
    Error,
    /// Unknown status (fallback for unrecognized values).
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for KvStoreMemberStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Ready => "ready",
            Self::Initializing => "initializing",
            Self::Error => "error",
            Self::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

/// KVStore member information.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct KvStoreMember {
    pub guid: String,
    pub host: String,
    pub port: u32,
    #[serde(rename = "replicaSet")]
    pub replica_set: String,
    pub status: KvStoreMemberStatus,
}

/// KVStore replication status.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct KvStoreReplicationStatus {
    #[serde(rename = "oplogSize")]
    #[serde(deserialize_with = "crate::serde_helpers::usize_from_string_or_number")]
    pub oplog_size: usize,
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

#[cfg(test)]
mod tests {
    use super::*;

    mod kv_store_member_status_tests {
        use super::*;

        #[test]
        fn test_deserialize_ready() {
            let json = "\"ready\"";
            let status: KvStoreMemberStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, KvStoreMemberStatus::Ready);
        }

        #[test]
        fn test_deserialize_initializing() {
            let json = "\"initializing\"";
            let status: KvStoreMemberStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, KvStoreMemberStatus::Initializing);
        }

        #[test]
        fn test_deserialize_error() {
            let json = "\"error\"";
            let status: KvStoreMemberStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, KvStoreMemberStatus::Error);
        }

        #[test]
        fn test_deserialize_unknown_values_fallback() {
            // Unknown values should deserialize to Unknown variant
            let json = "\"some_random_status\"";
            let status: KvStoreMemberStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, KvStoreMemberStatus::Unknown);
        }

        #[test]
        fn test_deserialize_empty_string_fallback() {
            let json = "\"\"";
            let status: KvStoreMemberStatus = serde_json::from_str(json).unwrap();
            assert_eq!(status, KvStoreMemberStatus::Unknown);
        }

        #[test]
        fn test_serialize_ready() {
            let status = KvStoreMemberStatus::Ready;
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, "\"ready\"");
        }

        #[test]
        fn test_serialize_initializing() {
            let status = KvStoreMemberStatus::Initializing;
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, "\"initializing\"");
        }

        #[test]
        fn test_serialize_error() {
            let status = KvStoreMemberStatus::Error;
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, "\"error\"");
        }

        #[test]
        fn test_serialize_unknown() {
            let status = KvStoreMemberStatus::Unknown;
            let json = serde_json::to_string(&status).unwrap();
            assert_eq!(json, "\"unknown\"");
        }

        #[test]
        fn test_display_ready() {
            assert_eq!(format!("{}", KvStoreMemberStatus::Ready), "ready");
        }

        #[test]
        fn test_display_initializing() {
            assert_eq!(
                format!("{}", KvStoreMemberStatus::Initializing),
                "initializing"
            );
        }

        #[test]
        fn test_display_error() {
            assert_eq!(format!("{}", KvStoreMemberStatus::Error), "error");
        }

        #[test]
        fn test_display_unknown() {
            assert_eq!(format!("{}", KvStoreMemberStatus::Unknown), "unknown");
        }

        #[test]
        fn test_default_is_unknown() {
            let status: KvStoreMemberStatus = Default::default();
            assert_eq!(status, KvStoreMemberStatus::Unknown);
        }

        #[test]
        fn test_clone_and_copy() {
            let status = KvStoreMemberStatus::Ready;
            let cloned = status;
            assert_eq!(status, cloned);

            // Verify Copy trait by using after move
            let status2 = status;
            let _status3 = status; // This would fail if not Copy
            assert_eq!(status2, KvStoreMemberStatus::Ready);
        }

        #[test]
        fn test_partial_eq_and_eq() {
            assert_eq!(KvStoreMemberStatus::Ready, KvStoreMemberStatus::Ready);
            assert_ne!(KvStoreMemberStatus::Ready, KvStoreMemberStatus::Error);
        }
    }

    mod kv_store_member_integration_tests {
        use super::*;

        #[test]
        fn test_kv_store_member_deserialization_with_status() {
            let json = r#"{
                "guid": "abc-123",
                "host": "localhost",
                "port": 8089,
                "replicaSet": "rs0",
                "status": "ready"
            }"#;

            let member: KvStoreMember = serde_json::from_str(json).unwrap();
            assert_eq!(member.guid, "abc-123");
            assert_eq!(member.host, "localhost");
            assert_eq!(member.port, 8089);
            assert_eq!(member.replica_set, "rs0");
            assert_eq!(member.status, KvStoreMemberStatus::Ready);
        }

        #[test]
        fn test_kv_store_member_with_initializing_status() {
            let json = r#"{
                "guid": "def-456",
                "host": "remotehost",
                "port": 8089,
                "replicaSet": "rs1",
                "status": "initializing"
            }"#;

            let member: KvStoreMember = serde_json::from_str(json).unwrap();
            assert_eq!(member.status, KvStoreMemberStatus::Initializing);
        }

        #[test]
        fn test_kv_store_member_with_error_status() {
            let json = r#"{
                "guid": "ghi-789",
                "host": "brokenhost",
                "port": 8089,
                "replicaSet": "rs2",
                "status": "error"
            }"#;

            let member: KvStoreMember = serde_json::from_str(json).unwrap();
            assert_eq!(member.status, KvStoreMemberStatus::Error);
        }

        #[test]
        fn test_kv_store_member_with_unknown_status_fallback() {
            let json = r#"{
                "guid": "jkl-000",
                "host": "unknownhost",
                "port": 8089,
                "replicaSet": "rs3",
                "status": "some_future_status"
            }"#;

            let member: KvStoreMember = serde_json::from_str(json).unwrap();
            assert_eq!(member.status, KvStoreMemberStatus::Unknown);
        }

        #[test]
        fn test_kv_store_member_serialization() {
            let member = KvStoreMember {
                guid: "test-guid".to_string(),
                host: "testhost".to_string(),
                port: 8089,
                replica_set: "rs_test".to_string(),
                status: KvStoreMemberStatus::Ready,
            };

            let json = serde_json::to_string(&member).unwrap();
            assert!(json.contains("\"status\":\"ready\""));
            assert!(json.contains("\"guid\":\"test-guid\""));
            assert!(json.contains("\"host\":\"testhost\""));
            assert!(json.contains("\"port\":8089"));
            assert!(json.contains("\"replicaSet\":\"rs_test\""));
        }
    }
}
