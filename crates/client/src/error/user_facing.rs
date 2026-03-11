//! Purpose: Render shared client errors into stable user-facing failure messages.
//! Responsibilities: Map low-level client errors into high-level categories, titles, diagnoses, and actionable hints.
//! Scope: Presentation-neutral shared messaging only; CLI exit codes and TUI popup policy live outside this module.
//! Usage: Called by both CLI and TUI when displaying actionable failure details.
//! Invariants/Assumptions: Equivalent underlying failures produce equivalent user-facing guidance across frontends.

use super::kinds::{ClientError, FailureCategory, UserFacingFailure};

impl ClientError {
    /// Convert this error to a user-facing failure with consistent messaging.
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
                    retry_after.map_or("Please wait before retrying.".to_string(), |delay| {
                        format!("Retry after {:?}.", delay)
                    })
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
            Self::HttpError(error) => {
                if error.is_timeout() {
                    UserFacingFailure {
                        category: FailureCategory::Timeout,
                        title: "Request timeout",
                        diagnosis: format!("The request timed out: {}", error),
                        action_hints: vec![
                            "Check network connectivity".to_string(),
                            "Consider increasing SPLUNK_TIMEOUT".to_string(),
                        ],
                        status_code: error.status(),
                        request_id: None,
                    }
                } else if error.is_connect() {
                    UserFacingFailure {
                        category: FailureCategory::Connection,
                        title: "Connection error",
                        diagnosis: format!("Failed to connect to server: {}", error),
                        action_hints: vec![
                            "Verify the Splunk server is running".to_string(),
                            "Check SPLUNK_BASE_URL is correct".to_string(),
                        ],
                        status_code: error.status(),
                        request_id: None,
                    }
                } else {
                    let error_text = error.classification_text();
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
                            diagnosis: format!("TLS/SSL connection error: {}", error),
                            action_hints: vec![
                                "Verify the Splunk server's TLS certificate is valid".to_string(),
                                "Check system time is correctly synchronized".to_string(),
                                "If using self-signed certificates, ensure they are trusted"
                                    .to_string(),
                                "Consider setting SPLUNK_SKIP_VERIFY=true for development"
                                    .to_string(),
                            ],
                            status_code: error.status(),
                            request_id: None,
                        }
                    } else {
                        UserFacingFailure {
                            category: FailureCategory::Unknown,
                            title: "Request failed",
                            diagnosis: format!("HTTP request failed: {}", error),
                            action_hints: vec![
                                "Check network connectivity".to_string(),
                                "Verify SPLUNK_BASE_URL configuration".to_string(),
                            ],
                            status_code: error.status(),
                            request_id: None,
                        }
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
                        .map(|failure| failure.resource_name.clone())
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
}
