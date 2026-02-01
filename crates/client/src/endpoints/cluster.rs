//! Cluster management endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{
    ClusterInfo, ClusterManagementResponse, ClusterPeer, DecommissionPeerParams,
    MaintenanceModeParams, RemovePeersParams,
};

/// Get cluster configuration/status.
pub async fn get_cluster_info(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ClusterInfo> {
    let url = format!("{}/services/cluster/master/config", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/cluster/master/config",
        "GET",
        metrics,
    )
    .await?;

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
        maintenance_mode: content["maintenance_mode"].as_bool(),
    })
}

/// Get cluster peer information.
pub async fn get_cluster_peers(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<ClusterPeer>> {
    let url = format!("{}/services/cluster/master/peers", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/cluster/master/peers",
        "GET",
        metrics,
    )
    .await?;

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

/// Set maintenance mode on the cluster manager.
pub async fn set_maintenance_mode(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &MaintenanceModeParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ClusterManagementResponse> {
    let url = format!(
        "{}/services/cluster/master/control/default/maintenance",
        base_url
    );

    let form_params: Vec<(String, String)> = vec![
        ("mode".to_string(), params.mode.to_string()),
        ("output_mode".to_string(), "json".to_string()),
    ];

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/cluster/master/control/default/maintenance",
        "POST",
        metrics,
    )
    .await?;

    // Parse response - may be empty on success
    let text = response.text().await?;
    if text.trim().is_empty() {
        return Ok(ClusterManagementResponse {
            success: true,
            message: Some(format!(
                "Maintenance mode {}",
                if params.mode { "enabled" } else { "disabled" }
            )),
        });
    }

    let resp: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse maintenance mode response: {}", e))
    })?;

    Ok(ClusterManagementResponse {
        success: true,
        message: resp["entry"][0]["content"]["message"]
            .as_str()
            .map(|s| s.to_string()),
    })
}

/// Rebalance primary buckets across all peers.
pub async fn rebalance_cluster(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ClusterManagementResponse> {
    let url = format!(
        "{}/services/cluster/master/control/control/rebalance_primaries",
        base_url
    );

    let form_params: Vec<(String, String)> = vec![("output_mode".to_string(), "json".to_string())];

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/cluster/master/control/control/rebalance_primaries",
        "POST",
        metrics,
    )
    .await?;

    // Parse response - may be empty on success
    let text = response.text().await?;
    if text.trim().is_empty() {
        return Ok(ClusterManagementResponse {
            success: true,
            message: Some("Cluster rebalance initiated".to_string()),
        });
    }

    let resp: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse rebalance response: {}", e))
    })?;

    Ok(ClusterManagementResponse {
        success: true,
        message: resp["entry"][0]["content"]["message"]
            .as_str()
            .map(|s| s.to_string()),
    })
}

/// Remove one or more peers from the cluster.
pub async fn remove_peers(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &RemovePeersParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ClusterManagementResponse> {
    let url = format!(
        "{}/services/cluster/master/control/control/remove_peers",
        base_url
    );

    let form_params: Vec<(String, String)> = vec![
        ("peers".to_string(), params.peers.clone()),
        ("output_mode".to_string(), "json".to_string()),
    ];

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/cluster/master/control/control/remove_peers",
        "POST",
        metrics,
    )
    .await?;

    // Parse response - may be empty on success
    let text = response.text().await?;
    if text.trim().is_empty() {
        return Ok(ClusterManagementResponse {
            success: true,
            message: Some(format!("Peer(s) {} removal initiated", params.peers)),
        });
    }

    let resp: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse remove peers response: {}", e))
    })?;

    Ok(ClusterManagementResponse {
        success: true,
        message: resp["entry"][0]["content"]["message"]
            .as_str()
            .map(|s| s.to_string()),
    })
}

/// Decommission a specific peer.
pub async fn decommission_peer(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    peer_name: &str,
    params: &DecommissionPeerParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ClusterPeer> {
    let url = format!("{}/services/cluster/master/peers/{}", base_url, peer_name);

    let form_params: Vec<(String, String)> = vec![
        ("decommission".to_string(), params.decommission.to_string()),
        ("output_mode".to_string(), "json".to_string()),
    ];

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/cluster/master/peers/{}", peer_name),
        "POST",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry from response
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry in decommission peer response".to_string())
    })?;

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse(
            "Missing entry content in decommission peer response".to_string(),
        )
    })?;

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
}
