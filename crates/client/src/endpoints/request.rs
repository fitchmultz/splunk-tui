//! Retry helper for HTTP requests with exponential backoff.
//!
//! This module provides functionality to automatically retry HTTP requests
//! that fail with HTTP 429 (Too Many Requests) status codes, using
//! exponential backoff between retry attempts.

use reqwest::{RequestBuilder, Response};
use tracing::debug;

use crate::error::{ClientError, Result};
use crate::models::SplunkMessages;

/// Maximum number of retry attempts for rate-limited requests.
const DEFAULT_MAX_RETRIES: usize = 3;

/// Sends an HTTP request with automatic retry logic for HTTP 429 responses.
///
/// This function wraps a `reqwest::RequestBuilder` with retry logic that:
/// - Detects HTTP 429 (Too Many Requests) status codes
/// - Implements exponential backoff (1s, 2s, 4s = 2^attempt)
/// - Respects the `max_retries` parameter
/// - Logs retry attempts with `tracing::debug`
/// - Returns `MaxRetriesExceeded` error when retries are exhausted
///
/// # Arguments
///
/// * `builder` - The `reqwest::RequestBuilder` to execute
/// * `max_retries` - Maximum number of retry attempts (defaults to 3 if 0)
///
/// # Returns
///
/// * `Result<Response>` - The successful HTTP response or an error
///
/// # Errors
///
/// Returns `ClientError::MaxRetriesExceeded` when all retry attempts are exhausted.
/// Propagates other `reqwest` errors as `ClientError::ReqwestError`.
pub async fn send_request_with_retry(
    builder: RequestBuilder,
    max_retries: usize,
) -> Result<Response> {
    let max_retries = if max_retries == 0 {
        DEFAULT_MAX_RETRIES
    } else {
        max_retries
    };

    for attempt in 0..=max_retries {
        // Try to clone the builder for this attempt
        // On first attempt (0), we try to clone to see if retry is possible
        // On subsequent attempts, we clone again for the retry
        let attempt_builder = match builder.try_clone() {
            Some(cloned) => cloned,
            None => {
                // Can't clone - this is either:
                // 1. First attempt with a non-clonable builder - use it directly
                // 2. Subsequent attempt but can't clone - error out
                if attempt == 0 {
                    debug!("Request builder cannot be cloned, single attempt only");
                    return builder.send().await.map_err(ClientError::from);
                } else {
                    debug!("Cannot clone request builder for retry");
                    return Err(ClientError::MaxRetriesExceeded(attempt));
                }
            }
        };

        match attempt_builder.send().await {
            Ok(response) if response.status().as_u16() == 429 => {
                if attempt < max_retries {
                    // Calculate exponential backoff: 2^attempt seconds
                    let backoff_secs = 2u64.pow(attempt as u32);
                    debug!(
                        attempt = attempt + 1,
                        max_retries = max_retries + 1,
                        backoff_secs = backoff_secs,
                        "Rate limited (HTTP 429), retrying with exponential backoff"
                    );

                    tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                } else {
                    debug!(
                        attempts = attempt + 1,
                        "Max retries exhausted for rate-limited request"
                    );
                    return Err(ClientError::MaxRetriesExceeded(max_retries + 1));
                }
            }
            Ok(response) => {
                if response.status().is_success() {
                    // Successful response
                    if attempt > 0 {
                        debug!(attempt = attempt + 1, "Request succeeded after retry");
                    }
                    return Ok(response);
                } else {
                    // Handle non-success status codes
                    let status = response.status().as_u16();
                    let url = response.url().to_string();
                    let request_id = response
                        .headers()
                        .get("X-Splunk-Request-Id")
                        .and_then(|h| h.to_str().ok())
                        .map(|s| s.to_string());
                    let body = response
                        .text()
                        .await
                        .unwrap_or_else(|_| "Could not read error response body".to_string());

                    // Try to parse Splunk error messages for a cleaner display
                    let message = if let Ok(m) = serde_json::from_str::<SplunkMessages>(&body) {
                        m.messages
                            .iter()
                            .map(|msg| format!("{}: {}", msg.message_type, msg.text))
                            .collect::<Vec<_>>()
                            .join("; ")
                    } else {
                        body
                    };

                    return Err(ClientError::ApiError {
                        status,
                        url,
                        message,
                        request_id,
                    });
                }
            }
            Err(e) => {
                // For non-429 errors, propagate immediately
                return Err(ClientError::from(e));
            }
        }
    }

    // This should never be reached, but handle it for completeness
    Err(ClientError::MaxRetriesExceeded(max_retries + 1))
}
