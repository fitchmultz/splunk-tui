//! Purpose: Define the stable error kinds and shared data structures for client failures.
//! Responsibilities: Hold error enums/structs, clone-safe HTTP error snapshots, and cross-cutting conversions.
//! Scope: Type definitions only; classification and user-facing rendering live in sibling modules.
//! Usage: Re-exported through `splunk_client::error`.
//! Invariants/Assumptions: HTTP transport failures must remain clone-safe and never degrade into a different semantic error when cloned.

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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FailureCategory {
    AuthInvalidCredentials,
    AuthInsufficientPermissions,
    SessionExpired,
    TlsCertificate,
    Connection,
    Timeout,
    RateLimited,
    NotFound,
    InvalidRequest,
    Validation,
    Server,
    Unknown,
}

/// User-facing failure information with consistent messaging.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct UserFacingFailure {
    pub category: FailureCategory,
    pub title: &'static str,
    pub diagnosis: String,
    pub action_hints: Vec<String>,
    pub status_code: Option<u16>,
    pub request_id: Option<String>,
}

/// Clone-safe snapshot of a reqwest error.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HttpErrorSnapshot {
    message: String,
    status: Option<u16>,
    url: Option<String>,
    source: Option<String>,
    is_timeout: bool,
    is_connect: bool,
}

impl HttpErrorSnapshot {
    /// Build a clone-safe snapshot from a reqwest error.
    pub fn from_reqwest_error(error: &reqwest::Error) -> Self {
        Self {
            message: error.to_string(),
            status: error.status().map(|status| status.as_u16()),
            url: error.url().map(|url| url.to_string()),
            source: error.source().map(|source| source.to_string()),
            is_timeout: error.is_timeout(),
            is_connect: error.is_connect(),
        }
    }

    /// Returns the HTTP status code, if one was attached to the error.
    pub fn status(&self) -> Option<u16> {
        self.status
    }

    /// Returns the request URL associated with the error, if any.
    pub fn url(&self) -> Option<&str> {
        self.url.as_deref()
    }

    /// True when the error represents a timeout.
    pub fn is_timeout(&self) -> bool {
        self.is_timeout
    }

    /// True when the error represents a connection failure.
    pub fn is_connect(&self) -> bool {
        self.is_connect
    }

    /// Lowercased combined message/source text for string-based fallback classification.
    pub fn classification_text(&self) -> String {
        format!(
            "{} {}",
            self.message.to_lowercase(),
            self.source.as_deref().unwrap_or_default().to_lowercase()
        )
    }
}

impl std::fmt::Display for HttpErrorSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.message)
    }
}

impl std::error::Error for HttpErrorSnapshot {}

impl From<reqwest::Error> for HttpErrorSnapshot {
    fn from(error: reqwest::Error) -> Self {
        Self::from_reqwest_error(&error)
    }
}

/// Errors that can occur during Splunk client operations.
#[derive(Error, Debug, Clone)]
pub enum ClientError {
    #[error("Authentication failed: {0}")]
    AuthFailed(String),

    #[error("HTTP error: {0}")]
    HttpError(HttpErrorSnapshot),

    #[error("API error ({status}) at {url}: {message}{}", .request_id.as_ref().map(|id| format!(" [Request ID: {id}]")).unwrap_or_default())]
    ApiError {
        status: u16,
        url: String,
        message: String,
        request_id: Option<String>,
    },

    #[error("Session expired for user '{username}', please re-authenticate")]
    SessionExpired { username: String },

    #[error("Invalid response format: {0}")]
    InvalidResponse(String),

    #[error("Operation '{operation}' timed out after {timeout:?}")]
    OperationTimeout {
        operation: &'static str,
        timeout: Duration,
    },

    #[error("Rate limited: retry after {0:?}")]
    RateLimited(Option<Duration>),

    #[error("Connection refused to {0}")]
    ConnectionRefused(String),

    #[error("TLS error: {0}")]
    TlsError(String),

    #[error("Maximum retries exceeded ({0} attempts): {1}")]
    MaxRetriesExceeded(usize, Box<ClientError>),

    #[error("Invalid URL: {0}")]
    InvalidUrl(String),

    #[error("Resource not found: {0}")]
    NotFound(String),

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Token refresh failed for user '{username}' using {auth_method}: {source}")]
    TokenRefreshFailed {
        username: String,
        auth_method: String,
        #[source]
        source: Box<ClientError>,
    },

    #[error("Invalid request: {0}")]
    InvalidRequest(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Circuit breaker open: {0}")]
    CircuitBreakerOpen(String),

    #[error("Transaction rollback failed with {count} error(s): {}", failures.iter().map(|f| f.to_string()).collect::<Vec<_>>().join("; "))]
    TransactionRollbackError {
        count: usize,
        failures: Vec<RollbackFailure>,
    },
}

impl From<reqwest::Error> for ClientError {
    fn from(error: reqwest::Error) -> Self {
        Self::HttpError(error.into())
    }
}

impl From<crate::client::circuit_breaker::CircuitBreakerError> for ClientError {
    fn from(error: crate::client::circuit_breaker::CircuitBreakerError) -> Self {
        match error {
            crate::client::circuit_breaker::CircuitBreakerError::CircuitOpen { endpoint } => {
                Self::CircuitBreakerOpen(endpoint)
            }
        }
    }
}
