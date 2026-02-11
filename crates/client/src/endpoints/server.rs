//! Server information endpoints.

use reqwest::Client;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::encode_path_segment;
use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{App, AppListResponse, ServerInfo, SplunkHealth};
use crate::name_merge::attach_entry_name;

/// Get server information.
#[allow(clippy::too_many_arguments)]
pub async fn get_server_info(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
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
        circuit_breaker,
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
#[allow(clippy::too_many_arguments)]
pub async fn get_health(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
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
        circuit_breaker,
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
#[allow(clippy::too_many_arguments)]
pub async fn list_apps(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
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
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/apps/local",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: AppListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Get specific app details by name.
#[allow(clippy::too_many_arguments)]
pub async fn get_app(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    app_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<App> {
    let encoded_app_name = encode_path_segment(app_name);
    let url = format!("{}/services/apps/local/{}", base_url, encoded_app_name);

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
        circuit_breaker,
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
#[allow(clippy::too_many_arguments)]
pub async fn enable_app(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    app_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    let encoded_app_name = encode_path_segment(app_name);
    let url = format!(
        "{}/services/apps/local/{}/enable",
        base_url, encoded_app_name
    );

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token));
    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/apps/local/{app_name}/enable",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(())
}

/// Disable an app by name.
#[allow(clippy::too_many_arguments)]
pub async fn disable_app(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    app_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    let encoded_app_name = encode_path_segment(app_name);
    let url = format!(
        "{}/services/apps/local/{}/disable",
        base_url, encoded_app_name
    );

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token));
    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/apps/local/{app_name}/disable",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(())
}

/// Install a Splunk app from a .spl package file.
///
/// Uses multipart/form-data upload to POST /services/apps/appinstall.
/// Note: Streaming uploads cannot be retried - this function only retries
/// on authentication errors (401) where the body hasn't been consumed yet.
///
/// # Arguments
///
/// * `client` - The HTTP client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - The authentication token
/// * `file_path` - Path to the .spl package file
/// * `max_retries` - Maximum number of retries for authentication failures
/// * `metrics` - Optional metrics collector
#[allow(clippy::too_many_arguments)]
pub async fn install_app(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    file_path: &std::path::Path,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<App> {
    let url = format!("{}/services/apps/appinstall", base_url);

    // Read the file content
    let file_content = tokio::fs::read(file_path).await.map_err(|e| {
        ClientError::InvalidRequest(format!(
            "Failed to read app package file '{}': {}",
            file_path.display(),
            e
        ))
    })?;

    let file_name = file_path
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("app.spl");

    // Build multipart form
    let form = reqwest::multipart::Form::new().part(
        "splunk_file",
        reqwest::multipart::Part::bytes(file_content)
            .file_name(file_name.to_string())
            .mime_str("application/octet-stream")
            .map_err(|e| ClientError::InvalidRequest(format!("Invalid mime type: {}", e)))?,
    );

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")])
        .multipart(form);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/apps/appinstall",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry name and content from first entry (same pattern as get_app)
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry in install_app response".to_string())
    })?;

    let entry_name = entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or("unknown")
        .to_string();

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry content in install_app response".to_string())
    })?;

    // Deserialize content into App struct and attach the entry name
    let app: App = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse app info: {}", e)))?;

    Ok(crate::name_merge::attach_entry_name(entry_name, app))
}

/// Remove (uninstall) a Splunk app by name.
///
/// DELETE /services/apps/local/{app_name}
#[allow(clippy::too_many_arguments)]
pub async fn remove_app(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    app_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    let encoded_app_name = encode_path_segment(app_name);
    let url = format!("{}/services/apps/local/{}", base_url, encoded_app_name);

    let builder = client
        .delete(&url)
        .header("Authorization", format!("Bearer {}", auth_token));

    let _response = send_request_with_retry(
        builder,
        max_retries,
        "/services/apps/local/{app_name}",
        "DELETE",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(())
}
