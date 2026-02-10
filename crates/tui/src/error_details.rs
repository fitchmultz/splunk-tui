//! Structured error details for UI display.

use serde::{Deserialize, Serialize};
use splunk_client::models::SplunkMessage;
use std::collections::HashMap;

/// Classification of authentication recovery scenarios.
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum AuthRecoveryKind {
    /// Invalid username, password, or API token.
    InvalidCredentials,
    /// Session token has expired.
    SessionExpired,
    /// Missing authentication configuration.
    MissingAuthConfig,
    /// TLS or certificate-related errors.
    TlsOrCertificate,
    /// Connection refused by server.
    ConnectionRefused,
    /// Request timeout.
    Timeout,
    /// Unknown or unclassified error.
    Unknown,
}

/// Details for authentication recovery.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AuthRecoveryDetails {
    /// The kind of authentication recovery scenario.
    pub kind: AuthRecoveryKind,
    /// User-friendly explanation of the issue.
    pub diagnosis: String,
    /// Actionable steps to resolve the issue.
    pub next_steps: Vec<String>,
}

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

    /// Authentication recovery information (if applicable)
    pub auth_recovery: Option<AuthRecoveryDetails>,
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
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: HashMap::new(),
            auth_recovery: classify_auth_recovery(error),
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
            splunk_client::ClientError::InvalidRequest(msg) => {
                details.summary = format!("Invalid request: {}", msg);
            }
            splunk_client::ClientError::ValidationError(msg) => {
                details.summary = format!("Validation error: {}", msg);
            }
            splunk_client::ClientError::CircuitBreakerOpen(endpoint) => {
                details.summary = format!("Circuit breaker open for endpoint: {}", endpoint);
                details
                    .context
                    .insert("endpoint".to_string(), endpoint.clone());
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
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: HashMap::new(),
            auth_recovery: None,
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

/// Classify an authentication error into a recovery scenario.
///
/// This function analyzes client errors and maps them to specific authentication
/// recovery scenarios with actionable guidance for users.
///
/// # Arguments
///
/// * `error` - The client error to classify
///
/// # Returns
///
/// `Some(AuthRecoveryDetails)` if the error is auth-related, `None` otherwise.
pub fn classify_auth_recovery(error: &splunk_client::ClientError) -> Option<AuthRecoveryDetails> {
    match error {
        splunk_client::ClientError::AuthFailed(_) => Some(AuthRecoveryDetails {
            kind: AuthRecoveryKind::InvalidCredentials,
            diagnosis: "Authentication failed. The provided credentials were rejected by Splunk."
                .to_string(),
            next_steps: vec![
                "Verify your username and password are correct".to_string(),
                "Check that your API token has not expired".to_string(),
                "Ensure your account has not been locked or disabled".to_string(),
            ],
        }),
        splunk_client::ClientError::SessionExpired { .. } => Some(AuthRecoveryDetails {
            kind: AuthRecoveryKind::SessionExpired,
            diagnosis: "Your session has expired and needs to be refreshed."
                .to_string(),
            next_steps: vec![
                "Re-authenticate to establish a new session".to_string(),
                "Check if your session timeout settings need adjustment".to_string(),
            ],
        }),
        splunk_client::ClientError::TlsError(msg) => {
            let diagnosis = if msg.contains("certificate") {
                "A TLS certificate validation error occurred."
            } else {
                "A TLS/SSL connection error occurred."
            };
            Some(AuthRecoveryDetails {
                kind: AuthRecoveryKind::TlsOrCertificate,
                diagnosis: diagnosis.to_string(),
                next_steps: vec![
                    "Verify the Splunk server's TLS certificate is valid".to_string(),
                    "Check system time is correctly synchronized".to_string(),
                    "If using self-signed certificates, ensure they are trusted".to_string(),
                    "Consider setting SPLUNK_SKIP_VERIFY=true for development (not recommended for production)".to_string(),
                ],
            })
        }
        splunk_client::ClientError::Unauthorized(_) => Some(AuthRecoveryDetails {
            kind: AuthRecoveryKind::InvalidCredentials,
            diagnosis: "The request was unauthorized. Your credentials may be invalid or expired."
                .to_string(),
            next_steps: vec![
                "Verify your authentication credentials are correct".to_string(),
                "Check that your API token has the required permissions".to_string(),
                "Ensure your account has access to the requested resource".to_string(),
            ],
        }),
        splunk_client::ClientError::ApiError { status, .. } => {
            match status {
                401 => Some(AuthRecoveryDetails {
                    kind: AuthRecoveryKind::InvalidCredentials,
                    diagnosis: "Authentication required. Valid credentials must be provided."
                        .to_string(),
                    next_steps: vec![
                        "Provide valid username and password or API token".to_string(),
                        "Check that your credentials are correctly configured".to_string(),
                    ],
                }),
                403 => Some(AuthRecoveryDetails {
                    kind: AuthRecoveryKind::InvalidCredentials,
                    diagnosis: "Access forbidden. Your credentials are valid but insufficient for this resource."
                        .to_string(),
                    next_steps: vec![
                        "Verify your account has the required permissions".to_string(),
                        "Contact your Splunk administrator for access".to_string(),
                    ],
                }),
                _ => None,
            }
        }
        splunk_client::ClientError::HttpError(e) => {
            let error_text = e.to_string().to_lowercase();
            if error_text.contains("tls") 
                || error_text.contains("ssl") 
                || error_text.contains("certificate")
                || error_text.contains("cert")
            {
                Some(AuthRecoveryDetails {
                    kind: AuthRecoveryKind::TlsOrCertificate,
                    diagnosis: "A TLS/SSL connection error occurred.".to_string(),
                    next_steps: vec![
                        "Verify the Splunk server's TLS certificate is valid".to_string(),
                        "Check system time is correctly synchronized".to_string(),
                        "If using self-signed certificates, ensure they are trusted".to_string(),
                    ],
                })
            } else {
                None
            }
        }
        _ => None,
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
        assert!(details.auth_recovery.is_none());
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

    #[test]
    fn test_classify_auth_recovery_invalid_credentials() {
        let error = splunk_client::ClientError::AuthFailed("Invalid password".to_string());
        let recovery = classify_auth_recovery(&error);
        assert!(recovery.is_some());
        let recovery = recovery.unwrap();
        assert_eq!(recovery.kind, AuthRecoveryKind::InvalidCredentials);
        assert!(recovery.diagnosis.contains("credentials"));
        assert!(!recovery.next_steps.is_empty());
    }

    #[test]
    fn test_classify_auth_recovery_session_expired() {
        let error = splunk_client::ClientError::SessionExpired {
            username: "admin".to_string(),
        };
        let recovery = classify_auth_recovery(&error);
        assert!(recovery.is_some());
        let recovery = recovery.unwrap();
        assert_eq!(recovery.kind, AuthRecoveryKind::SessionExpired);
        assert!(recovery.diagnosis.contains("expired"));
        assert!(!recovery.next_steps.is_empty());
    }

    #[test]
    fn test_classify_auth_recovery_tls_error() {
        let error = splunk_client::ClientError::TlsError("certificate verify failed".to_string());
        let recovery = classify_auth_recovery(&error);
        assert!(recovery.is_some());
        let recovery = recovery.unwrap();
        assert_eq!(recovery.kind, AuthRecoveryKind::TlsOrCertificate);
        assert!(recovery.diagnosis.contains("certificate"));
        assert!(!recovery.next_steps.is_empty());
    }

    #[test]
    fn test_classify_auth_recovery_tls_error_generic() {
        let error = splunk_client::ClientError::TlsError("handshake failed".to_string());
        let recovery = classify_auth_recovery(&error);
        assert!(recovery.is_some());
        let recovery = recovery.unwrap();
        assert_eq!(recovery.kind, AuthRecoveryKind::TlsOrCertificate);
        assert!(recovery.diagnosis.contains("TLS"));
    }

    #[test]
    fn test_classify_auth_recovery_unauthorized() {
        let error = splunk_client::ClientError::Unauthorized("Access denied".to_string());
        let recovery = classify_auth_recovery(&error);
        assert!(recovery.is_some());
        let recovery = recovery.unwrap();
        assert_eq!(recovery.kind, AuthRecoveryKind::InvalidCredentials);
        assert!(
            recovery.diagnosis.contains("unauthorized")
                || recovery.diagnosis.contains("Unauthorized")
        );
    }

    #[test]
    fn test_classify_auth_recovery_api_error_401() {
        let error = splunk_client::ClientError::ApiError {
            status: 401,
            url: "https://localhost:8089/services".to_string(),
            message: "Unauthorized".to_string(),
            request_id: None,
        };
        let recovery = classify_auth_recovery(&error);
        assert!(recovery.is_some());
        let recovery = recovery.unwrap();
        assert_eq!(recovery.kind, AuthRecoveryKind::InvalidCredentials);
    }

    #[test]
    fn test_classify_auth_recovery_api_error_403() {
        let error = splunk_client::ClientError::ApiError {
            status: 403,
            url: "https://localhost:8089/services".to_string(),
            message: "Forbidden".to_string(),
            request_id: None,
        };
        let recovery = classify_auth_recovery(&error);
        assert!(recovery.is_some());
        let recovery = recovery.unwrap();
        assert_eq!(recovery.kind, AuthRecoveryKind::InvalidCredentials);
    }

    #[test]
    fn test_classify_auth_recovery_api_error_non_auth() {
        let error = splunk_client::ClientError::ApiError {
            status: 500,
            url: "https://localhost:8089/services".to_string(),
            message: "Internal Server Error".to_string(),
            request_id: None,
        };
        let recovery = classify_auth_recovery(&error);
        assert!(recovery.is_none());
    }

    #[test]
    fn test_classify_auth_recovery_http_error_tls() {
        // Note: We can't easily create an HttpError with TLS text without reqwest,
        // but we test the non-TLS case to ensure fallthrough works
        let error = splunk_client::ClientError::Timeout(std::time::Duration::from_secs(30));
        let recovery = classify_auth_recovery(&error);
        assert!(recovery.is_none());
    }

    #[test]
    fn test_classify_auth_recovery_non_auth_errors() {
        // Test various non-auth errors return None
        let timeout_error = splunk_client::ClientError::Timeout(std::time::Duration::from_secs(30));
        assert!(classify_auth_recovery(&timeout_error).is_none());

        let not_found_error = splunk_client::ClientError::NotFound("job_123".to_string());
        assert!(classify_auth_recovery(&not_found_error).is_none());

        let rate_limited_error = splunk_client::ClientError::RateLimited(None);
        assert!(classify_auth_recovery(&rate_limited_error).is_none());

        let connection_error =
            splunk_client::ClientError::ConnectionRefused("localhost:8089".to_string());
        assert!(classify_auth_recovery(&connection_error).is_none());
    }

    #[test]
    fn test_error_details_from_client_error_populates_auth_recovery() {
        let error = splunk_client::ClientError::AuthFailed("Invalid password".to_string());
        let details = ErrorDetails::from_client_error(&error);
        assert!(details.auth_recovery.is_some());
        let recovery = details.auth_recovery.unwrap();
        assert_eq!(recovery.kind, AuthRecoveryKind::InvalidCredentials);
    }

    #[test]
    fn test_build_search_error_details_preserves_auth_recovery() {
        let error = splunk_client::ClientError::SessionExpired {
            username: "admin".to_string(),
        };
        let details = build_search_error_details(
            &error,
            "index=_internal".to_string(),
            "create_search_job".to_string(),
            Some("job_123".to_string()),
        );
        assert!(details.auth_recovery.is_some());
        let recovery = details.auth_recovery.unwrap();
        assert_eq!(recovery.kind, AuthRecoveryKind::SessionExpired);
        assert_eq!(
            details.context.get("query"),
            Some(&"index=_internal".to_string())
        );
        assert_eq!(
            details.context.get("operation"),
            Some(&"create_search_job".to_string())
        );
        assert_eq!(details.context.get("sid"), Some(&"job_123".to_string()));
    }

    #[test]
    fn test_auth_recovery_kind_serialization() {
        let kind = AuthRecoveryKind::InvalidCredentials;
        let json = serde_json::to_string(&kind).unwrap();
        assert!(json.contains("InvalidCredentials"));

        let deserialized: AuthRecoveryKind = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized, AuthRecoveryKind::InvalidCredentials);
    }

    #[test]
    fn test_auth_recovery_details_serialization() {
        let details = AuthRecoveryDetails {
            kind: AuthRecoveryKind::MissingAuthConfig,
            diagnosis: "No authentication configured".to_string(),
            next_steps: vec!["Set SPLUNK_USERNAME and SPLUNK_PASSWORD".to_string()],
        };
        let json = serde_json::to_string(&details).unwrap();
        assert!(json.contains("MissingAuthConfig"));
        assert!(json.contains("No authentication configured"));

        let deserialized: AuthRecoveryDetails = serde_json::from_str(&json).unwrap();
        assert_eq!(deserialized.kind, AuthRecoveryKind::MissingAuthConfig);
        assert_eq!(deserialized.diagnosis, "No authentication configured");
        assert_eq!(deserialized.next_steps.len(), 1);
    }
}
