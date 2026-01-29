//! License management endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::Result;
use crate::metrics::MetricsCollector;
use crate::models::{LicensePool, LicenseStack, LicenseUsage, SplunkResponse};

/// Get license usage information.
pub async fn get_license_usage(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<LicenseUsage>> {
    let url = format!("{}/services/licenser/usage", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/usage",
        "GET",
        metrics,
    )
    .await?;

    let resp: SplunkResponse<LicenseUsage> = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| {
            let mut content = e.content;
            content.name = e.name;
            content
        })
        .collect())
}

/// List all license pools.
pub async fn list_license_pools(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<LicensePool>> {
    let url = format!("{}/services/licenser/pools", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/pools",
        "GET",
        metrics,
    )
    .await?;

    let resp: SplunkResponse<LicensePool> = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| {
            let mut content = e.content;
            content.name = e.name;
            content
        })
        .collect())
}

/// List all license stacks.
pub async fn list_license_stacks(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<Vec<LicenseStack>> {
    let url = format!("{}/services/licenser/stacks", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/licenser/stacks",
        "GET",
        metrics,
    )
    .await?;

    let resp: SplunkResponse<LicenseStack> = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| {
            let mut content = e.content;
            content.name = e.name;
            content
        })
        .collect())
}
