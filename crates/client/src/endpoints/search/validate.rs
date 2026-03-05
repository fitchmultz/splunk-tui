//! SPL validation operations.
//!
//! This module provides endpoints for validating SPL (Search Processing Language) syntax.
//!
//! # What this module handles:
//! - SPL syntax validation via Splunk's parser endpoint
//! - Extracting warnings from validation responses
//! - Extracting errors from validation responses
//!
//! # What this module does NOT handle:
//! - Search job execution (see [`super::jobs`])
//! - Saved search management (see [`super::saved`])

use reqwest::Client;
use reqwest::StatusCode;
use tracing::debug;

use crate::redact_query;

use crate::client::circuit_breaker::CircuitBreaker;
use crate::endpoints::send_request_with_retry;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::ValidateSplResponse;

/// Validate SPL syntax using Splunk's search parser endpoint.
///
/// Sends the query to `/services/search/parser` which parses the SPL
/// and returns either a parse tree (on success) or error details (on failure).
///
/// # Arguments
/// * `client` - The reqwest HTTP client
/// * `base_url` - The Splunk base URL
/// * `auth_token` - Authentication token
/// * `search` - The SPL query to validate
/// * `max_retries` - Maximum number of retries for transient failures
/// * `metrics` - Optional metrics collector
///
/// # Returns
/// * `Ok(ValidateSplResponse)` - Validation result with errors/warnings
/// * `Err(ClientError)` - Transport or API error
///
/// # Note
/// This endpoint returns HTTP 200 for valid SPL and HTTP 400 for syntax errors.
/// Both are considered "successful" responses from a validation perspective.
#[allow(clippy::too_many_arguments)]
pub async fn validate_spl(
    client: &Client,
    base_url: &str,
    auth_token: &str,
    search: &str,
    max_retries: usize,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<ValidateSplResponse> {
    // Security: Log only redacted query to avoid exposing sensitive data (tokens, PII, etc.)
    debug!("Validating SPL syntax: {}", redact_query(search));

    let url = format!("{}/services/search/parser", base_url);

    let form_data = [("q", search), ("output_mode", "json")];

    let builder = client
        .post(&url)
        .header("Authorization", format!("Bearer {}", auth_token))
        .form(&form_data);

    let response = send_request_with_retry(
        builder,
        max_retries,
        "/services/search/parser",
        "POST",
        metrics,
        circuit_breaker,
    )
    .await?;

    let status = response.status();
    let body_text = response.text().await?;

    match status {
        StatusCode::OK => {
            // Valid SPL - parse any warnings from response
            let body: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
                ClientError::InvalidResponse(format!("Failed to parse validation response: {}", e))
            })?;

            let warnings = extract_warnings(&body);

            Ok(ValidateSplResponse {
                valid: true,
                errors: vec![],
                warnings,
            })
        }
        StatusCode::BAD_REQUEST => {
            // Syntax error - parse error details
            let body: serde_json::Value = serde_json::from_str(&body_text).map_err(|e| {
                ClientError::InvalidResponse(format!("Failed to parse validation error: {}", e))
            })?;

            let errors = extract_errors(&body);

            Ok(ValidateSplResponse {
                valid: false,
                errors,
                warnings: vec![],
            })
        }
        _ => Err(ClientError::ApiError {
            status: status.as_u16(),
            url,
            message: body_text,
            request_id: None,
        }),
    }
}

/// Extract warnings from parser response.
fn extract_warnings(body: &serde_json::Value) -> Vec<crate::models::SplWarning> {
    let mut warnings = vec![];

    // Splunk may return warnings in different formats depending on version
    if let Some(messages) = body.get("messages")
        && let Some(arr) = messages.as_array()
    {
        for msg in arr {
            if let Some(text) = msg.get("text").and_then(|t| t.as_str()) {
                warnings.push(crate::models::SplWarning {
                    message: text.to_string(),
                    line: msg.get("line").and_then(|l| l.as_u64()).map(|n| n as u32),
                    column: msg.get("column").and_then(|c| c.as_u64()).map(|n| n as u32),
                });
            }
        }
    }

    warnings
}

/// Extract errors from parser error response.
fn extract_errors(body: &serde_json::Value) -> Vec<crate::models::SplError> {
    let mut errors = vec![];

    // Try to extract from messages array first
    if let Some(messages) = body.get("messages")
        && let Some(arr) = messages.as_array()
    {
        for msg in arr {
            if let Some(text) = msg.get("text").and_then(|t| t.as_str()) {
                errors.push(crate::models::SplError {
                    message: text.to_string(),
                    line: msg.get("line").and_then(|l| l.as_u64()).map(|n| n as u32),
                    column: msg.get("column").and_then(|c| c.as_u64()).map(|n| n as u32),
                });
            }
        }
    }

    // If no messages array, look for error field
    if errors.is_empty()
        && let Some(error) = body.get("error").and_then(|e| e.as_str())
    {
        errors.push(crate::models::SplError {
            message: error.to_string(),
            line: None,
            column: None,
        });
    }

    // Last resort: use the entire body as error message
    if errors.is_empty() {
        errors.push(crate::models::SplError {
            message: body.to_string(),
            line: None,
            column: None,
        });
    }

    errors
}
