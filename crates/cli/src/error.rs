//! CLI exit codes for scripting and automation.
//!
//! Responsibilities:
//! - Define structured exit codes that scripts can use to distinguish error types.
//! - Map ClientError variants to appropriate exit codes.
//!
//! Does NOT handle:
//! - Error message formatting (handled by anyhow Display).
//! - Signal handling (see cancellation.rs for SIGINT handling).
//!
//! Invariants:
//! - Exit codes 1-9 are reserved for specific error categories.
//! - Exit code 130 is reserved for SIGINT (Unix standard: 128 + SIGINT).

use splunk_client::ClientError;

/// Structured exit codes for splunk-cli.
///
/// These codes enable scripts to distinguish between different failure modes
/// and take appropriate action (retry, refresh credentials, fail fast, etc.).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum ExitCode {
    /// Success - command completed successfully.
    Success = 0,

    /// General error - unhandled or generic failure.
    GeneralError = 1,

    /// Authentication failure - invalid credentials or expired session.
    ///
    /// Scripts should refresh credentials or prompt for re-authentication.
    AuthenticationFailed = 2,

    /// Connection error - network, timeout, or DNS failure.
    ///
    /// Scripts may retry with exponential backoff.
    ConnectionError = 3,

    /// Resource not found - job, index, saved search, etc.
    ///
    /// Scripts should verify resource identifiers or create missing resources.
    NotFound = 4,

    /// Validation error - invalid SPL, bad parameters.
    ///
    /// Scripts should fix the input and not retry the same request.
    ValidationError = 5,

    /// Permission denied - insufficient privileges.
    ///
    /// Scripts should escalate permissions or use different credentials.
    PermissionDenied = 6,

    /// Rate limited - HTTP 429 Too Many Requests.
    ///
    /// Scripts should back off and retry later.
    RateLimited = 7,

    /// Service unavailable - HTTP 503, maintenance mode.
    ///
    /// Scripts should back off and retry later.
    ServiceUnavailable = 8,

    /// Interrupted - SIGINT/Ctrl+C (Unix standard: 128 + 2).
    ///
    /// This matches the Unix convention where exit code 128 + signal number
    /// indicates termination by that signal.
    #[allow(dead_code)]
    Interrupted = 130,
}

impl ExitCode {
    /// Convert the exit code to an i32 for use with std::process::exit().
    pub const fn as_i32(self) -> i32 {
        self as u8 as i32
    }

    /// Returns true if this exit code indicates a retryable condition.
    ///
    /// Retryable conditions include:
    /// - Connection errors (temporary network issues)
    /// - Rate limiting (should retry after delay)
    /// - Service unavailable (maintenance mode may resolve)
    #[allow(dead_code)]
    pub const fn is_retryable(self) -> bool {
        matches!(
            self,
            ExitCode::ConnectionError | ExitCode::RateLimited | ExitCode::ServiceUnavailable
        )
    }
}

impl From<&ClientError> for ExitCode {
    /// Map ClientError variants to structured exit codes.
    ///
    /// This mapping is the core of the structured exit code feature.
    /// Each ClientError variant is categorized based on how scripts should respond.
    fn from(err: &ClientError) -> Self {
        match err {
            // Authentication errors (exit code 2)
            ClientError::AuthFailed(_) => ExitCode::AuthenticationFailed,
            ClientError::SessionExpired { .. } => ExitCode::AuthenticationFailed,
            ClientError::Unauthorized(_) => ExitCode::AuthenticationFailed,

            // Connection errors (exit code 3)
            ClientError::ConnectionRefused(_) => ExitCode::ConnectionError,
            ClientError::Timeout(_) => ExitCode::ConnectionError,
            ClientError::InvalidUrl(_) => ExitCode::ConnectionError,
            ClientError::TlsError(_) => ExitCode::ConnectionError,

            // Not found (exit code 4)
            ClientError::NotFound(_) => ExitCode::NotFound,
            ClientError::ApiError { status: 404, .. } => ExitCode::NotFound,

            // Validation errors (exit code 5)
            ClientError::InvalidRequest(_) => ExitCode::ValidationError,
            ClientError::ValidationError(_) => ExitCode::ValidationError,
            ClientError::InvalidResponse(_) => ExitCode::ValidationError,
            ClientError::ApiError { status: 400, .. } => ExitCode::ValidationError,

            // Authentication errors (exit code 2) - HTTP status codes
            ClientError::ApiError { status: 401, .. } => ExitCode::AuthenticationFailed,

            // Permission denied (exit code 6)
            ClientError::ApiError { status: 403, .. } => ExitCode::PermissionDenied,

            // Rate limited (exit code 7)
            ClientError::RateLimited(_) => ExitCode::RateLimited,
            // Also handle HTTP 429 from ApiError (when retries are exhausted)
            ClientError::ApiError { status: 429, .. } => ExitCode::RateLimited,

            // Service unavailable (exit code 8)
            ClientError::ApiError { status: 503, .. } => ExitCode::ServiceUnavailable,
            ClientError::ApiError { status: 502, .. } => ExitCode::ServiceUnavailable,
            ClientError::ApiError { status: 504, .. } => ExitCode::ServiceUnavailable,
            ClientError::CircuitBreakerOpen(_) => ExitCode::ServiceUnavailable,

            // Max retries exceeded - check the underlying error recursively
            ClientError::MaxRetriesExceeded(_, inner) => Self::from(inner.as_ref()),

            // HttpError - check if it's a connection/timeout error
            ClientError::HttpError(e) => {
                if e.is_connect() || e.is_timeout() {
                    ExitCode::ConnectionError
                } else {
                    ExitCode::GeneralError
                }
            }

            // Default: general error
            ClientError::ApiError { .. } => ExitCode::GeneralError,

            // Transaction rollback error - data integrity issue (exit code 1)
            ClientError::TransactionRollbackError { .. } => ExitCode::GeneralError,
        }
    }
}

/// Extension trait for anyhow::Error to extract exit codes.
///
/// This trait provides a convenient way to get the appropriate exit code
/// from any anyhow error, handling both ClientError and other error types.
pub trait ExitCodeExt {
    /// Extract the appropriate exit code from this error.
    ///
    /// Returns ExitCode::GeneralError if the error is not a ClientError.
    fn exit_code(&self) -> ExitCode;
}

impl ExitCodeExt for anyhow::Error {
    fn exit_code(&self) -> ExitCode {
        // Try to downcast to ClientError
        if let Some(client_err) = self.downcast_ref::<ClientError>() {
            return ExitCode::from(client_err);
        }

        // Try to find ClientError in the chain
        for cause in self.chain() {
            if let Some(client_err) = cause.downcast_ref::<ClientError>() {
                return ExitCode::from(client_err);
            }
        }

        // Default to general error
        ExitCode::GeneralError
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    #[test]
    fn test_exit_code_as_i32() {
        assert_eq!(ExitCode::Success.as_i32(), 0);
        assert_eq!(ExitCode::GeneralError.as_i32(), 1);
        assert_eq!(ExitCode::AuthenticationFailed.as_i32(), 2);
        assert_eq!(ExitCode::Interrupted.as_i32(), 130);
    }

    #[test]
    fn test_is_retryable() {
        assert!(!ExitCode::Success.is_retryable());
        assert!(!ExitCode::GeneralError.is_retryable());
        assert!(!ExitCode::AuthenticationFailed.is_retryable());
        assert!(ExitCode::ConnectionError.is_retryable());
        assert!(!ExitCode::NotFound.is_retryable());
        assert!(!ExitCode::ValidationError.is_retryable());
        assert!(!ExitCode::PermissionDenied.is_retryable());
        assert!(ExitCode::RateLimited.is_retryable());
        assert!(ExitCode::ServiceUnavailable.is_retryable());
    }

    #[test]
    fn test_from_client_error_auth_failed() {
        let err = ClientError::AuthFailed("invalid credentials".to_string());
        assert_eq!(ExitCode::from(&err), ExitCode::AuthenticationFailed);
    }

    #[test]
    fn test_from_client_error_session_expired() {
        let err = ClientError::SessionExpired {
            username: "admin".to_string(),
        };
        assert_eq!(ExitCode::from(&err), ExitCode::AuthenticationFailed);
    }

    #[test]
    fn test_from_client_error_unauthorized() {
        let err = ClientError::Unauthorized("access denied".to_string());
        assert_eq!(ExitCode::from(&err), ExitCode::AuthenticationFailed);
    }

    #[test]
    fn test_from_client_error_connection_refused() {
        let err = ClientError::ConnectionRefused("localhost:8089".to_string());
        assert_eq!(ExitCode::from(&err), ExitCode::ConnectionError);
    }

    #[test]
    fn test_from_client_error_timeout() {
        let err = ClientError::Timeout(Duration::from_secs(30));
        assert_eq!(ExitCode::from(&err), ExitCode::ConnectionError);
    }

    #[test]
    fn test_from_client_error_not_found() {
        let err = ClientError::NotFound("job 123".to_string());
        assert_eq!(ExitCode::from(&err), ExitCode::NotFound);
    }

    #[test]
    fn test_from_client_error_invalid_request() {
        let err = ClientError::InvalidRequest("bad parameter".to_string());
        assert_eq!(ExitCode::from(&err), ExitCode::ValidationError);
    }

    #[test]
    fn test_from_client_error_api_error_403() {
        let err = ClientError::ApiError {
            status: 403,
            url: "https://localhost:8089".to_string(),
            message: "Forbidden".to_string(),
            request_id: None,
        };
        assert_eq!(ExitCode::from(&err), ExitCode::PermissionDenied);
    }

    #[test]
    fn test_from_client_error_api_error_429() {
        let err = ClientError::ApiError {
            status: 429,
            url: "https://localhost:8089".to_string(),
            message: "Too Many Requests".to_string(),
            request_id: None,
        };
        assert_eq!(ExitCode::from(&err), ExitCode::RateLimited);
    }

    #[test]
    fn test_from_client_error_api_error_503() {
        let err = ClientError::ApiError {
            status: 503,
            url: "https://localhost:8089".to_string(),
            message: "Service Unavailable".to_string(),
            request_id: None,
        };
        assert_eq!(ExitCode::from(&err), ExitCode::ServiceUnavailable);
    }

    #[test]
    fn test_from_client_error_rate_limited() {
        let err = ClientError::RateLimited(Some(Duration::from_secs(60)));
        assert_eq!(ExitCode::from(&err), ExitCode::RateLimited);
    }

    #[test]
    fn test_from_client_error_max_retries_exceeded() {
        let inner = ClientError::ConnectionRefused("localhost:8089".to_string());
        let err = ClientError::MaxRetriesExceeded(3, Box::new(inner));
        assert_eq!(ExitCode::from(&err), ExitCode::ConnectionError);
    }

    #[test]
    fn test_from_client_error_max_retries_exceeded_nested() {
        let inner = ClientError::NotFound("test".to_string());
        let middle = ClientError::MaxRetriesExceeded(3, Box::new(inner));
        let outer = ClientError::MaxRetriesExceeded(3, Box::new(middle));
        assert_eq!(ExitCode::from(&outer), ExitCode::NotFound);
    }
}
