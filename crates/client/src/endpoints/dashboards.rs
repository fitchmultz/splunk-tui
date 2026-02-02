//! Dashboard management endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{Dashboard, DashboardListResponse};
use crate::name_merge::attach_entry_name;

/// List all dashboards.
pub async fn list_dashboards(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<u64>,
    offset: Option<u64>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<Dashboard>> {
    let url = format!("{}/services/data/ui/views", base_url);

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
        "/services/data/ui/views",
        "GET",
        metrics,
    )
    .await?;

    let resp: DashboardListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Get a single dashboard by name (includes XML data).
pub async fn get_dashboard(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    dashboard_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Dashboard> {
    let url = format!("{}/services/data/ui/views/{}", base_url, dashboard_name);

    let query_params: Vec<(String, String)> = vec![("output_mode".to_string(), "json".to_string())];

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/data/ui/views/{}", dashboard_name),
        "GET",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry from response
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry in dashboard response".to_string())
    })?;

    let entry_name = entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(dashboard_name)
        .to_string();

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry content in dashboard response".to_string())
    })?;

    let dashboard: Dashboard = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse dashboard: {}", e)))?;

    Ok(attach_entry_name(entry_name, dashboard))
}
