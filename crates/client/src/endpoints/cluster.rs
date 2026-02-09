//! Cluster management endpoints.

use reqwest::Client;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::send_request_with_retry;
use crate::endpoints::{extract_entry_content, extract_entry_message};
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{
    ClusterInfo, ClusterManagementResponse, ClusterPeer, DecommissionPeerParams,
    MaintenanceModeParams, RemovePeersParams,
};

/// Get cluster configuration/status.
#[allow(clippy::too_many_arguments)]
pub async fn get_cluster_info(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
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
        circuit_breaker,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    let content = extract_entry_content(&resp)?;

    let info: ClusterInfo = serde_json::from_value(content.clone()).map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse cluster info: {}", e))
    })?;
    Ok(info)
}

/// Get cluster peer information.
#[allow(clippy::too_many_arguments)]
pub async fn get_cluster_peers(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
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
        circuit_breaker,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    let empty = vec![];
    let entries = resp["entry"].as_array().unwrap_or(&empty);

    entries
        .iter()
        .map(|e| {
            let content = e.get("content").ok_or_else(|| {
                ClientError::InvalidResponse("Missing content in cluster peer entry".to_string())
            })?;
            let peer: ClusterPeer = serde_json::from_value(content.clone()).map_err(|e| {
                ClientError::InvalidResponse(format!("Failed to parse cluster peer: {}", e))
            })?;
            Ok(peer)
        })
        .collect()
}

/// Set maintenance mode on the cluster manager.
#[allow(clippy::too_many_arguments)]
pub async fn set_maintenance_mode(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &MaintenanceModeParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
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
        circuit_breaker,
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
        message: extract_entry_message(&resp),
    })
}

/// Rebalance primary buckets across all peers.
#[allow(clippy::too_many_arguments)]
pub async fn rebalance_cluster(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
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
        circuit_breaker,
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
        message: extract_entry_message(&resp),
    })
}

/// Remove one or more peers from the cluster.
#[allow(clippy::too_many_arguments)]
pub async fn remove_peers(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &RemovePeersParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
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
        circuit_breaker,
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
        message: extract_entry_message(&resp),
    })
}

/// Decommission a specific peer.
#[allow(clippy::too_many_arguments)]
pub async fn decommission_peer(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    peer_name: &str,
    params: &DecommissionPeerParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
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
        circuit_breaker,
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

    let peer: ClusterPeer = serde_json::from_value(content.clone()).map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse cluster peer: {}", e))
    })?;
    Ok(peer)
}
