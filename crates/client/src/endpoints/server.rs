//! Server information endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{App, AppListResponse, ServerInfo, SplunkHealth};
use crate::name_merge::attach_entry_name;

/// Get server information.
pub async fn get_server_info(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<ServerInfo> {
    let url = format!("{}/services/server/info", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/server/info",
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
    metrics: Option<&MetricsCollector>,
) -> Result<SplunkHealth> {
    let url = format!("{}/services/server/health/splunkd", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/server/health/splunkd",
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
    metrics: Option<&MetricsCollector>,
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
    let response =
        send_request_with_retry(builder, max_retries, "/services/apps/local", "GET", metrics)
            .await?;

    let resp: AppListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Get specific app details by name.
pub async fn get_app(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    app_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<App> {
    let url = format!("{}/services/apps/local/{}", base_url, app_name);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode".to_string(), "json".to_string())]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/apps/local/{app_name}",
        "GET",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry name and content from first entry
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse(format!(
            "Missing entry in get_app response for '{}'",
            app_name
        ))
    })?;

    let entry_name = entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(app_name)
        .to_string();

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse(format!(
            "Missing entry content in get_app response for '{}'",
            app_name
        ))
    })?;

    // Deserialize content into App struct and attach the entry name
    let app: App = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse app info: {}", e)))?;

    Ok(crate::name_merge::attach_entry_name(entry_name, app))
}

/// Enable an app by name.
pub async fn enable_app(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    app_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    let url = format!("{}/services/apps/local/{}/enable", base_url, app_name);

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token));
    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/apps/local/{app_name}/enable",
        "POST",
        metrics,
    )
    .await?;

    Ok(())
}

/// Disable an app by name.
pub async fn disable_app(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    app_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<()> {
    let url = format!("{}/services/apps/local/{}/disable", base_url, app_name);

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token));
    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/apps/local/{app_name}/disable",
        "POST",
        metrics,
    )
    .await?;

    Ok(())
}
