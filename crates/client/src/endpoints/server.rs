//! Server information endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::models::{ServerInfo, SplunkHealth};

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

    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::ApiError {
            status,
            message: body,
        });
    }

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

    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(ClientError::ApiError {
            status,
            message: body,
        });
    }

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
