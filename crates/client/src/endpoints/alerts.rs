//! Alert endpoints for Splunk alerts API.
//!
//! This module provides low-level HTTP endpoints for Splunk alert operations.
//!
//! # What this module handles:
//! - Fired alerts listing and retrieval
//! - Alert configuration access
//!
//! # What this module does NOT handle:
//! - High-level alert operations (see [`crate::client::alerts`])
//! - Result parsing beyond JSON deserialization

use reqwest::Client;
use tracing::debug;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::{FiredAlert, FiredAlertListResponse};
use crate::name_merge::attach_entry_name;

/// List fired alerts.
///
/// Returns a summary of triggered alerts from `/services/alerts/fired_alerts`.
#[allow(clippy::too_many_arguments)]
pub async fn list_fired_alerts(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    count: Option<usize>,
    offset: Option<usize>,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Vec<FiredAlert>> {
    debug!("Listing fired alerts");

    let url = format!("{}/services/alerts/fired_alerts", base_url);

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
        "/services/alerts/fired_alerts",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let resp: FiredAlertListResponse = response.json().await.map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse fired alerts response: {}", e))
    })?;

    Ok(resp
        .entry
        .into_iter()
        .map(|e| attach_entry_name(e.name, e.content))
        .collect())
}

/// Get a specific fired alert by name.
///
/// Retrieves details about a specific triggered alert instance.
#[allow(clippy::too_many_arguments)]
pub async fn get_fired_alert(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    name: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<FiredAlert> {
    debug!("Getting fired alert: {}", name);

    let url = format!("{}/services/alerts/fired_alerts/{}", base_url, name);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .query(&[("output_mode", "json")]);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/alerts/fired_alerts/{name}",
        "GET",
        metrics,
        circuit_breaker,
    )
    .await?;

    let body: FiredAlertListResponse = response.json().await.map_err(|e| {
        ClientError::InvalidResponse(format!("Failed to parse fired alert response: {}", e))
    })?;

    let entry = body
        .entry
        .into_iter()
        .next()
        .ok_or_else(|| ClientError::NotFound(format!("Fired alert '{}' not found", name)))?;

    Ok(attach_entry_name(entry.name, entry.content))
}
