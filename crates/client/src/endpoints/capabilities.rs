//! Capability management endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::Result;
use crate::metrics::MetricsCollector;
use crate::models::{Capability, CapabilityListResponse};
use crate::name_merge::attach_entry_name;

/// List all capabilities.
///
/// Capabilities are read-only in Splunk. They represent the set of
/// permissions that can be assigned to roles.
pub async fn list_capabilities(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<Capability>> {
    let url = format!("{}/services/authorization/capabilities", base_url);

    let query_params: Vec<(String, String)> = vec![("output_mode".to_string(), "json".to_string())];

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&query_params);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/authorization/capabilities",
        "GET",
        metrics,
    )
    .await?;

    let resp: CapabilityListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}
