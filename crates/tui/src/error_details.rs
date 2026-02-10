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

impl From<splunk_client::FailureCategory> for AuthRecoveryKind {
    /// Map client failure categories to TUI auth recovery kinds.
    fn from(category: splunk_client::FailureCategory) -> Self {
        match category {
            splunk_client::FailureCategory::AuthInvalidCredentials => {
                AuthRecoveryKind::InvalidCredentials
            }
            splunk_client::FailureCategory::AuthInsufficientPermissions => {
                AuthRecoveryKind::InvalidCredentials
            }
            splunk_client::FailureCategory::SessionExpired => AuthRecoveryKind::SessionExpired,
            splunk_client::FailureCategory::TlsCertificate => AuthRecoveryKind::TlsOrCertificate,
            splunk_client::FailureCategory::Connection => AuthRecoveryKind::ConnectionRefused,
            splunk_client::FailureCategory::Timeout => AuthRecoveryKind::Timeout,
            _ => AuthRecoveryKind::Unknown,
        }
    }
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

impl From<&splunk_client::UserFacingFailure> for AuthRecoveryDetails {
    /// Convert a client user-facing failure to TUI auth recovery details.
    fn from(failure: &splunk_client::UserFacingFailure) -> Self {
        Self {
            kind: failure.category.into(),
            diagnosis: failure.diagnosis.clone(),
            next_steps: failure.action_hints.clone(),
        }
    }
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
    ///
    /// This method uses the shared `UserFacingFailure` classifier from the client
    /// to ensure consistent error messaging across all TUI flows.
    pub fn from_client_error(error: &splunk_client::ClientError) -> Self {
        // Get the unified user-facing failure classification
        let failure = error.to_user_facing_failure();

        let mut details = Self {
            summary: failure.title.to_string(),
            status_code: failure.status_code,
            url: None,
            request_id: failure.request_id.clone(),
            messages: Vec::new(),
            raw_body: None,
            timestamp: chrono::Utc::now().to_rfc3339(),
            context: HashMap::new(),
            auth_recovery: Self::should_show_auth_recovery(&failure).then(|| (&failure).into()),
        };

        // Extract additional metadata based on specific error variants
        match error {
            splunk_client::ClientError::ApiError {
                status: _,
                url,
                message,
                request_id,
            } => {
                details.url = Some(url.clone());
                details.request_id = request_id.clone();
                // For API errors, use the detailed message as summary
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
            splunk_client::ClientError::HttpError(e) => {
                if let Some(url) = e.url() {
                    details.url = Some(url.to_string());
                }
            }
            splunk_client::ClientError::ConnectionRefused(addr) => {
                details.url = Some(addr.clone());
            }
            splunk_client::ClientError::NotFound(resource) => {
                details.url = Some(resource.clone());
            }
            _ => {}
        }

        details
    }

    /// Determine if auth recovery should be shown for this failure.
    fn should_show_auth_recovery(failure: &splunk_client::UserFacingFailure) -> bool {
        matches!(
            failure.category,
            splunk_client::FailureCategory::AuthInvalidCredentials
                | splunk_client::FailureCategory::AuthInsufficientPermissions
                | splunk_client::FailureCategory::SessionExpired
                | splunk_client::FailureCategory::TlsCertificate
                | splunk_client::FailureCategory::Connection
                | splunk_client::FailureCategory::Timeout
        )
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
/// **Deprecated**: This function is kept for backward compatibility.
/// New code should use `ErrorDetails::from_client_error()` which uses the shared
/// `UserFacingFailure` classifier from the client crate.
///
/// # Arguments
///
/// * `error` - The client error to classify
///
/// # Returns
///
/// `Some(AuthRecoveryDetails)` if the error is auth-related, `None` otherwise.
pub fn classify_auth_recovery(error: &splunk_client::ClientError) -> Option<AuthRecoveryDetails> {
    // Use the shared classifier and convert to TUI-specific format
    let failure = error.to_user_facing_failure();
    if ErrorDetails::should_show_auth_recovery(&failure) {
        Some((&failure).into())
    } else {
        None
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
/// Uses the shared `UserFacingFailure` classifier for consistent messaging
/// across all TUI flows.
///
/// # Arguments
///
/// * `error` - The client error to map
///
/// # Returns
///
/// A string suitable for display to the user.
pub fn search_error_message(error: &splunk_client::ClientError) -> String {
    error.to_user_facing_failure().title.to_string()
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
        assert!(recovery.diagnosis.contains("Invalid password"));
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
        assert!(recovery.diagnosis.contains("admin"));
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
        // Server errors don't show auth recovery
        assert!(recovery.is_none());
    }

    #[test]
    fn test_classify_auth_recovery_non_auth_errors() {
        // Test various non-auth errors return None
        // Note: Timeout and Connection errors now DO have auth recovery (for unified UX)

        let not_found_error = splunk_client::ClientError::NotFound("job_123".to_string());
        assert!(classify_auth_recovery(&not_found_error).is_none());

        let rate_limited_error = splunk_client::ClientError::RateLimited(None);
        assert!(classify_auth_recovery(&rate_limited_error).is_none());
    }

    #[test]
    fn test_classify_auth_recovery_timeout() {
        // Timeout errors now have auth recovery (for unified UX)
        let timeout_error = splunk_client::ClientError::Timeout(std::time::Duration::from_secs(30));
        let recovery = classify_auth_recovery(&timeout_error);
        assert!(recovery.is_some());
        let recovery = recovery.unwrap();
        assert_eq!(recovery.kind, AuthRecoveryKind::Timeout);
    }

    #[test]
    fn test_classify_auth_recovery_connection_refused() {
        let connection_error =
            splunk_client::ClientError::ConnectionRefused("localhost:8089".to_string());
        let recovery = classify_auth_recovery(&connection_error);
        assert!(recovery.is_some());
        let recovery = recovery.unwrap();
        assert_eq!(recovery.kind, AuthRecoveryKind::ConnectionRefused);
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
    fn test_error_details_from_client_error_uses_shared_classifier() {
        let error = splunk_client::ClientError::SessionExpired {
            username: "admin".to_string(),
        };
        let details = ErrorDetails::from_client_error(&error);
        // Should use the shared classifier's title
        assert_eq!(details.summary, "Session expired");
        assert_eq!(details.status_code, Some(401));
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

    #[test]
    fn test_search_error_message_uses_classifier() {
        let error = splunk_client::ClientError::AuthFailed("test".to_string());
        let msg = search_error_message(&error);
        assert_eq!(msg, "Authentication failed");

        let error = splunk_client::ClientError::SessionExpired {
            username: "admin".to_string(),
        };
        let msg = search_error_message(&error);
        assert_eq!(msg, "Session expired");

        let error = splunk_client::ClientError::Timeout(std::time::Duration::from_secs(30));
        let msg = search_error_message(&error);
        assert_eq!(msg, "Request timeout");
    }

    #[test]
    fn test_failure_category_to_auth_recovery_kind_mapping() {
        assert_eq!(
            AuthRecoveryKind::from(splunk_client::FailureCategory::AuthInvalidCredentials),
            AuthRecoveryKind::InvalidCredentials
        );
        assert_eq!(
            AuthRecoveryKind::from(splunk_client::FailureCategory::SessionExpired),
            AuthRecoveryKind::SessionExpired
        );
        assert_eq!(
            AuthRecoveryKind::from(splunk_client::FailureCategory::TlsCertificate),
            AuthRecoveryKind::TlsOrCertificate
        );
        assert_eq!(
            AuthRecoveryKind::from(splunk_client::FailureCategory::Connection),
            AuthRecoveryKind::ConnectionRefused
        );
        assert_eq!(
            AuthRecoveryKind::from(splunk_client::FailureCategory::Timeout),
            AuthRecoveryKind::Timeout
        );
        assert_eq!(
            AuthRecoveryKind::from(splunk_client::FailureCategory::Unknown),
            AuthRecoveryKind::Unknown
        );
    }
}
