//! License management endpoints.

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::Result;
use crate::models::{LicenseUsage, SplunkResponse};

/// Get license usage information.
pub async fn get_license_usage(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    max_retries: usize,
) -> Result<Vec<LicenseUsage>> {
    let url = format!("{}/services/license/usage", base_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(builder, max_retries).await?;

    let resp: SplunkResponse<LicenseUsage> = response.json().await?;

    Ok(resp.entry.into_iter().map(|e| e.content).collect())
}
