//! Authentication endpoints.

use reqwest::Client;
use tracing::debug;

use crate::endpoints::send_request_with_retry;
use crate::error::Result;

/// Login to Splunk with username and password.
pub async fn login(
    client: &Client,
    base_url: &str,
    username: &str,
    password: &str,
    max_retries: usize,
) -> Result<String> {
    debug!("Logging in to Splunk as {}", username);

    let url = format!("{}/services/auth/login", base_url);
    let builder = client
        .post(&url)
        .form(&[("username", username), ("password", password)])
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(builder, max_retries).await?;

    let splunk_resp: serde_json::Value = response.json().await?;

    splunk_resp["entry"][0]["content"]["sessionKey"]
        .as_str()
        .ok_or_else(|| {
            crate::error::ClientError::InvalidResponse("Missing sessionKey in response".to_string())
        })
        .map(|s| s.to_string())
}
