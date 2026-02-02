//! Data model management endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{DataModel, DataModelListResponse};
use crate::name_merge::attach_entry_name;

/// List all data models.
pub async fn list_datamodels(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<u64>,
    offset: Option<u64>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<DataModel>> {
    let url = format!("{}/services/datamodel", base_url);

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
        send_request_with_retry(builder, max_retries, "/services/datamodel", "GET", metrics)
            .await?;

    let resp: DataModelListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Get a single data model by name (includes JSON data).
pub async fn get_datamodel(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    datamodel_name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<DataModel> {
    let url = format!("{}/services/datamodel/{}", base_url, datamodel_name);

    let query_params: Vec<(String, String)> = vec![("output_mode".to_string(), "json".to_string())];

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params);

    let response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/datamodel/{}", datamodel_name),
        "GET",
        metrics,
    )
    .await?;

    let resp: serde_json::Value = response.json().await?;

    // Extract entry from response
    let entry = resp.get("entry").and_then(|e| e.get(0)).ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry in data model response".to_string())
    })?;

    let entry_name = entry
        .get("name")
        .and_then(|n| n.as_str())
        .unwrap_or(datamodel_name)
        .to_string();

    let content = entry.get("content").ok_or_else(|| {
        ClientError::InvalidResponse("Missing entry content in data model response".to_string())
    })?;

    let datamodel: DataModel = serde_json::from_value(content.clone())
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to parse data model: {}", e)))?;

    Ok(attach_entry_name(entry_name, datamodel))
}
