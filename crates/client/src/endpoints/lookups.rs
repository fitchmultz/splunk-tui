//! Lookup table management endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{LookupTable, LookupTableListResponse};

/// List all lookup table files.
///
/// This endpoint returns CSV-based lookup files stored in Splunk.
/// KV store lookups are managed via a different endpoint.
pub async fn list_lookup_tables(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<u32>,
    offset: Option<u32>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<LookupTable>> {
    let url = format!("{}/services/data/lookup-table-files", base_url);

    let mut query_params: Vec<(String, String)> =
        vec![("output_mode".to_string(), "json".to_string())];

    if let Some(c) = count {
        query_params.push(("count".to_string(), c.to_string()));
    }
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
        "/services/data/lookup-table-files",
        "GET",
        metrics,
    )
    .await?;

    let resp: LookupTableListResponse = response.json().await.map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse lookup tables response: {}", e))
    })?;

    Ok(resp.entry.into_iter().map(|e| e.content).collect())
}
