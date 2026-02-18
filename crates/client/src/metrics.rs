//! Metrics collection for API call performance.
//!
//! This module provides metrics collection for Splunk API calls, including:
//! - Request latency histograms
//! - Request counters (total, retries, errors)
//! - Error categorization
//!
//! # What this module does NOT handle:
//! - Metrics exposition/export (use a metrics exporter like `metrics-exporter-prometheus`)
//! - Persistent storage of metrics
//! - Alerting or threshold monitoring
//!
//! # Invariants
//! - All metrics use consistent label names: `endpoint`, `method`, `status`, `error_category`
//! - Metric recording is infallible (errors are silently ignored to prevent disrupting API calls)
//! - Zero-cost when no metrics recorder is installed

use crate::error::ClientError;
use std::time::Duration;

/// Metric name for request duration histogram.
pub const METRIC_REQUEST_DURATION: &str = "splunk_api_request_duration_seconds";

/// Metric name for total request counter.
pub const METRIC_REQUESTS_TOTAL: &str = "splunk_api_requests_total";

/// Metric name for retry counter.
pub const METRIC_RETRIES_TOTAL: &str = "splunk_api_retries_total";

/// Metric name for error counter.
pub const METRIC_ERRORS_TOTAL: &str = "splunk_api_errors_total";

/// Metric name for cache hit counter.
pub const METRIC_CACHE_HITS: &str = "splunk_api_cache_hits_total";

/// Metric name for cache miss counter.
pub const METRIC_CACHE_MISSES: &str = "splunk_api_cache_misses_total";

/// Metric name for cache size gauge.
pub const METRIC_CACHE_SIZE: &str = "splunk_api_cache_size";

/// Metric name for TUI frame render duration histogram.
pub const METRIC_TUI_FRAME_RENDER_DURATION: &str = "splunk_tui_frame_render_duration_seconds";

/// Metric name for TUI action queue depth gauge.
pub const METRIC_TUI_ACTION_QUEUE_DEPTH: &str = "splunk_tui_action_queue_depth";

/// Metric name for deserialization failure counter.
pub const METRIC_DESERIALIZATION_FAILURES: &str = "splunk_api_deserialization_failures_total";

/// Metric name for UX auth recovery popup shown.
pub const METRIC_UX_AUTH_RECOVERY_TOTAL: &str = "splunk_tui_ux_auth_recovery_total";

/// Metric name for UX auth recovery action success.
pub const METRIC_UX_AUTH_RECOVERY_SUCCESS: &str = "splunk_tui_ux_auth_recovery_success_total";

/// Metric name for UX navigation reversal.
pub const METRIC_UX_NAVIGATION_REVERSAL: &str = "splunk_tui_ux_navigation_reversal_total";

/// Metric name for UX help opened.
pub const METRIC_UX_HELP_OPENED: &str = "splunk_tui_ux_help_opened_total";

/// Metric name for UX bootstrap connect attempts.
pub const METRIC_UX_BOOTSTRAP_CONNECT: &str = "splunk_tui_ux_bootstrap_connect_total";

/// Error categories for metrics labeling.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCategory {
    /// Transport-level errors (connection refused, DNS, etc.)
    Transport,
    /// HTTP 4xx client errors
    Http4xx,
    /// HTTP 5xx server errors
    Http5xx,
    /// API-level errors (parsed from response body)
    Api,
    /// Request timeout
    Timeout,
    /// TLS/SSL errors
    Tls,
    /// Unknown/unclassified errors
    Unknown,
}

impl ErrorCategory {
    /// Returns the string label for this error category.
    pub const fn as_str(&self) -> &'static str {
        match self {
            ErrorCategory::Transport => "transport",
            ErrorCategory::Http4xx => "http_4xx",
            ErrorCategory::Http5xx => "http_5xx",
            ErrorCategory::Api => "api",
            ErrorCategory::Timeout => "timeout",
            ErrorCategory::Tls => "tls",
            ErrorCategory::Unknown => "unknown",
        }
    }
}

impl From<&ClientError> for ErrorCategory {
    /// Categorize a ClientError for metrics purposes.
    fn from(error: &ClientError) -> Self {
        match error {
            ClientError::OperationTimeout { .. } => ErrorCategory::Timeout,
            ClientError::ConnectionRefused(_) => ErrorCategory::Transport,
            ClientError::TlsError(_) => ErrorCategory::Tls,
            ClientError::ApiError { status, .. } => {
                if (400..500).contains(status) {
                    ErrorCategory::Http4xx
                } else if (500..600).contains(status) {
                    ErrorCategory::Http5xx
                } else {
                    ErrorCategory::Api
                }
            }
            ClientError::HttpError(e) => {
                // Try to determine if it's a transport error or HTTP error
                let err_str = e.to_string().to_lowercase();
                if err_str.contains("connection")
                    || err_str.contains("dns")
                    || err_str.contains("reset")
                    || err_str.contains("refused")
                {
                    ErrorCategory::Transport
                } else {
                    ErrorCategory::Unknown
                }
            }
            ClientError::MaxRetriesExceeded(_, inner) => ErrorCategory::from(inner.as_ref()),
            _ => ErrorCategory::Unknown,
        }
    }
}

/// Metrics collector for Splunk API calls.
///
/// This struct provides a lightweight wrapper around the `metrics` crate macros,
/// providing type-safe methods for recording API metrics with consistent labels.
///
/// # Example
///
/// ```rust,ignore
/// use splunk_client::metrics::MetricsCollector;
///
/// let collector = MetricsCollector::new();
/// collector.record_request_duration("/services/search/jobs", "POST", Duration::from_millis(150), Some(200));
/// ```
#[derive(Debug, Clone, Default)]
pub struct MetricsCollector {
    /// Whether metrics collection is enabled.
    enabled: bool,
}

impl MetricsCollector {
    /// Create a new metrics collector.
    ///
    /// The collector is enabled by default. Use [`Self::disabled()`] to create
    /// a collector that does not record any metrics.
    pub fn new() -> Self {
        Self { enabled: true }
    }

    /// Create a disabled metrics collector.
    ///
    /// This is useful when metrics are conditionally enabled and you need
    /// a placeholder that implements the same interface but does nothing.
    pub fn disabled() -> Self {
        Self { enabled: false }
    }

    /// Check if metrics collection is enabled.
    pub fn is_enabled(&self) -> bool {
        self.enabled
    }

    /// Record the duration of an API request.
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint path (e.g., "/services/search/jobs")
    /// * `method` - The HTTP method (e.g., "GET", "POST")
    /// * `duration` - The request duration
    /// * `status` - The HTTP status code, or None if the request failed before receiving a response
    pub fn record_request_duration(
        &self,
        endpoint: &str,
        method: &str,
        duration: Duration,
        status: Option<u16>,
    ) {
        if !self.enabled {
            return;
        }

        let status_label = status.map_or("error".to_string(), |s| s.to_string());

        metrics::histogram!(METRIC_REQUEST_DURATION,
            "endpoint" => endpoint.to_string(),
            "method" => method.to_string(),
            "status" => status_label,
        )
        .record(duration.as_secs_f64());
    }

    /// Record a request attempt.
    ///
    /// This should be called for every request attempt, including retries.
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint path
    /// * `method` - The HTTP method
    pub fn record_request(&self, endpoint: &str, method: &str) {
        if !self.enabled {
            return;
        }

        metrics::counter!(METRIC_REQUESTS_TOTAL,
            "endpoint" => endpoint.to_string(),
            "method" => method.to_string(),
        )
        .increment(1);
    }

    /// Record a retry attempt.
    ///
    /// This should be called for each retry attempt (not the initial request).
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint path
    /// * `method` - The HTTP method
    /// * `attempt` - The retry attempt number (1-based)
    pub fn record_retry(&self, endpoint: &str, method: &str, attempt: usize) {
        if !self.enabled {
            return;
        }

        metrics::counter!(METRIC_RETRIES_TOTAL,
            "endpoint" => endpoint.to_string(),
            "method" => method.to_string(),
            "attempt" => attempt.to_string(),
        )
        .increment(1);
    }

    /// Record an error.
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint path
    /// * `method` - The HTTP method
    /// * `category` - The error category
    pub fn record_error(&self, endpoint: &str, method: &str, category: ErrorCategory) {
        if !self.enabled {
            return;
        }

        metrics::counter!(METRIC_ERRORS_TOTAL,
            "endpoint" => endpoint.to_string(),
            "method" => method.to_string(),
            "error_category" => category.as_str(),
        )
        .increment(1);
    }

    /// Record an error from a ClientError.
    ///
    /// This is a convenience method that categorizes the error automatically.
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint path
    /// * `method` - The HTTP method
    /// * `error` - The client error
    pub fn record_client_error(&self, endpoint: &str, method: &str, error: &ClientError) {
        let category = ErrorCategory::from(error);
        self.record_error(endpoint, method, category);
    }

    /// Record a cache hit.
    pub fn record_cache_hit(&self) {
        if !self.enabled {
            return;
        }
        metrics::counter!(METRIC_CACHE_HITS).increment(1);
    }

    /// Record a cache miss.
    pub fn record_cache_miss(&self) {
        if !self.enabled {
            return;
        }
        metrics::counter!(METRIC_CACHE_MISSES).increment(1);
    }

    /// Record current cache size.
    pub fn record_cache_size(&self, size: u64) {
        if !self.enabled {
            return;
        }
        metrics::gauge!(METRIC_CACHE_SIZE).set(size as f64);
    }

    /// Record a deserialization failure.
    ///
    /// # Arguments
    /// * `endpoint` - The API endpoint path
    /// * `model_type` - The type name that failed to deserialize (e.g., "LogEntry", "LogParsingError")
    pub fn record_deserialization_failure(&self, endpoint: &str, model_type: &'static str) {
        if !self.enabled {
            return;
        }
        metrics::counter!(METRIC_DESERIALIZATION_FAILURES,
            "endpoint" => endpoint.to_string(),
            "model_type" => model_type.to_string(),
        )
        .increment(1);
    }

    /// Record TUI frame render duration.
    ///
    /// # Arguments
    /// * `duration` - The time taken to render a frame
    pub fn record_tui_frame_render_duration(&self, duration: Duration) {
        if !self.enabled {
            return;
        }
        metrics::histogram!(METRIC_TUI_FRAME_RENDER_DURATION).record(duration.as_secs_f64());
    }

    /// Record TUI action queue depth.
    ///
    /// # Arguments
    /// * `depth` - Current number of items in the action queue
    pub fn record_tui_action_queue_depth(&self, depth: usize) {
        if !self.enabled {
            return;
        }
        metrics::gauge!(METRIC_TUI_ACTION_QUEUE_DEPTH).set(depth as f64);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_category_as_str() {
        assert_eq!(ErrorCategory::Transport.as_str(), "transport");
        assert_eq!(ErrorCategory::Http4xx.as_str(), "http_4xx");
        assert_eq!(ErrorCategory::Http5xx.as_str(), "http_5xx");
        assert_eq!(ErrorCategory::Api.as_str(), "api");
        assert_eq!(ErrorCategory::Timeout.as_str(), "timeout");
        assert_eq!(ErrorCategory::Tls.as_str(), "tls");
        assert_eq!(ErrorCategory::Unknown.as_str(), "unknown");
    }

    #[test]
    fn test_error_categorization() {
        let timeout_err = ClientError::OperationTimeout {
            operation: "test",
            timeout: Duration::from_secs(1),
        };
        assert_eq!(ErrorCategory::from(&timeout_err), ErrorCategory::Timeout);

        let conn_err = ClientError::ConnectionRefused("localhost:8089".to_string());
        assert_eq!(ErrorCategory::from(&conn_err), ErrorCategory::Transport);

        let tls_err = ClientError::TlsError("cert error".to_string());
        assert_eq!(ErrorCategory::from(&tls_err), ErrorCategory::Tls);

        let api_400 = ClientError::ApiError {
            status: 400,
            url: "test".to_string(),
            message: "bad request".to_string(),
            request_id: None,
        };
        assert_eq!(ErrorCategory::from(&api_400), ErrorCategory::Http4xx);

        let api_500 = ClientError::ApiError {
            status: 500,
            url: "test".to_string(),
            message: "server error".to_string(),
            request_id: None,
        };
        assert_eq!(ErrorCategory::from(&api_500), ErrorCategory::Http5xx);

        let api_200 = ClientError::ApiError {
            status: 200,
            url: "test".to_string(),
            message: "ok".to_string(),
            request_id: None,
        };
        assert_eq!(ErrorCategory::from(&api_200), ErrorCategory::Api);
    }

    #[test]
    fn test_max_retries_exceeded_categorization() {
        let inner = ClientError::OperationTimeout {
            operation: "test",
            timeout: Duration::from_secs(1),
        };
        let outer = ClientError::MaxRetriesExceeded(3, Box::new(inner));
        assert_eq!(ErrorCategory::from(&outer), ErrorCategory::Timeout);
    }

    #[test]
    fn test_metrics_collector_enabled() {
        let collector = MetricsCollector::new();
        assert!(collector.is_enabled());

        let disabled = MetricsCollector::disabled();
        assert!(!disabled.is_enabled());
    }
}
