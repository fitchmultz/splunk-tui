//! Error types for the Splunk client.

use std::time::Duration;
use thiserror::Error;

/// Result type alias for client operations.
pub type Result<T> = std::result::Result<T, ClientError>;

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
    #[error("Session expired, please re-authenticate")]
    SessionExpired,

    /// Invalid response format from Splunk.
    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    /// Request timed out.
    #[error("Request timed out after {0:?}")]
    Timeout(Duration),

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
    #[error("Maximum retries exceeded ({0} attempts)")]
    MaxRetriesExceeded(usize),

    /// Invalid URL.
    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    /// Not found.
    #[error("Resource not found: {0}")]
    NotFound(String),

    /// Unauthorized access.
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
}

impl ClientError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::HttpError(_) | Self::Timeout(_) | Self::RateLimited(_)
        )
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
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            Self::AuthFailed(_) | Self::SessionExpired | Self::Unauthorized(_)
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_is_retryable() {
        let err = ClientError::Timeout(Duration::from_secs(1));
        assert!(err.is_retryable());

        let err = ClientError::AuthFailed("test".to_string());
        assert!(!err.is_retryable());
    }

    #[test]
    fn test_error_is_auth_error() {
        let err = ClientError::AuthFailed("test".to_string());
        assert!(err.is_auth_error());

        let err = ClientError::SessionExpired;
        assert!(err.is_auth_error());

        let err = ClientError::Timeout(Duration::from_secs(1));
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
}
