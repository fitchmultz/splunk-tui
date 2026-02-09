//! Authentication endpoints.

use reqwest::Client;
use tracing::debug;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::send_request_with_retry;
use crate::error::Result;
use crate::metrics::MetricsCollector;

/// Login to Splunk with username and password.
#[allow(clippy::too_many_arguments)]
pub async fn login(
    client: &Client,
    base_url: &str,
    username: &str,
    password: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<String> {
    debug!("Logging in to Splunk as {}", username);

    let url = format!("{}/services/auth/login", base_url);
    let builder = client
        .post(&url)
        .form(&[("username", username), ("password", password)])
        .query(&[("output_mode", "json")]);
    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/auth/login",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    let splunk_resp: serde_json::Value = response.json().await?;

    splunk_resp["sessionKey"]
        .as_str()
        .ok_or_else(|| {
            crate::error::ClientError::InvalidResponse("Missing sessionKey in response".to_string())
        })
        .map(|s| s.to_string())
}
