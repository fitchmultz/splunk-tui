//! Purpose: Implement internal classification and retry semantics for client errors.
//! Responsibilities: Decide retryability, auth semantics, and conversion from HTTP/transport failures into shared client error variants.
//! Scope: Internal client error classification only; user-facing rendering lives in `user_facing.rs`.
//! Usage: Called by request execution, endpoint parsing, and tests.
//! Invariants/Assumptions: 403 responses classify as semantic permission failures in the client layer and transport connection failures remain retryable.

use std::error::Error as StdError;
use std::time::Duration;

use super::kinds::ClientError;

impl ClientError {
    /// Check if this error is retryable.
    pub fn is_retryable(&self) -> bool {
        matches!(
            self,
            Self::HttpError(_)
                | Self::OperationTimeout { .. }
                | Self::RateLimited(_)
                | Self::ConnectionRefused(_)
        )
    }

    /// Check if this error is a circuit breaker error.
    pub fn is_circuit_breaker_error(&self) -> bool {
        matches!(self, Self::CircuitBreakerOpen(_))
    }

    /// Check if an HTTP status code is retryable.
    pub fn is_retryable_status(status: u16) -> bool {
        matches!(status, 429 | 502 | 503 | 504)
    }

    /// Check if this error indicates authentication failure.
    pub fn is_auth_error(&self) -> bool {
        matches!(
            self,
            Self::AuthFailed(_)
                | Self::SessionExpired { .. }
                | Self::Unauthorized(_)
                | Self::TokenRefreshFailed { .. }
        ) || matches!(self, Self::ApiError { status, .. } if *status == 401)
    }

    /// Enrich SessionExpired error with actual username if it contains "unknown".
    pub(crate) fn with_username(self, username: &str) -> Self {
        match self {
            Self::SessionExpired {
                username: ref existing,
            } if existing == "unknown" => Self::SessionExpired {
                username: username.to_string(),
            },
            other => other,
        }
    }

    /// Create a ClientError from an HTTP status response with intelligent classification.
    pub(crate) fn from_status_response(
        status: u16,
        url: String,
        message: String,
        request_id: Option<String>,
    ) -> Self {
        let lower = message.to_lowercase();

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

        if status == 403 {
            return Self::Unauthorized(message);
        }

        if status == 404 {
            return Self::NotFound(url);
        }

        if status == 400 {
            return Self::InvalidRequest(message);
        }

        Self::ApiError {
            status,
            url,
            message,
            request_id,
        }
    }

    /// Create a ClientError from a reqwest error with transport-level classification.
    pub(crate) fn from_reqwest_error_classified(error: reqwest::Error) -> Self {
        if error.is_timeout() {
            return Self::OperationTimeout {
                operation: "http_request",
                timeout: Duration::from_secs(0),
            };
        }

        if error.is_connect() {
            return Self::ConnectionRefused(error.to_string());
        }

        let snapshot = super::HttpErrorSnapshot::from_reqwest_error(&error);
        let text = snapshot.classification_text();

        if text.contains("tls")
            || text.contains("ssl")
            || text.contains("certificate")
            || text.contains("x509")
            || text.contains("handshake")
            || text.contains("unknown ca")
            || error
                .source()
                .map(|source| source.to_string().to_lowercase().contains("certificate"))
                .unwrap_or(false)
        {
            return Self::TlsError(error.to_string());
        }

        Self::HttpError(snapshot)
    }
}
