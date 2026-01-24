//! Server information endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::models::{App, AppListResponse, ServerInfo, SplunkHealth};

/// Get server information.
pub async fn get_server_info(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
) -> Result<ServerInfo> {
    let url = format!("{}/services/server/info", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(builder, max_retries).await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract content from the first entry
    let content = resp
        .get("entry")
        .and_then(|e| e.get(0))
        .and_then(|e| e.get("content"))
        .ok_or_else(|| {
            ClientError::InvalidResponse(
                "Missing entry content in server info response".to_string(),
            )
        })?;

    // Deserialize content into ServerInfo struct
    serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse server info: {}", e)))
}

/// Get system-wide health information.
pub async fn get_health(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
) -> Result<SplunkHealth> {
    let url = format!("{}/services/server/health/splunkd", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(builder, max_retries).await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract content from the first entry
    let content = resp
        .get("entry")
        .and_then(|e| e.get(0))
        .and_then(|e| e.get("content"))
        .ok_or_else(|| {
            ClientError::InvalidResponse("Missing entry content in health response".to_string())
        })?;

    // Deserialize content into SplunkHealth struct
    serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse health info: {}", e)))
}

/// List all installed apps.
pub async fn list_apps(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<u64>,
    offset: Option<u64>,
    max_retries: usize,
) -> Result<Vec<App>> {
    let url = format!("{}/services/apps/local", base_url);

    let mut query_params: Vec<(String, String)> = vec![
        ("output_mode".to_string(), "json".to_string()),
        ("count".to_string(), count.unwrap_or(30).to_string()),
    ];

    if let Some(o) = offset {
        query_params.push(("offset".to_string(), o.to_string()));
    }

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params);
    let response = send_request_with_retry(builder, max_retries).await?;

    let resp: AppListResponse = response.json().await?;

    Ok(resp.entry.into_iter().map(|e| e.content).collect())
}
