//! Structured error details for UI display.

use serde::{Deserialize, Serialize};
use splunk_client::models::SplunkMessage;
use std::collections::HashMap;

/// Structured error information captured from failed operations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ErrorDetails {
    /// Human-readable error summary
    pub summary: String,

    /// HTTP status code (if applicable)
    pub status_code: Option<u16>,

    /// Request URL that failed
    pub url: Option<String>,

    /// Splunk request ID for debugging
    pub request_id: Option<String>,

    /// Parsed error messages from Splunk
    pub messages: Vec<SplunkMessage>,

    /// Raw error response body for inspection
    pub raw_body: Option<String>,

    /// Timestamp when error occurred
    pub timestamp: String,

    /// Additional context (e.g., search query, job SID)
    pub context: HashMap<String, String>,
}

impl ErrorDetails {
    /// Create ErrorDetails from ClientError.
    pub fn from_client_error(error: &splunk_client::ClientError) -> Self {
        let mut details = Self {
            summary: error.to_string(),
            status_code: None,
            url: None,
            request_id: None,
            messages: Vec::new(),
            raw_body: None,
            timestamp: format!("{:?}", std::time::SystemTime::now()),
            context: HashMap::new(),
        };

        match error {
            splunk_client::ClientError::ApiError {
                status,
                url,
                message,
                request_id,
            } => {
                details.status_code = Some(*status);
                details.url = Some(url.clone());
                details.request_id = request_id.clone();
                details.summary = message.clone();

                // Try to parse SplunkMessages from message if it's JSON
                if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(message)
                    && let Some(msgs) = parsed.get("messages").and_then(|m| m.as_array())
                {
                    details.messages = msgs
                        .iter()
                        .filter_map(|m| serde_json::from_value(m.clone()).ok())
                        .collect();
                }
            }
            splunk_client::ClientError::AuthFailed(msg) => {
                details.summary = msg.clone();
            }
            splunk_client::ClientError::HttpError(e) => {
                details.summary = format!("HTTP error: {}", e);
                if let Some(status) = e.status() {
                    details.status_code = Some(status.as_u16());
                }
                if let Some(url) = e.url() {
                    details.url = Some(url.to_string());
                }
            }
            splunk_client::ClientError::SessionExpired { username } => {
                details.summary =
                    format!("Session expired for user '{username}', please re-authenticate");
            }
            splunk_client::ClientError::InvalidResponse(msg) => {
                details.summary = msg.clone();
            }
            splunk_client::ClientError::Timeout(duration) => {
                details.summary = format!("Request timed out after {:?}", duration);
            }
            splunk_client::ClientError::RateLimited(duration) => {
                details.summary = format!("Rate limited, retry after {:?}", duration);
                details.status_code = Some(429);
            }
            splunk_client::ClientError::ConnectionRefused(addr) => {
                details.summary = format!("Connection refused to {}", addr);
            }
            splunk_client::ClientError::TlsError(msg) => {
                details.summary = format!("TLS error: {}", msg);
            }
            splunk_client::ClientError::MaxRetriesExceeded(count, source) => {
                details.summary =
                    format!("Maximum retries exceeded ({} attempts): {}", count, source);
            }
            splunk_client::ClientError::InvalidUrl(msg) => {
                details.summary = format!("Invalid URL: {}", msg);
            }
            splunk_client::ClientError::NotFound(resource) => {
                details.summary = format!("Resource not found: {}", resource);
                details.status_code = Some(404);
            }
            splunk_client::ClientError::Unauthorized(msg) => {
                details.summary = format!("Unauthorized: {}", msg);
                details.status_code = Some(401);
            }
        }

        details
    }

    /// Create ErrorDetails from error string (for backward compatibility).
    pub fn from_error_string(error_str: &str) -> Self {
        Self {
            summary: error_str.to_string(),
            status_code: None,
            url: None,
            request_id: None,
            messages: Vec::new(),
            raw_body: None,
            timestamp: format!("{:?}", std::time::SystemTime::now()),
            context: HashMap::new(),
        }
    }

    /// Create a brief summary suitable for toast display.
    pub fn to_summary(&self) -> String {
        let max_chars = 50;
        let chars: Vec<char> = self.summary.chars().collect();
        if chars.len() > max_chars {
            let truncated: String = chars.iter().take(max_chars - 3).collect();
            format!("{}...", truncated)
        } else {
            self.summary.clone()
        }
    }

    /// Add context information to error details.
    pub fn add_context(&mut self, key: String, value: String) {
        self.context.insert(key, value);
    }
}

/// Build error details with consistent search context.
///
/// This helper attaches common search-related context (query, operation, sid)
/// to error details for better debugging and user feedback.
///
/// # Arguments
///
/// * `error` - The client error to build details from
/// * `query` - The search query that was being executed
/// * `operation` - The operation that failed (e.g., "create_search_job", "wait_for_job")
/// * `sid` - Optional search job ID
///
/// # Returns
///
/// An `ErrorDetails` struct with the error information and context attached.
pub fn build_search_error_details(
    error: &splunk_client::ClientError,
    query: String,
    operation: String,
    sid: Option<String>,
) -> ErrorDetails {
    let mut details = ErrorDetails::from_client_error(error);
    details.add_context("query".to_string(), query);
    details.add_context("operation".to_string(), operation);
    if let Some(sid) = sid {
        details.add_context("sid".to_string(), sid);
    }
    details
}

/// Get a user-facing error message from a client error.
///
/// This function maps client errors to concise, user-friendly messages
/// suitable for toast notifications and UI display.
///
/// # Arguments
///
/// * `error` - The client error to map
///
/// # Returns
///
/// A string suitable for display to the user.
pub fn search_error_message(error: &splunk_client::ClientError) -> String {
    match error {
        splunk_client::ClientError::Timeout(_) => "Search timeout".to_string(),
        splunk_client::ClientError::AuthFailed(_) => "Authentication failed".to_string(),
        splunk_client::ClientError::SessionExpired { .. } => "Session expired".to_string(),
        splunk_client::ClientError::RateLimited(_) => "Rate limited".to_string(),
        splunk_client::ClientError::ConnectionRefused(_) => "Connection refused".to_string(),
        _ => error.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_details_from_error_string() {
        let details = ErrorDetails::from_error_string("Test error message");
        assert_eq!(details.summary, "Test error message");
        assert!(details.status_code.is_none());
        assert!(details.url.is_none());
        assert!(details.request_id.is_none());
        assert!(details.messages.is_empty());
        assert!(details.raw_body.is_none());
    }

    #[test]
    fn test_to_summary_truncation() {
        let long_msg = "This is a very long error message that should be truncated";
        let details = ErrorDetails::from_error_string(long_msg);
        let summary = details.to_summary();
        assert!(summary.len() <= 50);
        assert!(summary.ends_with("..."));
    }

    #[test]
    fn test_to_summary_no_truncation() {
        let short_msg = "Short error";
        let details = ErrorDetails::from_error_string(short_msg);
        let summary = details.to_summary();
        assert_eq!(summary, "Short error");
        assert!(!summary.ends_with("..."));
    }

    #[test]
    fn test_add_context() {
        let mut details = ErrorDetails::from_error_string("Test error");
        details.add_context("query".to_string(), "index=_internal".to_string());
        details.add_context("sid".to_string(), "123456".to_string());
        assert_eq!(
            details.context.get("query"),
            Some(&"index=_internal".to_string())
        );
        assert_eq!(details.context.get("sid"), Some(&"123456".to_string()));
    }
}
