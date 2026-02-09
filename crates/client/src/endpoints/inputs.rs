//! Input management endpoints.
//!
//! This module provides low-level HTTP endpoint functions for interacting
//! with the Splunk data inputs REST API.
//!
//! # What this module handles:
//! - HTTP GET requests to list data inputs
//! - HTTP POST requests to enable/disable inputs
//! - Query parameter construction for pagination
//!
//! # What this module does NOT handle:
//! - Authentication retry logic (handled by [`crate::client`])
//! - High-level client operations (see [`crate::client::inputs`])
//! - Response deserialization (delegated to models)

use reqwest::Client;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::send_request_with_retry;
use crate::error::Result;
use crate::metrics::MetricsCollector;
use crate::models::{Input, InputListResponse};
use crate::name_merge::attach_entry_name;

/// List inputs of a specific type.
///
/// Retrieves a list of data inputs from a specific endpoint.
/// Supports pagination via `count` and `offset` parameters.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request
/// * `base_url` - The base URL of the Splunk server
/// * `auth_token` - The authentication token for the request
/// * `input_type` - The type of input (tcp/raw, tcp/cooked, udp, monitor, script)
/// * `count` - Maximum number of results to return (default: 30)
/// * `offset` - Offset for pagination
/// * `max_retries` - Maximum number of retry attempts for failed requests
/// * `metrics` - Optional metrics collector for request tracking
///
/// # Returns
///
/// A `Result` containing a vector of `Input` structs on success.
///
/// # Errors
///
/// Returns a `ClientError` if the request fails or the response cannot be parsed.
#[allow(clippy::too_many_arguments)]
pub async fn list_inputs_by_type(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    input_type: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<Input>> {
    let url = format!("{}/services/data/inputs/{}", base_url, input_type);

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
        &format!("/services/data/inputs/{}", input_type),
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: InputListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Enable an input.
///
/// Sends a POST request to enable a data input.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request
/// * `base_url` - The base URL of the Splunk server
/// * `auth_token` - The authentication token for the request
/// * `input_type` - The type of input (tcp/raw, tcp/cooked, udp, monitor, script)
/// * `name` - The name of the input to enable
/// * `max_retries` - Maximum number of retry attempts for failed requests
/// * `metrics` - Optional metrics collector for request tracking
///
/// # Errors
///
/// Returns a `ClientError` if the request fails.
#[allow(clippy::too_many_arguments)]
pub async fn enable_input(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    input_type: &str,
    name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    let url = format!(
        "{}/services/data/inputs/{}/{}/enable",
        base_url, input_type, name
    );

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token));
    let _response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/data/inputs/{}/{{name}}/enable", input_type),
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(())
}

/// Disable an input.
///
/// Sends a POST request to disable a data input.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request
/// * `base_url` - The base URL of the Splunk server
/// * `auth_token` - The authentication token for the request
/// * `input_type` - The type of input (tcp/raw, tcp/cooked, udp, monitor, script)
/// * `name` - The name of the input to disable
/// * `max_retries` - Maximum number of retry attempts for failed requests
/// * `metrics` - Optional metrics collector for request tracking
///
/// # Errors
///
/// Returns a `ClientError` if the request fails.
#[allow(clippy::too_many_arguments)]
pub async fn disable_input(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    input_type: &str,
    name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<()> {
    let url = format!(
        "{}/services/data/inputs/{}/{}/disable",
        base_url, input_type, name
    );

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token));
    let _response = send_request_with_retry(
        builder,
        max_retries,
        &format!("/services/data/inputs/{}/{{name}}/disable", input_type),
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    Ok(())
}
