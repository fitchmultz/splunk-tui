//! Error types for the Splunk client.

use std::error::Error as StdError;
use std::time::Duration;
use thiserror::Error;

/// Result type alias for client operations.
pub type Result<T> = std::result::Result<T, ClientError>;

/// Represents a single failed rollback operation.
#[derive(Debug, Clone)]
pub struct RollbackFailure {
    /// The name of the resource that failed to be cleaned up.
    pub resource_name: String,
    /// The type of operation that failed (e.g., "delete_index", "delete_user").
    pub operation: String,
    /// The error that occurred during rollback.
    pub error: ClientError,
}

impl std::fmt::Display for RollbackFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Failed to {} '{}' during rollback: {}",
            self.operation, self.resource_name, self.error
        )
    }
}

/// Classification of user-facing failure categories.
///
/// This enum provides a stable, high-level categorization of errors that can be
/// presented to users consistently across all UI flows (CLI, TUI).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCategory {
    /// Invalid username, password, or API token.
    AuthInvalidCredentials,
    /// Valid credentials but insufficient permissions for the resource.
    AuthInsufficientPermissions,
    /// Session token has expired and needs refresh.
    SessionExpired,
    /// TLS certificate validation or handshake failure.
    TlsCertificate,
    /// Network connection issues (refused, reset, DNS, etc.).
    Connection,
    /// Request timeout.
    Timeout,
    /// Rate limited by server.
    RateLimited,
    /// Resource not found (404).
    NotFound,
    /// Invalid request parameters (400).
    InvalidRequest,
    /// Validation error on input data.
    Validation,
    /// Server-side error (5xx).
    Server,
    /// Unknown or unclassified error.
    Unknown,
}

/// User-facing failure information with consistent messaging.
///
/// This struct normalizes error presentation across all client consumers,
/// ensuring equivalent failures render consistent titles, diagnosis, and action hints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFacingFailure {
    /// High-level failure category for programmatic handling.
    pub category: FailureCategory,
    /// Concise, user-friendly title for display (e.g., toast notifications).
    pub title: &'static str,
    /// Detailed explanation of what went wrong.
    pub diagnosis: String,
    /// Actionable steps the user can take to resolve the issue.
    pub action_hints: Vec<String>,
    /// HTTP status code if applicable.
    pub status_code: Option<u16>,
    /// Request ID for debugging/tracing.
    pub request_id: Option<String>,
}

/// Errors that can occur during Splunk client operations.
#[derive(Error, Debug)]
pub enum ClientError {
    /// Authentication failed.
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    /// HTTP request error.
    #[error("HTTP error: {0}")]
    HttpError(#[from] reqwest::Error),

    /// API error response from Splunk.
    #[error("API error ({status}) at {url}: {message}{}", .request_id.as_ref().map(|id| format!(" [Request ID: {id}]")).unwrap_or_default())]
    ApiError {
        status: u16,
        url: String,
        message: String,
        request_id: Option<String>,
    },

    /// Session expired and could not be renewed.
    #[error("Session expired for user '{username}', please re-authenticate")]
    SessionExpired { username: String },

    /// Invalid response format from Splunk.
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    /// Request timed out during a specific operation.
    #[error("Operation '{operation}' timed out after {timeout:?}")]
    OperationTimeout {
        /// Name of the operation that timed out (e.g., "fetch_indexes", "list_jobs").
        operation: &'static str,
        /// The configured timeout duration.
        timeout: Duration,
    },

    /// Rate limited - too many requests.
    #[error("Rate limited: retry after {0:?}")]
    RateLimited(Option<Duration>),

    /// Connection refused.
    #[error("Connection refused to {0}")]
    ConnectionRefused(String),

    /// TLS/SSL error.
    #[error("TLS error: {0}")]
    TlsError(String),

    /// Maximum retries exceeded.
    #[error("Maximum retries exceeded ({0} attempts): {1}")]
    MaxRetriesExceeded(usize, Box<ClientError>),

    /// Invalid URL.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Not found.
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Unauthorized access.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    /// Token refresh failed during concurrent singleflight operation.
    /// Contains the original error from the leader's refresh attempt.
    #[error("Token refresh failed for user '{username}' using {auth_method}: {source}")]
    TokenRefreshFailed {
        username: String,
        auth_method: String,
        #[source]
        source: Box<ClientError>,
    },

    /// Invalid request parameters.
    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    /// Validation error.
    #[error("Validation error: {0}")]
    ValidationError(String),

    /// Circuit breaker is open.
    #[error("Circuit breaker open: {0}")]
    CircuitBreakerOpen(String),

    /// Transaction rollback failed for one or more operations.
    #[error("Transaction rollback failed with {count} error(s): {}", failures.iter().map(|f| f.to_string()).collect::<Vec<_>>().join("; "))]
    TransactionRollbackError {
        count: usize,
        failures: Vec<RollbackFailure>,
    },
}

impl Clone for ClientError {
    fn clone(&self) -> Self {
        match self {
            Self::AuthFailed(msg) => Self::AuthFailed(msg.clone()),
            Self::HttpError(e) => Self::ConnectionRefused(format!("HTTP error: {}", e)),
            Self::ApiError {
                status,
                url,
                message,
                request_id,
            } => Self::ApiError {
                status: *status,
                url: url.clone(),
                message: message.clone(),
                request_id: request_id.clone(),
            },
            Self::SessionExpired { username } => Self::SessionExpired {
                username: username.clone(),
            },
            Self::InvalidResponse(msg) => Self::InvalidResponse(msg.clone()),
            Self::OperationTimeout { operation, timeout } => Self::OperationTimeout {
                operation,
                timeout: *timeout,
            },
            Self::RateLimited(d) => Self::RateLimited(*d),
            Self::ConnectionRefused(addr) => Self::ConnectionRefused(addr.clone()),
            Self::TlsError(msg) => Self::TlsError(msg.clone()),
            Self::MaxRetriesExceeded(count, source) => {
                Self::MaxRetriesExceeded(*count, Box::new(source.as_ref().clone()))
            }
            Self::InvalidUrl(url) => Self::InvalidUrl(url.clone()),
            Self::NotFound(resource) => Self::NotFound(resource.clone()),
            Self::Unauthorized(msg) => Self::Unauthorized(msg.clone()),
            Self::InvalidRequest(msg) => Self::InvalidRequest(msg.clone()),
            Self::ValidationError(msg) => Self::ValidationError(msg.clone()),
            Self::CircuitBreakerOpen(endpoint) => Self::CircuitBreakerOpen(endpoint.clone()),
            Self::TransactionRollbackError { count, failures } => Self::TransactionRollbackError {
                count: *count,
                failures: failures.clone(),
            },
            Self::TokenRefreshFailed {
                username,
                auth_method,
                source,
            } => Self::TokenRefreshFailed {
                username: username.clone(),
                auth_method: auth_method.clone(),
                source: Box::new(source.as_ref().clone()),
            },
        }
    }
}

impl ClientError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::HttpError(_) | Self::OperationTimeout { .. } | Self::RateLimited(_)
        )
    }

    /// Check if this error is a circuit breaker error.
    pub fn is_circuit_breaker_error(&self) -> bool {
        matches!(self, Self::CircuitBreakerOpen(_))
    }

    /// Check if an HTTP status code is retryable.
    ///
    /// Retryable status codes:
    /// - 429: Too Many Requests (rate limiting)
    /// - 502: Bad Gateway (transient server error)
    /// - 503: Service Unavailable (transient server error)
    /// - 504: Gateway Timeout (transient server error)
    ///
    /// Non-retryable status codes (fail immediately):
    /// - 400, 401, 403, 404: Client errors
    /// - 500: Internal Server Error (typically indicates a bug, not transient)
    /// - 501: Not Implemented
    pub fn is_retryable_status(status: u16) -> bool {
        matches!(status, 429 | 502 | 503 | 504)
    }

    /// Check if this error indicates authentication failure.
    ///
    /// Includes explicit auth errors as well as ApiError with 401/403 status codes.
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            Self::AuthFailed(_)
                | Self::SessionExpired { .. }
                | Self::Unauthorized(_)
                | Self::TokenRefreshFailed { .. }
        ) || matches!(self, Self::ApiError { status, .. } if *status == 401 || *status == 403)
    }

    /// Convert this error to a user-facing failure with consistent messaging.
    ///
    /// This is the single source of truth for error classification and user guidance
    /// across all UI flows (CLI, TUI).
    pub fn to_user_facing_failure(&self) -> UserFacingFailure {
        match self {
            Self::AuthFailed(msg) => UserFacingFailure {
                category: FailureCategory::AuthInvalidCredentials,
                title: "Authentication failed",
                diagnosis: format!("The provided credentials were rejected: {}", msg),
                action_hints: vec![
                    "Verify your username and password are correct".to_string(),
                    "Check that your API token has not expired".to_string(),
                    "Ensure your account has not been locked or disabled".to_string(),
                ],
                status_code: Some(401),
                request_id: None,
            },

            Self::SessionExpired { username } => UserFacingFailure {
                category: FailureCategory::SessionExpired,
                title: "Session expired",
                diagnosis: format!(
                    "Your session for user '{}' has expired and needs to be refreshed.",
                    username
                ),
                action_hints: vec![
                    "Re-authenticate to establish a new session".to_string(),
                    "Check if your session timeout settings need adjustment".to_string(),
                ],
                status_code: Some(401),
                request_id: None,
            },

            Self::Unauthorized(msg) => UserFacingFailure {
                category: FailureCategory::AuthInsufficientPermissions,
                title: "Access denied",
                diagnosis: format!("The request was unauthorized: {}", msg),
                action_hints: vec![
                    "Verify your authentication credentials are correct".to_string(),
                    "Check that your API token has the required permissions".to_string(),
                    "Ensure your account has access to the requested resource".to_string(),
                ],
                status_code: Some(403),
                request_id: None,
            },

            Self::TokenRefreshFailed {
                username,
                auth_method,
                source,
            } => {
                let source_failure = source.to_user_facing_failure();
                UserFacingFailure {
                    category: source_failure.category,
                    title: "Token refresh failed",
                    diagnosis: format!(
                        "Failed to refresh token for user '{}' ({}): {}",
                        username, auth_method, source_failure.diagnosis
                    ),
                    action_hints: source_failure.action_hints,
                    status_code: source_failure.status_code,
                    request_id: source_failure.request_id,
                }
            }

            Self::TlsError(msg) => {
                let (diagnosis, hints) = if msg.to_lowercase().contains("certificate") {
                    (
                        format!("TLS certificate validation failed: {}", msg),
                        vec![
                            "Verify the Splunk server's TLS certificate is valid".to_string(),
                            "Check system time is correctly synchronized".to_string(),
                            "If using self-signed certificates, ensure they are trusted".to_string(),
                            "Consider setting SPLUNK_SKIP_VERIFY=true for development (not recommended for production)".to_string(),
                        ],
                    )
                } else {
                    (
                        format!("TLS/SSL connection error: {}", msg),
                        vec![
                            "Verify the Splunk server's TLS configuration".to_string(),
                            "Check that your system trusts the server's certificate".to_string(),
                            "Ensure you're using the correct protocol (https vs http)".to_string(),
                        ],
                    )
                };
                UserFacingFailure {
                    category: FailureCategory::TlsCertificate,
                    title: "TLS certificate error",
                    diagnosis,
                    action_hints: hints,
                    status_code: None,
                    request_id: None,
                }
            }

            Self::ConnectionRefused(addr) => UserFacingFailure {
                category: FailureCategory::Connection,
                title: "Connection refused",
                diagnosis: format!("Could not connect to Splunk server at {}", addr),
                action_hints: vec![
                    "Verify the Splunk server is running and accessible".to_string(),
                    "Check that SPLUNK_BASE_URL is configured correctly".to_string(),
                    "Test connectivity with: curl $SPLUNK_BASE_URL".to_string(),
                ],
                status_code: None,
                request_id: None,
            },

            Self::OperationTimeout { operation, timeout } => UserFacingFailure {
                category: FailureCategory::Timeout,
                title: "Request timeout",
                diagnosis: format!("Operation '{}' timed out after {:?}", operation, timeout),
                action_hints: vec![
                    format!("Check if the '{}' endpoint is responding", operation),
                    "Consider increasing SPLUNK_TIMEOUT for slow connections".to_string(),
                    "Verify the Splunk server is not overloaded".to_string(),
                    "Check network connectivity to the Splunk server".to_string(),
                ],
                status_code: None,
                request_id: None,
            },

            Self::RateLimited(retry_after) => UserFacingFailure {
                category: FailureCategory::RateLimited,
                title: "Rate limited",
                diagnosis: format!(
                    "Too many requests. {}",
                    retry_after.map_or("Please wait before retrying.".to_string(), |d| format!(
                        "Retry after {:?}.",
                        d
                    ))
                ),
                action_hints: vec![
                    "Reduce request frequency".to_string(),
                    "Consider increasing SPLUNK_MAX_RETRIES".to_string(),
                    "The client automatically retries with exponential backoff".to_string(),
                ],
                status_code: Some(429),
                request_id: None,
            },

            Self::NotFound(resource) => UserFacingFailure {
                category: FailureCategory::NotFound,
                title: "Resource not found",
                diagnosis: format!("The requested resource was not found: {}", resource),
                action_hints: vec![
                    "Verify the resource name or ID is correct".to_string(),
                    "Check that the resource exists in your Splunk instance".to_string(),
                ],
                status_code: Some(404),
                request_id: None,
            },

            Self::InvalidRequest(msg) => UserFacingFailure {
                category: FailureCategory::InvalidRequest,
                title: "Invalid request",
                diagnosis: msg.clone(),
                action_hints: vec![
                    "Check your request parameters".to_string(),
                    "Verify the syntax of your search query".to_string(),
                ],
                status_code: Some(400),
                request_id: None,
            },

            Self::ValidationError(msg) => UserFacingFailure {
                category: FailureCategory::Validation,
                title: "Validation error",
                diagnosis: msg.clone(),
                action_hints: vec![
                    "Check the input data for errors".to_string(),
                    "Ensure all required fields are provided".to_string(),
                ],
                status_code: Some(400),
                request_id: None,
            },

            Self::ApiError {
                status,
                url,
                message,
                request_id,
            } => {
                let (category, title, hints) = match *status {
                    401 => (
                        FailureCategory::AuthInvalidCredentials,
                        "Authentication required",
                        vec![
                            "Provide valid username and password or API token".to_string(),
                            "Check that your credentials are correctly configured".to_string(),
                        ],
                    ),
                    403 => (
                        FailureCategory::AuthInsufficientPermissions,
                        "Access forbidden",
                        vec![
                            "Verify your account has the required permissions".to_string(),
                            "Contact your Splunk administrator for access".to_string(),
                        ],
                    ),
                    404 => (
                        FailureCategory::NotFound,
                        "Resource not found",
                        vec!["Verify the URL or resource identifier is correct".to_string()],
                    ),
                    400 => (
                        FailureCategory::InvalidRequest,
                        "Invalid request",
                        vec!["Check your request parameters and syntax".to_string()],
                    ),
                    429 => (
                        FailureCategory::RateLimited,
                        "Rate limited",
                        vec!["Wait before retrying or reduce request frequency".to_string()],
                    ),
                    500 => (
                        FailureCategory::Server,
                        "Server error",
                        vec![
                            "The Splunk server encountered an error".to_string(),
                            "Check Splunk server logs for details".to_string(),
                        ],
                    ),
                    502..=504 => (
                        FailureCategory::Server,
                        "Server temporarily unavailable",
                        vec![
                            "The Splunk server is temporarily unavailable".to_string(),
                            "Retry after a short delay".to_string(),
                        ],
                    ),
                    _ => (
                        FailureCategory::Server,
                        "Server error",
                        vec!["Check Splunk server status and logs".to_string()],
                    ),
                };
                UserFacingFailure {
                    category,
                    title,
                    diagnosis: format!("API error at {}: {}", url, message),
                    action_hints: hints,
                    status_code: Some(*status),
                    request_id: request_id.clone(),
                }
            }

            Self::HttpError(e) => {
                // Check for TLS-related errors in HTTP errors
                let error_text = format!(
                    "{} {}",
                    e.to_string().to_lowercase(),
                    e.source()
                        .map(|s| s.to_string().to_lowercase())
                        .unwrap_or_default()
                );

                if error_text.contains("tls")
                    || error_text.contains("ssl")
                    || error_text.contains("certificate")
                    || error_text.contains("cert")
                    || error_text.contains("x509")
                    || error_text.contains("handshake")
                    || error_text.contains("unknown ca")
                {
                    UserFacingFailure {
                        category: FailureCategory::TlsCertificate,
                        title: "TLS certificate error",
                        diagnosis: format!("TLS/SSL connection error: {}", e),
                        action_hints: vec![
                            "Verify the Splunk server's TLS certificate is valid".to_string(),
                            "Check system time is correctly synchronized".to_string(),
                            "If using self-signed certificates, ensure they are trusted"
                                .to_string(),
                            "Consider setting SPLUNK_SKIP_VERIFY=true for development".to_string(),
                        ],
                        status_code: e.status().map(|s| s.as_u16()),
                        request_id: None,
                    }
                } else if e.is_timeout() {
                    UserFacingFailure {
                        category: FailureCategory::Timeout,
                        title: "Request timeout",
                        diagnosis: format!("The request timed out: {}", e),
                        action_hints: vec![
                            "Check network connectivity".to_string(),
                            "Consider increasing SPLUNK_TIMEOUT".to_string(),
                        ],
                        status_code: None,
                        request_id: None,
                    }
                } else if error_text.contains("connection refused")
                    || error_text.contains("connection reset")
                    || error_text.contains("dns")
                    || error_text.contains("no such host")
                {
                    UserFacingFailure {
                        category: FailureCategory::Connection,
                        title: "Connection error",
                        diagnosis: format!("Failed to connect to server: {}", e),
                        action_hints: vec![
                            "Verify the Splunk server is running".to_string(),
                            "Check SPLUNK_BASE_URL is correct".to_string(),
                        ],
                        status_code: None,
                        request_id: None,
                    }
                } else {
                    UserFacingFailure {
                        category: FailureCategory::Unknown,
                        title: "Request failed",
                        diagnosis: format!("HTTP request failed: {}", e),
                        action_hints: vec![
                            "Check network connectivity".to_string(),
                            "Verify SPLUNK_BASE_URL configuration".to_string(),
                        ],
                        status_code: e.status().map(|s| s.as_u16()),
                        request_id: None,
                    }
                }
            }

            Self::InvalidResponse(msg) => UserFacingFailure {
                category: FailureCategory::Server,
                title: "Invalid server response",
                diagnosis: format!("The server returned an unexpected response: {}", msg),
                action_hints: vec![
                    "The Splunk server may be experiencing issues".to_string(),
                    "Check server logs for errors".to_string(),
                ],
                status_code: None,
                request_id: None,
            },

            Self::InvalidUrl(url) => UserFacingFailure {
                category: FailureCategory::InvalidRequest,
                title: "Invalid URL",
                diagnosis: format!("The configured URL is invalid: {}", url),
                action_hints: vec![
                    "Check SPLUNK_BASE_URL format".to_string(),
                    "Ensure the URL includes the scheme (https://)".to_string(),
                ],
                status_code: None,
                request_id: None,
            },

            Self::MaxRetriesExceeded(count, source) => {
                let source_failure = source.to_user_facing_failure();
                UserFacingFailure {
                    category: source_failure.category,
                    title: "Request failed after retries",
                    diagnosis: format!(
                        "Request failed after {} attempts. Original error: {}",
                        count, source_failure.diagnosis
                    ),
                    action_hints: source_failure.action_hints,
                    status_code: source_failure.status_code,
                    request_id: source_failure.request_id,
                }
            }

            Self::CircuitBreakerOpen(endpoint) => UserFacingFailure {
                category: FailureCategory::Server,
                title: "Service temporarily unavailable",
                diagnosis: format!("The circuit breaker is open for endpoint: {}", endpoint),
                action_hints: vec![
                    "Too many recent failures have triggered protection".to_string(),
                    "Wait a moment before retrying".to_string(),
                    "Check the Health screen for server status".to_string(),
                ],
                status_code: None,
                request_id: None,
            },

            Self::TransactionRollbackError { count, failures } => UserFacingFailure {
                category: FailureCategory::Server,
                title: "Transaction rollback failed",
                diagnosis: format!(
                    "Rollback of {} operation(s) failed. Resources may be in an inconsistent state: {}",
                    count,
                    failures
                        .iter()
                        .map(|f| f.resource_name.clone())
                        .collect::<Vec<_>>()
                        .join(", ")
                ),
                action_hints: vec![
                    "Review the failed resources and manually verify their state".to_string(),
                    "Check Splunk server logs for details on cleanup failures".to_string(),
                    "Consider manually removing any orphaned resources".to_string(),
                ],
                status_code: None,
                request_id: None,
            },
        }
    }

    /// Create a ClientError from an HTTP status response with intelligent classification.
    ///
    /// This helper analyzes the status code and message to create the most specific
    /// error variant possible (e.g., SessionExpired instead of generic ApiError).
    pub(crate) fn from_status_response(
        status: u16,
        url: String,
        message: String,
        request_id: Option<String>,
    ) -> Self {
        let lower = message.to_lowercase();

        // Classify 401 responses into specific auth/session errors
        if status == 401 {
            if lower.contains("session expired")
                || lower.contains("invalid session")
                || lower.contains("token expired")
                || lower.contains("session timeout")
            {
                return Self::SessionExpired {
                    username: "unknown".to_string(),
                };
            }
            if lower.contains("invalid credentials")
                || lower.contains("authentication failed")
                || lower.contains("invalid username")
                || lower.contains("invalid password")
                || lower.contains("login failed")
            {
                return Self::AuthFailed(message);
            }
            return Self::Unauthorized(message);
        }

        // Keep 403 as ApiError so CLI can map it to PermissionDenied exit code
        // The CLI's exit code mapping specifically checks for ApiError { status: 403, .. }

        // Classify 404 as NotFound
        if status == 404 {
            return Self::NotFound(url);
        }

        // Classify 400 as InvalidRequest
        if status == 400 {
            return Self::InvalidRequest(message);
        }

        // Default to ApiError for other status codes
        Self::ApiError {
            status,
            url,
            message,
            request_id,
        }
    }

    /// Create a ClientError from a reqwest error with transport-level classification.
    ///
    /// This helper analyzes transport errors to detect TLS, connection, and timeout issues.
    pub(crate) fn from_reqwest_error_classified(error: reqwest::Error) -> Self {
        // Check for timeout first
        if error.is_timeout() {
            return Self::OperationTimeout {
                operation: "http_request",
                timeout: Duration::from_secs(0),
            };
        }

        // Analyze error text for specific patterns
        let text = format!(
            "{} {}",
            error.to_string().to_lowercase(),
            error
                .source()
                .map(|s| s.to_string().to_lowercase())
                .unwrap_or_default()
        );

        // Check for TLS/certificate errors
        if text.contains("tls")
            || text.contains("ssl")
            || text.contains("certificate")
            || text.contains("x509")
            || text.contains("handshake")
            || text.contains("unknown ca")
        {
            return Self::TlsError(error.to_string());
        }

        // Check for connection errors
        if text.contains("connection refused")
            || text.contains("connection reset")
            || text.contains("broken pipe")
            || text.contains("network unreachable")
            || text.contains("no such host")
            || text.contains("dns")
        {
            return Self::ConnectionRefused(error.to_string());
        }

        // Default to wrapping the reqwest error
        Self::HttpError(error)
    }
}

impl From<crate::client::circuit_breaker::CircuitBreakerError> for ClientError {
    fn from(e: crate::client::circuit_breaker::CircuitBreakerError) -> Self {
        match e {
            crate::client::circuit_breaker::CircuitBreakerError::CircuitOpen { endpoint } => {
                ClientError::CircuitBreakerOpen(endpoint)
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_retryable() {
        let err = ClientError::OperationTimeout {
            operation: "test",
            timeout: Duration::from_secs(1),
        };
        assert!(err.is_retryable());

        let err = ClientError::AuthFailed("test".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_is_auth_error() {
        let err = ClientError::AuthFailed("test".to_string());
        assert!(err.is_auth_error());

        let err = ClientError::SessionExpired {
            username: "admin".to_string(),
        };
        assert!(err.is_auth_error());

        let err = ClientError::OperationTimeout {
            operation: "test",
            timeout: Duration::from_secs(1),
        };
        assert!(!err.is_auth_error());
    }

    #[test]
    fn test_is_auth_error_includes_api_error_401() {
        let err = ClientError::ApiError {
            status: 401,
            url: "https://localhost:8089/services".to_string(),
            message: "Unauthorized".to_string(),
            request_id: None,
        };
        assert!(err.is_auth_error());
    }

    #[test]
    fn test_is_auth_error_includes_api_error_403() {
        let err = ClientError::ApiError {
            status: 403,
            url: "https://localhost:8089/services".to_string(),
            message: "Forbidden".to_string(),
            request_id: None,
        };
        assert!(err.is_auth_error());
    }

    #[test]
    fn test_is_auth_error_excludes_other_api_errors() {
        let err = ClientError::ApiError {
            status: 500,
            url: "https://localhost:8089/services".to_string(),
            message: "Server Error".to_string(),
            request_id: None,
        };
        assert!(!err.is_auth_error());
    }

    #[test]
    fn test_is_retryable_status_retryable() {
        // Retryable status codes
        assert!(ClientError::is_retryable_status(429));
        assert!(ClientError::is_retryable_status(502));
        assert!(ClientError::is_retryable_status(503));
        assert!(ClientError::is_retryable_status(504));
    }

    #[test]
    fn test_is_retryable_status_not_retryable() {
        // Client errors (4xx) - should not retry
        assert!(!ClientError::is_retryable_status(400));
        assert!(!ClientError::is_retryable_status(401));
        assert!(!ClientError::is_retryable_status(403));
        assert!(!ClientError::is_retryable_status(404));

        // Server errors (5xx) that are not retryable
        assert!(!ClientError::is_retryable_status(500));
        assert!(!ClientError::is_retryable_status(501));

        // Success codes
        assert!(!ClientError::is_retryable_status(200));
        assert!(!ClientError::is_retryable_status(201));
    }

    #[test]
    fn test_session_expired_error_message() {
        let err = ClientError::SessionExpired {
            username: "admin".to_string(),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("admin"),
            "Error message should contain username"
        );
        assert!(
            msg.contains("Session expired"),
            "Error message should mention session expiry"
        );
    }

    #[test]
    fn test_session_expired_is_auth_error() {
        let err = ClientError::SessionExpired {
            username: "testuser".to_string(),
        };
        assert!(
            err.is_auth_error(),
            "SessionExpired should be an auth error"
        );
    }

    #[test]
    fn test_from_status_response_session_expired() {
        let err = ClientError::from_status_response(
            401,
            "https://localhost:8089".to_string(),
            "Session expired".to_string(),
            None,
        );
        assert!(matches!(err, ClientError::SessionExpired { .. }));
    }

    #[test]
    fn test_from_status_response_invalid_credentials() {
        let err = ClientError::from_status_response(
            401,
            "https://localhost:8089".to_string(),
            "Invalid credentials".to_string(),
            None,
        );
        assert!(matches!(err, ClientError::AuthFailed(_)));
    }

    #[test]
    fn test_from_status_response_authentication_failed() {
        let err = ClientError::from_status_response(
            401,
            "https://localhost:8089".to_string(),
            "Authentication failed for user admin".to_string(),
            None,
        );
        assert!(matches!(err, ClientError::AuthFailed(_)));
    }

    #[test]
    fn test_from_status_response_401_unauthorized() {
        let err = ClientError::from_status_response(
            401,
            "https://localhost:8089".to_string(),
            "Access denied".to_string(),
            None,
        );
        assert!(matches!(err, ClientError::Unauthorized(_)));
    }

    #[test]
    fn test_from_status_response_403_keeps_as_api_error() {
        // 403 is kept as ApiError (not Unauthorized) so CLI can map to PermissionDenied
        let err = ClientError::from_status_response(
            403,
            "https://localhost:8089".to_string(),
            "Forbidden".to_string(),
            Some("req-123".to_string()),
        );
        assert!(
            matches!(err, ClientError::ApiError { status: 403, .. }),
            "403 should remain as ApiError for CLI exit code mapping"
        );
        if let ClientError::ApiError { request_id, .. } = err {
            assert_eq!(request_id, Some("req-123".to_string()));
        }
    }

    #[test]
    fn test_from_status_response_not_found() {
        let err = ClientError::from_status_response(
            404,
            "https://localhost:8089/services/jobs/123".to_string(),
            "Not found".to_string(),
            None,
        );
        assert!(matches!(err, ClientError::NotFound(_)));
    }

    #[test]
    fn test_from_status_response_invalid_request() {
        let err = ClientError::from_status_response(
            400,
            "https://localhost:8089".to_string(),
            "Bad request".to_string(),
            None,
        );
        assert!(matches!(err, ClientError::InvalidRequest(_)));
    }

    #[test]
    fn test_from_status_response_api_error_fallback() {
        let err = ClientError::from_status_response(
            500,
            "https://localhost:8089".to_string(),
            "Internal server error".to_string(),
            Some("req-123".to_string()),
        );
        assert!(matches!(err, ClientError::ApiError { status: 500, .. }));
        if let ClientError::ApiError { request_id, .. } = err {
            assert_eq!(request_id, Some("req-123".to_string()));
        }
    }

    #[test]
    fn test_user_facing_failure_auth_failed() {
        let err = ClientError::AuthFailed("Invalid password".to_string());
        let failure = err.to_user_facing_failure();
        assert_eq!(failure.category, FailureCategory::AuthInvalidCredentials);
        assert_eq!(failure.title, "Authentication failed");
        assert!(failure.diagnosis.contains("Invalid password"));
        assert!(!failure.action_hints.is_empty());
        assert_eq!(failure.status_code, Some(401));
    }

    #[test]
    fn test_user_facing_failure_session_expired() {
        let err = ClientError::SessionExpired {
            username: "admin".to_string(),
        };
        let failure = err.to_user_facing_failure();
        assert_eq!(failure.category, FailureCategory::SessionExpired);
        assert_eq!(failure.title, "Session expired");
        assert!(failure.diagnosis.contains("admin"));
        assert_eq!(failure.status_code, Some(401));
    }

    #[test]
    fn test_user_facing_failure_tls_error() {
        let err = ClientError::TlsError("certificate verify failed".to_string());
        let failure = err.to_user_facing_failure();
        assert_eq!(failure.category, FailureCategory::TlsCertificate);
        assert_eq!(failure.title, "TLS certificate error");
        assert!(failure.diagnosis.contains("certificate"));
        assert!(!failure.action_hints.is_empty());
    }

    #[test]
    fn test_user_facing_failure_api_error_401() {
        let err = ClientError::ApiError {
            status: 401,
            url: "https://localhost:8089".to_string(),
            message: "Unauthorized".to_string(),
            request_id: Some("req-456".to_string()),
        };
        let failure = err.to_user_facing_failure();
        assert_eq!(failure.category, FailureCategory::AuthInvalidCredentials);
        assert_eq!(failure.title, "Authentication required");
        assert_eq!(failure.status_code, Some(401));
        assert_eq!(failure.request_id, Some("req-456".to_string()));
    }

    #[test]
    fn test_user_facing_failure_api_error_403() {
        let err = ClientError::ApiError {
            status: 403,
            url: "https://localhost:8089".to_string(),
            message: "Forbidden".to_string(),
            request_id: None,
        };
        let failure = err.to_user_facing_failure();
        assert_eq!(
            failure.category,
            FailureCategory::AuthInsufficientPermissions
        );
        assert_eq!(failure.title, "Access forbidden");
        assert_eq!(failure.status_code, Some(403));
    }

    #[test]
    fn test_user_facing_failure_connection_refused() {
        let err = ClientError::ConnectionRefused("localhost:8089".to_string());
        let failure = err.to_user_facing_failure();
        assert_eq!(failure.category, FailureCategory::Connection);
        assert_eq!(failure.title, "Connection refused");
        assert!(
            failure
                .action_hints
                .iter()
                .any(|h| h.contains("SPLUNK_BASE_URL"))
        );
    }

    #[test]
    fn test_user_facing_failure_timeout() {
        let err = ClientError::OperationTimeout {
            operation: "fetch_indexes",
            timeout: Duration::from_secs(30),
        };
        let failure = err.to_user_facing_failure();
        assert_eq!(failure.category, FailureCategory::Timeout);
        assert_eq!(failure.title, "Request timeout");
        assert!(failure.diagnosis.contains("fetch_indexes"));
    }

    #[test]
    fn test_user_facing_failure_not_found() {
        let err = ClientError::NotFound("job_123".to_string());
        let failure = err.to_user_facing_failure();
        assert_eq!(failure.category, FailureCategory::NotFound);
        assert_eq!(failure.title, "Resource not found");
        assert_eq!(failure.status_code, Some(404));
    }

    #[test]
    fn test_user_facing_failure_max_retries() {
        let source = ClientError::AuthFailed("test".to_string());
        let err = ClientError::MaxRetriesExceeded(3, Box::new(source));
        let failure = err.to_user_facing_failure();
        assert_eq!(failure.category, FailureCategory::AuthInvalidCredentials);
        assert_eq!(failure.title, "Request failed after retries");
        assert!(failure.diagnosis.contains("3 attempts"));
    }

    #[test]
    fn test_failure_category_equality() {
        assert_eq!(
            FailureCategory::AuthInvalidCredentials,
            FailureCategory::AuthInvalidCredentials
        );
        assert_ne!(
            FailureCategory::AuthInvalidCredentials,
            FailureCategory::SessionExpired
        );
    }

    #[test]
    fn test_rollback_failure_display() {
        let failure = RollbackFailure {
            resource_name: "test_index".to_string(),
            operation: "delete_index".to_string(),
            error: ClientError::NotFound("test_index".to_string()),
        };

        let display = format!("{}", failure);
        assert!(display.contains("delete_index"));
        assert!(display.contains("test_index"));
        assert!(display.contains("rollback"));
    }

    #[test]
    fn test_transaction_rollback_error_display() {
        let failures = vec![
            RollbackFailure {
                resource_name: "index1".to_string(),
                operation: "delete_index".to_string(),
                error: ClientError::OperationTimeout {
                    operation: "delete_index",
                    timeout: Duration::from_secs(30),
                },
            },
            RollbackFailure {
                resource_name: "user1".to_string(),
                operation: "delete_user".to_string(),
                error: ClientError::ConnectionRefused("localhost:8089".to_string()),
            },
        ];

        let error = ClientError::TransactionRollbackError {
            count: failures.len(),
            failures,
        };

        let display = format!("{}", error);
        assert!(display.contains("2 error(s)"));
    }

    #[test]
    fn test_transaction_rollback_error_not_retryable() {
        let error = ClientError::TransactionRollbackError {
            count: 1,
            failures: vec![RollbackFailure {
                resource_name: "test".to_string(),
                operation: "delete_index".to_string(),
                error: ClientError::NotFound("test".to_string()),
            }],
        };

        // Rollback failures are not retryable
        assert!(!error.is_retryable());
    }

    #[test]
    fn test_transaction_rollback_error_not_auth_error() {
        let error = ClientError::TransactionRollbackError {
            count: 1,
            failures: vec![RollbackFailure {
                resource_name: "test".to_string(),
                operation: "delete_index".to_string(),
                error: ClientError::NotFound("test".to_string()),
            }],
        };

        // Rollback failures are not auth errors
        assert!(!error.is_auth_error());
    }

    #[test]
    fn test_user_facing_failure_transaction_rollback_error() {
        let error = ClientError::TransactionRollbackError {
            count: 2,
            failures: vec![
                RollbackFailure {
                    resource_name: "index1".to_string(),
                    operation: "delete_index".to_string(),
                    error: ClientError::OperationTimeout {
                        operation: "delete_index",
                        timeout: Duration::from_secs(30),
                    },
                },
                RollbackFailure {
                    resource_name: "user1".to_string(),
                    operation: "delete_user".to_string(),
                    error: ClientError::ConnectionRefused("localhost:8089".to_string()),
                },
            ],
        };

        let failure = error.to_user_facing_failure();
        assert_eq!(failure.category, FailureCategory::Server);
        assert_eq!(failure.title, "Transaction rollback failed");
        assert!(failure.diagnosis.contains("2"));
        assert!(failure.diagnosis.contains("index1"));
        assert!(failure.diagnosis.contains("user1"));
        assert!(!failure.action_hints.is_empty());
    }

    #[test]
    fn test_token_refresh_failed_is_auth_error() {
        let err = ClientError::TokenRefreshFailed {
            username: "admin".to_string(),
            auth_method: "session token".to_string(),
            source: Box::new(ClientError::AuthFailed("bad password".to_string())),
        };
        assert!(err.is_auth_error());
    }

    #[test]
    fn test_token_refresh_failed_user_facing() {
        let err = ClientError::TokenRefreshFailed {
            username: "admin".to_string(),
            auth_method: "session token".to_string(),
            source: Box::new(ClientError::ConnectionRefused("localhost:8089".to_string())),
        };
        let failure = err.to_user_facing_failure();
        assert_eq!(failure.category, FailureCategory::Connection);
        assert_eq!(failure.title, "Token refresh failed");
        assert!(failure.diagnosis.contains("admin"));
        assert!(failure.diagnosis.contains("session token"));
    }

    #[test]
    fn test_token_refresh_failed_not_retryable() {
        let err = ClientError::TokenRefreshFailed {
            username: "admin".to_string(),
            auth_method: "session token".to_string(),
            source: Box::new(ClientError::AuthFailed("test".to_string())),
        };
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_operation_timeout_includes_operation_name() {
        let err = ClientError::OperationTimeout {
            operation: "fetch_indexes",
            timeout: Duration::from_secs(30),
        };
        let msg = err.to_string();
        assert!(
            msg.contains("fetch_indexes"),
            "Error message should contain operation name: {}",
            msg
        );
        assert!(
            msg.contains("30"),
            "Error message should contain timeout duration: {}",
            msg
        );
    }

    #[test]
    fn test_operation_timeout_is_retryable() {
        let err = ClientError::OperationTimeout {
            operation: "test",
            timeout: Duration::from_secs(1),
        };
        assert!(err.is_retryable(), "OperationTimeout should be retryable");
    }

    #[test]
    fn test_operation_timeout_user_facing_includes_operation() {
        let err = ClientError::OperationTimeout {
            operation: "fetch_jobs",
            timeout: Duration::from_secs(60),
        };
        let failure = err.to_user_facing_failure();
        assert_eq!(failure.category, FailureCategory::Timeout);
        assert!(failure.diagnosis.contains("fetch_jobs"));
        assert!(
            failure
                .action_hints
                .iter()
                .any(|h| h.contains("fetch_jobs"))
        );
    }
}
