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
}
