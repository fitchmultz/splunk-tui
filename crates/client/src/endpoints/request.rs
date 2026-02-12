//! Retry helper for HTTP requests with exponential backoff.
//!
//! This module provides functionality to automatically retry HTTP requests
//! that fail with transient errors, using exponential backoff between retry
//! attempts.
//!
//! Retryable conditions:
//! - HTTP 429 (Too Many Requests): Rate limiting, respects `Retry-After` header
//! - HTTP 502 (Bad Gateway): Transient server error
//! - HTTP 503 (Service Unavailable): Transient server error
//! - HTTP 504 (Gateway Timeout): Transient server error
//! - Transport errors: Connection refused, connection reset, timeouts
//!
//! The retry logic respects the `Retry-After` response header when present
//! for 429 responses, using the maximum of the calculated exponential backoff
//! and the server's suggested delay. Both delay-seconds format (e.g., "120")
//! and HTTP-date format (e.g., "Wed, 21 Oct 2015 07:28:00 GMT") are supported
//! per RFC 7231.
//!
//! For 5xx errors and transport errors, exponential backoff is used without
//! Retry-After header support.
//!
//! ## Retry Limitation for Non-Cloneable Requests
//!
//! Requests with streaming bodies (e.g., file uploads, multipart forms) cannot be
//! retried because the request body can only be consumed once. If such a request
//! fails with a retryable error, it will fail immediately without retry attempts.
//!
//! This limitation is detected when `reqwest::RequestBuilder::try_clone()` returns
//! `None`. Applications requiring retry guarantees should use non-streaming request
//! bodies or implement application-level retry logic.

use reqwest::{RequestBuilder, Response};
use std::time::{Duration, Instant};
use time::OffsetDateTime;
use time::format_description::well_known::Rfc2822;
use tracing::field::Empty;
use tracing::{debug, instrument, warn};

use crate::client::circuit_breaker::CircuitBreaker;
use crate::error::{ClientError, Result};
use crate::metrics::MetricsCollector;
use crate::models::SplunkMessages;
use crate::tracing::inject_trace_context;
use opentelemetry::trace::TraceContextExt;

/// Parses the Retry-After header from an HTTP response.
///
/// Supports both delay-seconds and HTTP-date formats according to RFC 7231:
/// - delay-seconds: a decimal integer number of seconds (e.g., "120")
/// - HTTP-date: an IMF-fixdate (RFC 7231) / RFC 2822 format date string
///   (e.g., "Wed, 21 Oct 2015 07:28:00 GMT")
///
/// Returns `None` if the header is not present, cannot be parsed, is zero
/// (for delay-seconds), or represents a time in the past (for HTTP-date).
fn parse_retry_after(response: &Response) -> Option<Duration> {
    response
        .headers()
        .get("retry-after")
        .and_then(|header_value| header_value.to_str().ok())
        .and_then(|header_str| {
            // Try delay-seconds format first (e.g., "120")
            // This is the most common format for rate limiting
            if header_str.chars().all(|c| c.is_ascii_digit()) {
                return header_str
                    .parse::<u64>()
                    .ok()
                    .filter(|&secs| secs > 0)
                    .map(Duration::from_secs);
            }

            // Try HTTP-date format (RFC 7231 / RFC 2822)
            // e.g., "Wed, 21 Oct 2015 07:28:00 GMT"
            match OffsetDateTime::parse(header_str, &Rfc2822) {
                Ok(retry_time) => {
                    let now = OffsetDateTime::now_utc();
                    if retry_time > now {
                        let duration = retry_time - now;
                        Some(Duration::from_secs(duration.whole_seconds().max(0) as u64))
                    } else {
                        // Date is in the past, fall back to exponential backoff
                        None
                    }
                }
                Err(_) => None,
            }
        })
}

/// Check if a reqwest error is retryable using structured error classification.
///
/// Uses reqwest's `is_timeout()` method for robust detection of transient
/// transport errors that may succeed on retry.
///
/// # Retryable Conditions
/// - Timeout errors (`is_timeout()`): Request or response timeout
///
/// # Non-Retryable Conditions
/// - Connection errors (`is_connect()`): DNS resolution failures, connection refused,
///   connection reset, network unreachable - these indicate the server is not
///   reachable or not running, and retrying won't help
/// - TLS/SSL errors (certificate validation, handshake failures)
/// - Request construction errors
/// - Response decoding errors
fn is_retryable_transport_error(error: &reqwest::Error) -> bool {
    error.is_timeout()
}

/// Parses a Splunk error response body into a displayable message string.
///
/// Attempts to parse the body as JSON containing Splunk's standard error message
/// format (`SplunkMessages`). If successful, formats each message as "{type}: {text}"
/// and joins them with "; ". If parsing fails, returns the raw body as a fallback.
///
/// # Arguments
///
/// * `body` - The raw response body string
///
/// # Returns
///
/// A formatted error message string suitable for display
fn parse_splunk_error_response(body: &str) -> String {
    if let Ok(messages) = serde_json::from_str::<SplunkMessages>(body) {
        messages
            .messages
            .iter()
            .map(|msg| format!("{}: {}", msg.message_type, msg.text))
            .collect::<Vec<_>>()
            .join("; ")
    } else {
        body.to_string()
    }
}

/// Sends an HTTP request with automatic retry logic for transient errors.
///
/// This function wraps a `reqwest::RequestBuilder` with retry logic that:
/// - Detects HTTP 429 (Too Many Requests) status codes and respects `Retry-After`
/// - Detects HTTP 502/503/504 (transient server errors) with exponential backoff
/// - Detects retryable transport errors (connection refused, reset, timeouts)
/// - Implements exponential backoff (1s, 2s, 4s = 2^attempt)
/// - Respects the `Retry-After` header when present for 429 responses, using
///   the maximum of the calculated backoff and the server's suggested delay
/// - Respects the `max_retries` parameter
/// - Logs retry attempts with `tracing::debug`
/// - Returns `MaxRetriesExceeded` error when retries are exhausted
/// - Records metrics for request duration, retries, and errors (if metrics collector provided)
///
/// # Arguments
///
/// * `builder` - The `reqwest::RequestBuilder` to execute
/// * `max_retries` - Maximum number of retry attempts (defaults to 3 if 0)
/// * `endpoint` - The API endpoint path for metrics labeling (e.g., "/services/search/jobs")
/// * `method` - The HTTP method for metrics labeling (e.g., "GET", "POST")
/// * `metrics` - Optional metrics collector for recording request metrics
/// * `circuit_breaker` - Optional circuit breaker for tracking endpoint health
///
/// # Returns
///
/// * `Result<Response>` - The successful HTTP response or an error
///
/// # Errors
///
/// Returns `ClientError::MaxRetriesExceeded` when all retry attempts are exhausted.
/// Returns `ClientError::CircuitBreakerOpen` when the circuit breaker is open.
/// Propagates other `reqwest` errors as `ClientError::ReqwestError`.
///
/// # Limitations
///
/// Requests with streaming bodies cannot be retried. If `try_clone()` fails on
/// the first attempt, the request is sent without retry capability. Callers
/// requiring guaranteed retries should use non-streaming request bodies.
#[instrument(
    skip(builder, metrics, circuit_breaker),
    fields(
        endpoint = endpoint,
        method = method,
        attempt = Empty,
        retry_count = Empty,
        duration_ms = Empty,
        status = Empty,
        error = Empty,
        trace_id = Empty,
    ),
    level = "debug"
)]
#[allow(clippy::too_many_arguments)]
pub async fn send_request_with_retry(
    builder: RequestBuilder,
    max_retries: usize,
    endpoint: &str,
    method: &str,
    metrics: Option<&MetricsCollector>,
    circuit_breaker: Option<&CircuitBreaker>,
) -> Result<Response> {
    let start_time = Instant::now();

    // Record trace_id if OTel is enabled
    let trace_id = opentelemetry::Context::current()
        .span()
        .span_context()
        .trace_id()
        .to_string();
    tracing::Span::current().record("trace_id", &trace_id);

    // Inject trace context into request headers for distributed tracing
    let builder = inject_trace_context(builder);

    // Record the initial request attempt
    if let Some(m) = metrics {
        m.record_request(endpoint, method);
    }

    for attempt in 0..=max_retries {
        // Check circuit breaker before each attempt (fail fast if opened during retry sleep)
        if let Some(cb) = circuit_breaker {
            if let Err(e) = cb.check(endpoint) {
                warn!(
                    endpoint = endpoint,
                    attempt = attempt + 1,
                    "Circuit breaker open, failing fast"
                );
                return Err(ClientError::from(e));
            }
        }

        // Record current attempt number in span
        tracing::Span::current().record("attempt", attempt as i64 + 1);

        // Try to clone the builder for this attempt
        // On first attempt (0), we try to clone to see if retry is possible
        // On subsequent attempts, we clone again for the retry
        let attempt_builder = match builder.try_clone() {
            Some(cloned) => cloned,
            None => {
                // Can't clone - this typically happens with streaming request bodies
                // (file uploads, multipart forms) where the body can only be consumed once.
                // In such cases, retry is impossible because the body is already consumed
                // on the first attempt.
                if attempt == 0 {
                    warn!("Request builder cannot be cloned, single attempt only");
                    let result = builder.send().await.map_err(ClientError::from);
                    // Record metrics for the result
                    if let Some(m) = metrics {
                        let duration = start_time.elapsed();
                        let status = result.as_ref().ok().map(|r| r.status().as_u16());
                        m.record_request_duration(endpoint, method, duration, status);
                        if let Err(ref e) = result {
                            m.record_client_error(endpoint, method, e);
                        }
                    }
                    return result;
                } else {
                    warn!("Cannot clone request builder for retry");
                    let err = ClientError::MaxRetriesExceeded(
                        attempt,
                        Box::new(ClientError::InvalidResponse(
                            "Request body cannot be cloned for retry (streaming body may have been consumed)".to_string()
                        ))
                    );
                    if let Some(m) = metrics {
                        let duration = start_time.elapsed();
                        m.record_request_duration(endpoint, method, duration, None);
                        m.record_client_error(endpoint, method, &err);
                    }
                    return Err(err);
                }
            }
        };

        match attempt_builder.send().await {
            Ok(response) => {
                let status = response.status();
                let status_u16 = status.as_u16();

                // Record status and duration in span
                let duration_ms = start_time.elapsed().as_millis() as i64;
                tracing::Span::current().record("status", status_u16 as i64);
                tracing::Span::current().record("duration_ms", duration_ms);
                tracing::Span::current().record("retry_count", attempt as i64);

                if status.is_success() {
                    // Successful response
                    if attempt > 0 {
                        debug!(attempt = attempt + 1, "Request succeeded after retry");
                    }
                    if let Some(m) = metrics {
                        let duration = start_time.elapsed();
                        m.record_request_duration(endpoint, method, duration, Some(status_u16));
                    }
                    // Record success in circuit breaker
                    if let Some(cb) = circuit_breaker {
                        cb.record_success(endpoint);
                    }
                    return Ok(response);
                }

                // Record failure in circuit breaker for server errors and 429
                if status.is_server_error() || status_u16 == 429 {
                    if let Some(cb) = circuit_breaker {
                        cb.record_failure(endpoint);
                    }
                }

                // Check for retryable status codes (429 or 5xx)
                if ClientError::is_retryable_status(status_u16) {
                    if attempt < max_retries {
                        // Record retry metric
                        if let Some(m) = metrics {
                            m.record_retry(endpoint, method, attempt + 1);
                        }

                        // Calculate exponential backoff: 2^attempt seconds
                        let backoff_secs = 2u64.pow(attempt as u32);

                        // For 429, check for Retry-After header
                        if status_u16 == 429 {
                            let retry_after = parse_retry_after(&response);

                            let sleep_duration = if let Some(retry_after_duration) = retry_after {
                                let retry_after_secs = retry_after_duration.as_secs();
                                // Use the larger of exponential backoff or Retry-After value
                                let sleep_secs = backoff_secs.max(retry_after_secs);
                                debug!(
                                    attempt = attempt + 1,
                                    max_retries = max_retries + 1,
                                    backoff_secs = backoff_secs,
                                    retry_after_secs = retry_after_secs,
                                    sleep_secs = sleep_secs,
                                    "Rate limited (HTTP 429), using max of backoff and Retry-After"
                                );
                                sleep_secs
                            } else {
                                debug!(
                                    attempt = attempt + 1,
                                    max_retries = max_retries + 1,
                                    backoff_secs = backoff_secs,
                                    "Rate limited (HTTP 429), retrying with exponential backoff (no Retry-After header)"
                                );
                                backoff_secs
                            };

                            tokio::time::sleep(tokio::time::Duration::from_secs(sleep_duration))
                                .await;
                        } else {
                            // For 5xx errors, use exponential backoff only
                            debug!(
                                attempt = attempt + 1,
                                max_retries = max_retries + 1,
                                status = status_u16,
                                backoff_secs = backoff_secs,
                                "Server error, retrying with exponential backoff"
                            );
                            tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs))
                                .await;
                        }
                    } else {
                        debug!(
                            attempts = attempt + 1,
                            status = status_u16,
                            "Max retries exhausted for retryable request"
                        );
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

                        let message = parse_splunk_error_response(&body);

                        let classified_err =
                            ClientError::from_status_response(status_u16, url, message, request_id);

                        let err = ClientError::MaxRetriesExceeded(
                            max_retries + 1,
                            Box::new(classified_err),
                        );
                        // Record error in span
                        tracing::Span::current().record(
                            "error",
                            format!("MaxRetriesExceeded: {}", status_u16).as_str(),
                        );
                        if let Some(m) = metrics {
                            let duration = start_time.elapsed();
                            m.record_request_duration(endpoint, method, duration, Some(status_u16));
                            m.record_client_error(endpoint, method, &err);
                        }
                        return Err(err);
                    }
                } else {
                    // Non-retryable error: extract details and return ApiError
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

                    let message = parse_splunk_error_response(&body);

                    let err = ClientError::from_status_response(
                        status_u16,
                        url,
                        message.clone(),
                        request_id,
                    );
                    // Record error in span
                    tracing::Span::current()
                        .record("error", format!("ApiError: {}", message).as_str());
                    if let Some(m) = metrics {
                        let duration = start_time.elapsed();
                        m.record_request_duration(endpoint, method, duration, Some(status_u16));
                        m.record_client_error(endpoint, method, &err);
                    }
                    return Err(err);
                }
            }
            Err(e) => {
                // Record failure in circuit breaker for transport errors
                if let Some(cb) = circuit_breaker {
                    cb.record_failure(endpoint);
                }

                // Check if this is a retryable transport error
                if is_retryable_transport_error(&e) && attempt < max_retries {
                    // Record retry metric
                    if let Some(m) = metrics {
                        m.record_retry(endpoint, method, attempt + 1);
                    }

                    let backoff_secs = 2u64.pow(attempt as u32);

                    debug!(
                        attempt = attempt + 1,
                        max_retries = max_retries + 1,
                        error = %e,
                        backoff_secs = backoff_secs,
                        "Transport error, retrying with exponential backoff"
                    );

                    tokio::time::sleep(tokio::time::Duration::from_secs(backoff_secs)).await;
                } else {
                    // Non-retryable error: classify and propagate immediately
                    let err = ClientError::from_reqwest_error_classified(e);
                    let error_str = err.to_string();
                    // Record error in span
                    tracing::Span::current().record("error", error_str.as_str());
                    tracing::Span::current()
                        .record("duration_ms", start_time.elapsed().as_millis() as i64);
                    if let Some(m) = metrics {
                        let duration = start_time.elapsed();
                        m.record_request_duration(endpoint, method, duration, None);
                        m.record_client_error(endpoint, method, &err);
                    }
                    return Err(err);
                }
            }
        }
    }

    // This should never be reached, but handle it for completeness
    let err = ClientError::MaxRetriesExceeded(
        max_retries + 1,
        Box::new(ClientError::InvalidResponse(
            "Retry loop exited without resolution".to_string(),
        )),
    );
    if let Some(m) = metrics {
        let duration = start_time.elapsed();
        m.record_request_duration(endpoint, method, duration, None);
        m.record_client_error(endpoint, method, &err);
    }
    Err(err)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_splunk_error_response_with_valid_json() {
        let json = r#"{"messages": [{"type": "ERROR", "text": "Invalid credentials"}]}"#;
        let result = parse_splunk_error_response(json);
        assert_eq!(result, "ERROR: Invalid credentials");
    }

    #[test]
    fn test_parse_splunk_error_response_with_multiple_messages() {
        let json = r#"{"messages": [
            {"type": "ERROR", "text": "First error"},
            {"type": "WARN", "text": "Second warning"}
        ]}"#;
        let result = parse_splunk_error_response(json);
        assert_eq!(result, "ERROR: First error; WARN: Second warning");
    }

    #[test]
    fn test_parse_splunk_error_response_with_empty_messages() {
        let json = r#"{"messages": []}"#;
        let result = parse_splunk_error_response(json);
        assert_eq!(result, "");
    }

    #[test]
    fn test_parse_splunk_error_response_with_invalid_json() {
        let body = "Raw error message without JSON structure";
        let result = parse_splunk_error_response(body);
        assert_eq!(result, "Raw error message without JSON structure");
    }

    #[test]
    fn test_parse_splunk_error_response_with_non_object_json() {
        let json = r#"["not", "an", "object"]"#;
        let result = parse_splunk_error_response(json);
        assert_eq!(result, r#"["not", "an", "object"]"#);
    }
}
