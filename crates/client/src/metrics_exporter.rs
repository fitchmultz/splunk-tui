//! Prometheus metrics exporter for production observability.
//!
//! This module provides HTTP endpoint exposition of metrics collected
//! by the `metrics` crate. It uses `metrics-exporter-prometheus` to
//! serve metrics in Prometheus text format at `/metrics`.
//!
//! # Example
//!
//! ```rust,ignore
//! use splunk_client::metrics_exporter::MetricsExporter;
//!
//! // Start exporter on localhost:9090
//! let exporter = MetricsExporter::install("localhost:9090")
//!     .expect("Failed to start metrics exporter");
//!
//! // Exporter runs until dropped
//! ```

use std::net::SocketAddr;

use metrics_exporter_prometheus::PrometheusBuilder;
use tracing::info;

/// Metrics exporter for Prometheus scraping.
///
/// When created, this installs a global PrometheusRecorder and starts
/// an HTTP server on the specified bind address serving `/metrics`.
pub struct MetricsExporter {
    bind_addr: SocketAddr,
}

impl MetricsExporter {
    /// Install the Prometheus exporter as the global metrics recorder.
    ///
    /// # Arguments
    /// * `bind_addr` - Socket address to bind the HTTP server (e.g., "localhost:9090")
    ///
    /// # Errors
    /// Returns an error if:
    /// - The bind address is invalid
    /// - Another recorder is already installed
    /// - The HTTP server fails to start
    ///
    /// # Example
    /// ```rust,ignore
    /// let exporter = MetricsExporter::install("0.0.0.0:9090")?;
    /// ```
    pub fn install(bind_addr: &str) -> Result<Self, MetricsExporterError> {
        let addr: SocketAddr = bind_addr
            .parse()
            .map_err(|e| MetricsExporterError::InvalidBindAddress(bind_addr.to_string(), e))?;

        PrometheusBuilder::new()
            .set_buckets_for_metric(
                metrics_exporter_prometheus::Matcher::Full(
                    "splunk_api_request_duration_seconds".to_string(),
                ),
                &[
                    0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0,
                ],
            )?
            .set_buckets_for_metric(
                metrics_exporter_prometheus::Matcher::Full(
                    "splunk_tui_frame_render_duration_seconds".to_string(),
                ),
                &[
                    0.0001, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5,
                ],
            )?
            .with_http_listener(addr)
            .install_recorder()
            .map_err(|_| MetricsExporterError::RecorderAlreadyInstalled)?;

        info!(
            "Prometheus metrics exporter started on http://{}/metrics",
            addr
        );

        Ok(Self { bind_addr: addr })
    }

    /// Get the bind address.
    pub fn bind_addr(&self) -> SocketAddr {
        self.bind_addr
    }
}

/// Errors that can occur when installing the metrics exporter.
#[derive(Debug, thiserror::Error)]
pub enum MetricsExporterError {
    /// Invalid bind address provided.
    #[error("Invalid bind address '{0}': {1}")]
    InvalidBindAddress(String, std::net::AddrParseError),

    /// A metrics recorder is already installed.
    #[error("A metrics recorder is already installed")]
    RecorderAlreadyInstalled,

    /// Failed to build the Prometheus recorder.
    #[error("Failed to build Prometheus recorder: {0}")]
    BuildError(String),
}

impl From<metrics_exporter_prometheus::BuildError> for MetricsExporterError {
    fn from(err: metrics_exporter_prometheus::BuildError) -> Self {
        MetricsExporterError::BuildError(err.to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invalid_bind_address() {
        let result = MetricsExporter::install("not-a-valid-addr");
        assert!(
            matches!(result, Err(MetricsExporterError::InvalidBindAddress(_, _))),
            "Expected InvalidBindAddress error for invalid address"
        );
    }

    #[test]
    fn test_valid_bind_address_parsing() {
        // Test that valid addresses parse correctly
        let addr: Result<SocketAddr, _> = "127.0.0.1:9090".parse();
        assert!(addr.is_ok());
        assert_eq!(addr.unwrap().to_string(), "127.0.0.1:9090");

        let addr: Result<SocketAddr, _> = "0.0.0.0:9090".parse();
        assert!(addr.is_ok());

        // Note: "localhost" may not resolve in some test environments,
        // so we only test IP addresses here
        let addr: Result<SocketAddr, _> = "[::1]:9090".parse();
        assert!(addr.is_ok());
    }

    #[test]
    fn test_error_display() {
        let parse_error = "invalid".parse::<SocketAddr>().unwrap_err();
        let error = MetricsExporterError::InvalidBindAddress("test".to_string(), parse_error);
        let error_string = error.to_string();
        assert!(error_string.contains("Invalid bind address"));
        assert!(error_string.contains("test"));

        let already_installed = MetricsExporterError::RecorderAlreadyInstalled;
        assert_eq!(
            already_installed.to_string(),
            "A metrics recorder is already installed"
        );
    }
}
