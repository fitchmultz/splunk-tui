//! Cluster management endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::models::{ClusterInfo, ClusterPeer};

/// Get cluster configuration/status.
pub async fn get_cluster_info(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
) -> Result<ClusterInfo> {
    let url = format!("{}/services/cluster/master/config", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(builder, max_retries).await?;

    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::ApiError {
            status,
            message: body,
        });
    }

    let resp: serde_json::Value = response.json().await?;

    let content = &resp["entry"][0]["content"];

    Ok(ClusterInfo {
        id: content["id"].as_str().unwrap_or("unknown").to_string(),
        label: content["label"].as_str().map(|s| s.to_string()),
        mode: content["mode"].as_str().unwrap_or("unknown").to_string(),
        manager_uri: content["manager_uri"].as_str().map(|s| s.to_string()),
        replication_factor: content["replication_factor"].as_u64().map(|v| v as u32),
        search_factor: content["search_factor"].as_u64().map(|v| v as u32),
        status: content["status"].as_str().map(|s| s.to_string()),
    })
}

/// Get cluster peer information.
pub async fn get_cluster_peers(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
) -> Result<Vec<ClusterPeer>> {
    let url = format!("{}/services/cluster/master/peers", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(builder, max_retries).await?;

    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::ApiError {
            status,
            message: body,
        });
    }

    let resp: serde_json::Value = response.json().await?;

    let empty = vec![];
    let entries = resp["entry"].as_array().unwrap_or(&empty);

    entries
        .iter()
        .map(|e| {
            let content = &e["content"];
            Ok(ClusterPeer {
                id: content["id"].as_str().unwrap_or("unknown").to_string(),
                label: content["label"].as_str().map(|s| s.to_string()),
                status: content["status"].as_str().unwrap_or("unknown").to_string(),
                peer_state: content["peer_state"]
                    .as_str()
                    .unwrap_or("unknown")
                    .to_string(),
                site: content["site"].as_str().map(|s| s.to_string()),
                guid: content["guid"].as_str().unwrap_or("unknown").to_string(),
                host: content["host"].as_str().unwrap_or("unknown").to_string(),
                port: content["port"].as_u64().map(|v| v as u32).unwrap_or(8089),
                replication_count: content["replication_count"].as_u64().map(|v| v as u32),
                replication_status: content["replication_status"]
                    .as_str()
                    .map(|s| s.to_string()),
                bundle_replication_count: content["bundle_replication_count"]
                    .as_u64()
                    .map(|v| v as u32),
                is_captain: content["is_captain"].as_bool(),
            })
        })
        .collect()
}
