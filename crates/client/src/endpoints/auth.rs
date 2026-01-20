//! Authentication endpoints.

use reqwest::Client;
use tracing::debug;

use crate::error::Result;

/// Login to Splunk with username and password.
pub async fn login(
    client: &Client,
    base_url: &str,
    username: &str,
    password: &str,
) -> Result<String> {
    debug!("Logging in to Splunk as {}", username);

    let url = format!("{}/services/auth/login", base_url);
    let response = client
        .post(&url)
        .form(&[("username", username), ("password", password)])
        .send()
        .await?;

    let status = response.status().as_u16();

    if !response.status().is_success() {
        let body = response.text().await.unwrap_or_default();
        return Err(crate::error::ClientError::ApiError {
            status,
            message: body,
        });
    }

    let splunk_resp: serde_json::Value = response.json().await?;

    splunk_resp["entry"][0]["content"]["sessionKey"]
        .as_str()
        .ok_or_else(|| {
            crate::error::ClientError::InvalidResponse("Missing sessionKey in response".to_string())
        })
        .map(|s| s.to_string())
}
