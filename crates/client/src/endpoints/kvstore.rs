//! KVStore status endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{KvStoreMember, KvStoreReplicationStatus, KvStoreStatus};

/// Get KVStore status.
pub async fn get_kvstore_status(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<KvStoreStatus> {
    let url = format!("{}/services/kvstore/status", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/kvstore/status",
        "GET",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract content from the first entry
    let content = resp
        .get("entry")
        .and_then(|e| e.get(0))
        .and_then(|e| e.get("content"))
        .ok_or_else(|| {
            ClientError::InvalidResponse(
                "Missing entry content in KVStore status response".to_string(),
            )
        })?;

    // Splunk can return different KVStore schemas depending on mode/version:
    // - Clustered: { currentMember: {...}, replicationStatus: {...} }
    // - Standalone: { current: {...}, members: {...} }
    if content.get("currentMember").is_some() {
        return serde_json::from_value(content.clone()).map_err(|e| {
            ClientError::InvalidResponse(format!("Failed to parse KVStore status: {}", e))
        });
    }

    let current = content.get("current").ok_or_else(|| {
        ClientError::InvalidResponse(
            "Missing current/currentMember in KVStore status response".to_string(),
        )
    })?;

    let guid = current
        .get("guid")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let replica_set = current
        .get("replicaSet")
        .and_then(|v| v.as_str())
        .unwrap_or_default()
        .to_string();

    let port = current
        .get("port")
        .and_then(|v| v.as_u64())
        .unwrap_or(0)
        .try_into()
        .unwrap_or(0);

    let status = current
        .get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("unknown");

    let replication_status = current
        .get("replicationStatus")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let status = if replication_status.is_empty() {
        status.to_string()
    } else {
        format!("{status} ({replication_status})")
    };

    let host = content
        .get("members")
        .and_then(|m| m.as_object())
        .and_then(|obj| obj.values().next())
        .and_then(|v| v.get("hostAndPort"))
        .and_then(|v| v.as_str())
        .and_then(|hp| hp.split(':').next())
        .unwrap_or("localhost")
        .to_string();

    Ok(KvStoreStatus {
        current_member: KvStoreMember {
            guid,
            host,
            port,
            replica_set,
            status,
        },
        // Standalone response doesn't expose oplog sizing; default to 0.
        replication_status: KvStoreReplicationStatus {
            oplog_size: 0,
            oplog_used: 0.0,
        },
    })
}
