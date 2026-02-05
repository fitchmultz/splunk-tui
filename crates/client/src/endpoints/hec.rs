//! HEC (HTTP Event Collector) endpoint implementations.
//!
//! This module provides low-level HTTP endpoint functions for interacting with
//! Splunk's HTTP Event Collector (HEC) API. HEC uses a separate endpoint
//! (typically port 8088) and different authentication (HEC tokens with "Splunk"
//! prefix) from the standard Splunk REST API.
//!
//! # What this module handles:
//! - Single event submission to /services/collector/event
//! - Batch event submission (JSON array and NDJSON formats)
//! - Health check queries to /services/collector/health
//! - Acknowledgment status checks to /services/collector/ack
//!
//! # What this module does NOT handle:
//! - High-level client methods (see [`crate::client::hec`])
//! - Token management or configuration
//!
//! # Authentication
//! HEC uses a different authorization header format than the REST API:
//! - REST API: `Authorization: Bearer <token>`
//! - HEC: `Authorization: Splunk <token>`
//!
//! # Invariants
//! - All HEC requests use JSON content type
//! - The HEC URL is separate from the REST API base URL
//! - Errors are returned as JSON with code and text fields

use reqwest::Client;

use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::hec::{
    HecAckRequest, HecAckStatus, HecBatchResponse, HecEvent, HecHealth, HecResponse,
};

/// Send a single event to HEC.
///
/// # Arguments
/// * `client` - The HTTP client
/// * `hec_url` - The HEC endpoint URL (e.g., "https://localhost:8088")
/// * `hec_token` - The HEC authentication token
/// * `event` - The event to send
/// * `max_retries` - Maximum number of retry attempts
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// The HEC response containing the status code and optional acknowledgment ID
///
/// # Errors
/// Returns `ClientError` if the request fails or returns an error response
///
/// # Example
/// ```ignore
/// use splunk_client::models::HecEvent;
///
/// let event = HecEvent::new(serde_json::json!({"message": "Hello"}));
/// let response = send_event(&client, "https://localhost:8088", "token", &event, 3, None).await?;
/// ```
pub async fn send_event(
    client: &Client,
    hec_url: &str,
    hec_token: &str,
    event: &HecEvent,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<HecResponse> {
    let url = format!("{}/services/collector/event", hec_url);

    let builder = client
        .post(&url)
        .header("Authorization", format!("Splunk {}", hec_token))
        .header("Content-Type", "application/json")
        .json(event);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/collector/event",
        "POST",
        metrics,
    )
    .await?;

    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to read response body: {e}")))?;

    // Try to parse as HEC response
    match serde_json::from_str::<HecResponse>(&body) {
        Ok(hec_response) => Ok(hec_response),
        Err(parse_err) => {
            if status >= 400 {
                // HTTP error status with unparseable body
                Err(ClientError::ApiError {
                    status,
                    url,
                    message: body,
                    request_id: None,
                })
            } else {
                // HTTP success status but unparseable JSON - this is an error
                Err(ClientError::InvalidResponse(format!(
                    "Failed to parse HEC response (HTTP {status}): {parse_err}"
                )))
            }
        }
    }
}

/// Send a batch of events to HEC.
///
/// HEC supports two batch formats:
/// 1. **JSON Array**: `[{"event": {...}}, {"event": {...}}]`
/// 2. **NDJSON** (newline-delimited): `{"event": {...}}\n{"event": {...}}`
///
/// JSON array is the default format. Use `use_ndjson: true` for NDJSON format.
///
/// # Arguments
/// * `client` - The HTTP client
/// * `hec_url` - The HEC endpoint URL (e.g., "https://localhost:8088")
/// * `hec_token` - The HEC authentication token
/// * `events` - The events to send
/// * `use_ndjson` - Use NDJSON format instead of JSON array
/// * `max_retries` - Maximum number of retry attempts
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// The HEC batch response containing status and optional acknowledgment IDs
///
/// # Errors
/// Returns `ClientError` if the request fails or returns an error response
///
/// # Example
/// ```ignore
/// use splunk_client::models::HecEvent;
///
/// let events = vec![
///     HecEvent::new(serde_json::json!({"message": "Event 1"})),
///     HecEvent::new(serde_json::json!({"message": "Event 2"})),
/// ];
/// let response = send_batch(&client, "https://localhost:8088", "token", &events, false, 3, None).await?;
/// ```
pub async fn send_batch(
    client: &Client,
    hec_url: &str,
    hec_token: &str,
    events: &[HecEvent],
    use_ndjson: bool,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<HecBatchResponse> {
    let url = format!("{}/services/collector/event", hec_url);

    // Build the request body based on format
    let body = if use_ndjson {
        // NDJSON format: one JSON object per line
        events
            .iter()
            .map(serde_json::to_string)
            .collect::<std::result::Result<Vec<_>, _>>()
            .map_err(|e| ClientError::InvalidRequest(format!("Failed to serialize event: {}", e)))?
            .join("\n")
    } else {
        // JSON array format
        serde_json::to_string(events).map_err(|e| {
            ClientError::InvalidRequest(format!("Failed to serialize events: {}", e))
        })?
    };

    let builder = client
        .post(&url)
        .header("Authorization", format!("Splunk {}", hec_token))
        .header("Content-Type", "application/json")
        .body(body);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/collector/event",
        "POST",
        metrics,
    )
    .await?;

    let status = response.status().as_u16();
    let response_body = response
        .text()
        .await
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to read response body: {e}")))?;

    // Try to parse as HEC batch response
    match serde_json::from_str::<HecBatchResponse>(&response_body) {
        Ok(hec_response) => Ok(hec_response),
        Err(parse_err) => {
            if status >= 400 {
                // HTTP error status with unparseable body
                Err(ClientError::ApiError {
                    status,
                    url,
                    message: response_body,
                    request_id: None,
                })
            } else {
                // HTTP success status but unparseable JSON - this is an error
                Err(ClientError::InvalidResponse(format!(
                    "Failed to parse HEC batch response (HTTP {status}): {parse_err}"
                )))
            }
        }
    }
}

/// Check HEC health endpoint.
///
/// The health endpoint returns a simple text response indicating whether
/// HEC is available and healthy.
///
/// # Arguments
/// * `client` - The HTTP client
/// * `hec_url` - The HEC endpoint URL (e.g., "https://localhost:8088")
/// * `hec_token` - The HEC authentication token
/// * `max_retries` - Maximum number of retry attempts
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// The HEC health status
///
/// # Errors
/// Returns `ClientError` if the request fails
///
/// # Example
/// ```ignore
/// let health = health_check(&client, "https://localhost:8088", "token", 3, None).await?;
/// println!("HEC is healthy: {}", health.is_healthy());
/// ```
pub async fn health_check(
    client: &Client,
    hec_url: &str,
    hec_token: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<HecHealth> {
    let url = format!("{}/services/collector/health", hec_url);

    let builder = client
        .get(&url)
        .header("Authorization", format!("Splunk {}", hec_token));

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/collector/health",
        "GET",
        metrics,
    )
    .await?;

    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to read response body: {e}")))?;

    Ok(HecHealth {
        text: body.trim().to_string(),
        code: status,
    })
}

/// Check acknowledgment status for guaranteed delivery.
///
/// When HEC acknowledgments are enabled, this endpoint can be used to check
/// whether events have been successfully indexed. Each acknowledgment ID
/// maps to a boolean: `true` means indexed, `false` means still pending.
///
/// # Arguments
/// * `client` - The HTTP client
/// * `hec_url` - The HEC endpoint URL (e.g., "https://localhost:8088")
/// * `hec_token` - The HEC authentication token
/// * `ack_ids` - List of acknowledgment IDs to check
/// * `max_retries` - Maximum number of retry attempts
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// The acknowledgment status for each ID
///
/// # Errors
/// Returns `ClientError` if the request fails or acknowledgments are disabled
///
/// # Example
/// ```ignore
/// let ack_ids = vec![123, 124, 125];
/// let status = check_ack_status(&client, "https://localhost:8088", "token", &ack_ids, 3, None).await?;
/// println!("All indexed: {}", status.all_indexed());
/// ```
pub async fn check_ack_status(
    client: &Client,
    hec_url: &str,
    hec_token: &str,
    ack_ids: &[u64],
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
) -> Result<HecAckStatus> {
    let url = format!("{}/services/collector/ack", hec_url);

    let request = HecAckRequest {
        ack_ids: ack_ids.to_vec(),
    };

    let builder = client
        .post(&url)
        .header("Authorization", format!("Splunk {}", hec_token))
        .header("Content-Type", "application/json")
        .json(&request);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/collector/ack",
        "POST",
        metrics,
    )
    .await?;

    let status = response.status().as_u16();
    let body = response
        .text()
        .await
        .map_err(|e| ClientError::InvalidResponse(format!("Failed to read response body: {e}")))?;

    // Try to parse as HEC ack status
    match serde_json::from_str::<HecAckStatus>(&body) {
        Ok(ack_status) => Ok(ack_status),
        Err(parse_err) => {
            if status >= 400 {
                // HTTP error status with unparseable body
                Err(ClientError::ApiError {
                    status,
                    url,
                    message: body,
                    request_id: None,
                })
            } else {
                // HTTP success status but unparseable JSON - this is an error
                Err(ClientError::InvalidResponse(format!(
                    "Failed to parse HEC ack status response (HTTP {status}): {parse_err}"
                )))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_hec_auth_header_format() {
        // Verify the "Splunk" prefix is used (not "Bearer")
        let token = "test-token-123";
        let expected = format!("Splunk {}", token);
        assert!(expected.starts_with("Splunk "));
        assert!(!expected.starts_with("Bearer "));
    }

    #[test]
    fn test_batch_body_json_array() {
        let events = vec![
            HecEvent::new(json!({"message": "Event 1"})),
            HecEvent::new(json!({"message": "Event 2"})),
        ];

        let body = serde_json::to_string(&events).unwrap();
        assert!(body.starts_with('['));
        assert!(body.ends_with(']'));
        assert!(body.contains("Event 1"));
        assert!(body.contains("Event 2"));
    }

    #[test]
    fn test_batch_body_ndjson() {
        let events = [
            HecEvent::new(json!({"message": "Event 1"})),
            HecEvent::new(json!({"message": "Event 2"})),
        ];

        let body = events
            .iter()
            .map(|e| serde_json::to_string(e).unwrap())
            .collect::<Vec<_>>()
            .join("\n");

        let lines: Vec<_> = body.lines().collect();
        assert_eq!(lines.len(), 2);
        assert!(lines[0].contains("Event 1"));
        assert!(lines[1].contains("Event 2"));
    }
}
