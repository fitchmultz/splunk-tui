//! Workload management endpoints.
//!
//! This module provides low-level HTTP endpoint functions for interacting
//! with the Splunk Workload Management API.
//!
//! # What this module handles:
//! - HTTP GET requests to list workload pools and rules
//! - Query parameter construction for pagination
//!
//! # What this module does NOT handle:
//! - Authentication retry logic (handled by [`crate::client`])
//! - High-level client operations (see [`crate::client::workload`])
//! - Response deserialization (delegated to models)

use reqwest::Client;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::send_request_with_retry;
use crate::error::Result;
use crate::metrics::MetricsCollector;
use crate::models::{
    WorkloadPool, WorkloadPoolListResponse, WorkloadRule, WorkloadRuleListResponse,
};
use crate::name_merge::attach_entry_name;

/// List all workload pools.
///
/// Retrieves a list of workload pools from the Splunk server.
/// Supports pagination via `count` and `offset` parameters.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request
/// * `base_url` - The base URL of the Splunk server
/// * `auth_token` - The authentication token for the request
/// * `count` - Maximum number of results to return (default: 30)
/// * `offset` - Offset for pagination
/// * `max_retries` - Maximum number of retry attempts for failed requests
/// * `metrics` - Optional metrics collector for request tracking
///
/// # Returns
///
/// A `Result` containing a vector of `WorkloadPool` structs on success.
///
/// # Errors
///
/// Returns a `ClientError` if the request fails or the response cannot be parsed.
#[allow(clippy::too_many_arguments)]
pub async fn list_workload_pools(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<WorkloadPool>> {
    let url = format!("{}/services/workloads/pools", base_url);

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
        "/services/workloads/pools",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: WorkloadPoolListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// List all workload rules.
///
/// Retrieves a list of workload rules from the Splunk server.
/// Supports pagination via `count` and `offset` parameters.
///
/// # Arguments
///
/// * `client` - The HTTP client to use for the request
/// * `base_url` - The base URL of the Splunk server
/// * `auth_token` - The authentication token for the request
/// * `count` - Maximum number of results to return (default: 30)
/// * `offset` - Offset for pagination
/// * `max_retries` - Maximum number of retry attempts for failed requests
/// * `metrics` - Optional metrics collector for request tracking
///
/// # Returns
///
/// A `Result` containing a vector of `WorkloadRule` structs on success.
///
/// # Errors
///
/// Returns a `ClientError` if the request fails or the response cannot be parsed.
#[allow(clippy::too_many_arguments)]
pub async fn list_workload_rules(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<WorkloadRule>> {
    let url = format!("{}/services/workloads/rules", base_url);

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
        "/services/workloads/rules",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: WorkloadRuleListResponse = response.json().await?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}
