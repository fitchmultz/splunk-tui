//! Search Head Cluster (SHC) management endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{
    AddShcMemberParams, RemoveShcMemberParams, RollingRestartParams, SetCaptainParams, ShcCaptain,
    ShcConfig, ShcManagementResponse, ShcMember, ShcStatus,
};

/// Get SHC members.
pub async fn get_shc_members(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<ShcMember>> {
    let url = format!("{}/services/shcluster/member/members", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/shcluster/member/members",
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
            Ok(ShcMember {
                id: content["id"].as_str().unwrap_or("unknown").to_string(),
                label: content["label"].as_str().map(|s| s.to_string()),
                host: content["host"].as_str().unwrap_or("unknown").to_string(),
                port: content["port"].as_u64().map(|v| v as u32).unwrap_or(8089),
                status: content["status"].as_str().unwrap_or("unknown").to_string(),
                is_captain: content["is_captain"].as_bool().unwrap_or(false),
                is_dynamic_captain: content["is_dynamic_captain"].as_bool(),
                guid: content["guid"].as_str().unwrap_or("unknown").to_string(),
                site: content["site"].as_str().map(|s| s.to_string()),
                replication_port: content["replication_port"].as_u64().map(|v| v as u32),
                last_heartbeat: content["last_heartbeat"].as_str().map(|s| s.to_string()),
                pending_job_count: content["pending_job_count"].as_u64().map(|v| v as u32),
            })
        })
        .collect()
}

/// Get SHC captain information.
pub async fn get_shc_captain(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ShcCaptain> {
    let url = format!("{}/services/shcluster/captain/info", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/shcluster/captain/info",
        "GET",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    let content = &resp["entry"][0]["content"];

    Ok(ShcCaptain {
        id: content["id"].as_str().unwrap_or("unknown").to_string(),
        label: content["label"].as_str().map(|s| s.to_string()),
        host: content["host"].as_str().unwrap_or("unknown").to_string(),
        port: content["port"].as_u64().map(|v| v as u32).unwrap_or(8089),
        guid: content["guid"].as_str().unwrap_or("unknown").to_string(),
        site: content["site"].as_str().map(|s| s.to_string()),
        is_dynamic_captain: content["is_dynamic_captain"].as_bool().unwrap_or(false),
        election_epoch: content["election_epoch"].as_u64(),
    })
}

/// Get SHC status.
pub async fn get_shc_status(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ShcStatus> {
    let url = format!("{}/services/shcluster/member/info", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/shcluster/member/info",
        "GET",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    let content = &resp["entry"][0]["content"];

    Ok(ShcStatus {
        is_captain: content["is_captain"].as_bool().unwrap_or(false),
        is_searchable: content["is_searchable"].as_bool().unwrap_or(true),
        captain_uri: content["captain_uri"].as_str().map(|s| s.to_string()),
        member_count: content["member_count"]
            .as_u64()
            .map(|v| v as u32)
            .unwrap_or(0),
        minimum_member_count: content["minimum_member_count"].as_u64().map(|v| v as u32),
        election_timeout: content["election_timeout"].as_u64().map(|v| v as u32),
        rolling_restart_flag: content["rolling_restart_flag"].as_bool(),
        service_ready_flag: content["service_ready_flag"].as_bool(),
    })
}

/// Get SHC configuration.
pub async fn get_shc_config(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ShcConfig> {
    let url = format!("{}/services/shcluster/config/config", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/shcluster/config/config",
        "GET",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    let content = &resp["entry"][0]["content"];

    Ok(ShcConfig {
        id: content["id"].as_str().unwrap_or("unknown").to_string(),
        label: content["label"].as_str().map(|s| s.to_string()),
        replication_factor: content["replication_factor"].as_u64().map(|v| v as u32),
        deployer_push_mode: content["deployer_push_mode"]
            .as_str()
            .map(|s| s.to_string()),
        captain_uri: content["captain_uri"].as_str().map(|s| s.to_string()),
        shcluster_label: content["shcluster_label"].as_str().map(|s| s.to_string()),
    })
}

/// Add a member to the SHC.
pub async fn add_shc_member(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &AddShcMemberParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ShcManagementResponse> {
    let url = format!("{}/services/shcluster/captain/members", base_url);

    let form_params: Vec<(String, String)> = vec![
        ("target_uri".to_string(), params.target_uri.clone()),
        ("output_mode".to_string(), "json".to_string()),
    ];

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/shcluster/captain/members",
        "POST",
        metrics,
    )
    .await?;

    let text = response.text().await?;
    if text.trim().is_empty() {
        return Ok(ShcManagementResponse {
            success: true,
            message: Some("Member added successfully".to_string()),
        });
    }

    let resp: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse add member response: {}", e))
    })?;

    Ok(ShcManagementResponse {
        success: true,
        message: resp["entry"][0]["content"]["message"]
            .as_str()
            .map(|s| s.to_string()),
    })
}

/// Remove a member from the SHC.
pub async fn remove_shc_member(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &RemoveShcMemberParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ShcManagementResponse> {
    let url = format!(
        "{}/services/shcluster/captain/members/{}",
        base_url, params.member
    );

    let builder = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/shcluster/captain/members/{}", params.member),
        "DELETE",
        metrics,
    )
    .await?;

    let text = response.text().await?;
    if text.trim().is_empty() {
        return Ok(ShcManagementResponse {
            success: true,
            message: Some(format!("Member {} removed successfully", params.member)),
        });
    }

    let resp: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse remove member response: {}", e))
    })?;

    Ok(ShcManagementResponse {
        success: true,
        message: resp["entry"][0]["content"]["message"]
            .as_str()
            .map(|s| s.to_string()),
    })
}

/// Trigger a rolling restart of the SHC.
pub async fn rolling_restart_shc(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &RollingRestartParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ShcManagementResponse> {
    let url = format!(
        "{}/services/shcluster/captain/control/default/rolling_restart",
        base_url
    );

    let form_params: Vec<(String, String)> = vec![
        ("force".to_string(), params.force.to_string()),
        ("output_mode".to_string(), "json".to_string()),
    ];

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/shcluster/captain/control/default/rolling_restart",
        "POST",
        metrics,
    )
    .await?;

    let text = response.text().await?;
    if text.trim().is_empty() {
        return Ok(ShcManagementResponse {
            success: true,
            message: Some("Rolling restart initiated".to_string()),
        });
    }

    let resp: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse rolling restart response: {}", e))
    })?;

    Ok(ShcManagementResponse {
        success: true,
        message: resp["entry"][0]["content"]["message"]
            .as_str()
            .map(|s| s.to_string()),
    })
}

/// Set a specific member as captain.
pub async fn set_shc_captain(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    params: &SetCaptainParams,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ShcManagementResponse> {
    let url = format!(
        "{}/services/shcluster/member/control/default/set_captain",
        base_url
    );

    let form_params: Vec<(String, String)> = vec![
        ("target_guid".to_string(), params.target_guid.clone()),
        ("output_mode".to_string(), "json".to_string()),
    ];

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/shcluster/member/control/default/set_captain",
        "POST",
        metrics,
    )
    .await?;

    let text = response.text().await?;
    if text.trim().is_empty() {
        return Ok(ShcManagementResponse {
            success: true,
            message: Some(format!("Captain set to {}", params.target_guid)),
        });
    }

    let resp: serde_json::Value = serde_json::from_str(&text).map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse set captain response: {}", e))
    })?;

    Ok(ShcManagementResponse {
        success: true,
        message: resp["entry"][0]["content"]["message"]
            .as_str()
            .map(|s| s.to_string()),
    })
}
