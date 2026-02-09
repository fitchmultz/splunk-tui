//! OpenTelemetry tracing initialization and configuration.
//!
//! This module provides centralized OpenTelemetry setup for both CLI and TUI.
//! It handles OTLP exporter configuration, tracer provider setup, and
//! trace context propagation.
//!
//! # Usage
//!
//! ```rust,ignore
//! use splunk_client::tracing::TracingConfig;
//!
//! let config = TracingConfig::builder()
//!     .otlp_endpoint("http://localhost:4317")
//!     .service_name("splunk-cli")
//!     .build();
//!
//! let _tracer = config.init()?;
//! // Run application...
//! config.shutdown(); // Flush spans before exit
//! ```

use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::runtime::Tokio;
use std::time::Duration;
use tracing_subscriber::{EnvFilter, layer::SubscriberExt, util::SubscriberInitExt};

/// Configuration for OpenTelemetry tracing.
#[derive(Debug, Clone)]
pub struct TracingConfig {
    /// OTLP endpoint (e.g., "http://localhost:4317" for Jaeger/Tempo)
    pub otlp_endpoint: Option<String>,
    /// Service name for trace attribution
    pub service_name: String,
    /// Service version
    pub service_version: String,
    /// Whether to enable stdout logging layer alongside OTLP
    pub enable_stdout: bool,
    /// Batch span processor timeout
    pub timeout: Duration,
}

impl Default for TracingConfig {
    fn default() -> Self {
        Self {
            otlp_endpoint: std::env::var("SPLUNK_OTLP_ENDPOINT").ok(),
            service_name: "splunk-client".to_string(),
            service_version: env!("CARGO_PKG_VERSION").to_string(),
            enable_stdout: true,
            timeout: Duration::from_secs(5),
        }
    }
}

impl TracingConfig {
    /// Create a new config with defaults.
    pub fn new() -> Self {
        Self::default()
    }

    /// Builder method to set OTLP endpoint.
    pub fn with_otlp_endpoint(mut self, endpoint: impl Into<String>) -> Self {
        self.otlp_endpoint = Some(endpoint.into());
        self
    }

    /// Builder method to set service name.
    pub fn with_service_name(mut self, name: impl Into<String>) -> Self {
        self.service_name = name.into();
        self
    }

    /// Builder method to set service version.
    pub fn with_service_version(mut self, version: impl Into<String>) -> Self {
        self.service_version = version.into();
        self
    }

    /// Builder method to control stdout layer.
    pub fn with_stdout(mut self, enable: bool) -> Self {
        self.enable_stdout = enable;
        self
    }

    /// Initialize the tracing subscriber with OpenTelemetry layer.
    ///
    /// # Returns
    /// A guard that must be held until application shutdown to ensure
    /// all spans are flushed.
    ///
    /// # Errors
    /// Returns an error if the OTLP pipeline fails to initialize.
    pub fn init(&self) -> Result<TracingGuard, TracingError> {
        use tracing_subscriber::fmt;

        let env_filter =
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

        // Add OpenTelemetry layer if OTLP endpoint is configured
        let tracer = if let Some(ref endpoint) = self.otlp_endpoint {
            Some(self.create_tracer(endpoint)?)
        } else {
            None
        };

        // Build and initialize subscriber based on configuration
        match (tracer.as_ref(), self.enable_stdout) {
            (Some(tracer), true) => {
                let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(otel_layer)
                    .with(fmt::layer())
                    .init();
            }
            (Some(tracer), false) => {
                let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer.clone());
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(otel_layer)
                    .init();
            }
            (None, true) => {
                tracing_subscriber::registry()
                    .with(env_filter)
                    .with(fmt::layer())
                    .init();
            }
            (None, false) => {
                tracing_subscriber::registry().with(env_filter).init();
            }
        }

        Ok(TracingGuard { tracer })
    }

    fn create_tracer(
        &self,
        endpoint: &str,
    ) -> Result<opentelemetry_sdk::trace::Tracer, TracingError> {
        use opentelemetry_otlp::{Protocol, WithExportConfig};
        use opentelemetry_sdk::trace::{BatchConfig, BatchSpanProcessor, Sampler};

        let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
            .with_tonic()
            .with_endpoint(endpoint)
            .with_timeout(self.timeout)
            .with_protocol(Protocol::Grpc)
            .build()
            .map_err(|e| TracingError::InitError(e.to_string()))?;

        let batch_config = BatchConfig::default();

        let batch_processor = BatchSpanProcessor::builder(otlp_exporter, Tokio)
            .with_batch_config(batch_config)
            .build();

        let resource = opentelemetry_sdk::Resource::new(vec![
            opentelemetry::KeyValue::new("service.name", self.service_name.clone()),
            opentelemetry::KeyValue::new("service.version", self.service_version.clone()),
            opentelemetry::KeyValue::new("telemetry.sdk.name", "opentelemetry-rust"),
            opentelemetry::KeyValue::new("telemetry.sdk.language", "rust"),
        ]);

        let provider = opentelemetry_sdk::trace::TracerProvider::builder()
            .with_span_processor(batch_processor)
            .with_resource(resource)
            .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
                1.0,
            ))))
            .build();

        Ok(provider.tracer("splunk-client"))
    }
}

/// Guard that holds tracer resources.
///
/// Must be kept alive until application shutdown to ensure all
/// pending spans are exported.
pub struct TracingGuard {
    /// Tracer instance - held to keep tracer alive during application lifecycle.
    #[allow(dead_code)]
    tracer: Option<opentelemetry_sdk::trace::Tracer>,
}

impl TracingGuard {
    /// Shutdown the tracer and flush any pending spans.
    ///
    /// This should be called before application exit to ensure all
    /// spans are exported.
    pub fn shutdown(&self) {
        opentelemetry::global::shutdown_tracer_provider();
    }
}

/// Errors that can occur during tracing initialization.
#[derive(Debug, thiserror::Error)]
pub enum TracingError {
    #[error("Failed to initialize OpenTelemetry: {0}")]
    InitError(String),
}

/// Propagate trace context to HTTP request headers.
///
/// Injects the current span context into the request headers using
/// W3C Trace Context format (traceparent header).
///
/// # Arguments
/// * `builder` - The reqwest RequestBuilder to add headers to
///
/// # Returns
/// The modified RequestBuilder with trace context headers
pub fn inject_trace_context(builder: reqwest::RequestBuilder) -> reqwest::RequestBuilder {
    use opentelemetry::propagation::TextMapPropagator;
    use opentelemetry_sdk::propagation::TraceContextPropagator;
    use std::collections::HashMap;

    let propagator = TraceContextPropagator::new();
    let mut headers = HashMap::new();

    propagator.inject_context(&opentelemetry::Context::current(), &mut headers);

    let mut result = builder;
    for (key, value) in headers {
        result = result.header(key, value);
    }
    result
}

/// Extract trace context from HTTP response headers.
///
/// This is useful when receiving callbacks/webhooks from Splunk
/// to continue a trace that started on the server.
///
/// # Arguments
/// * `headers` - The HTTP response headers
///
/// # Returns
/// A context that can be used as parent for new spans
#[allow(dead_code)]
pub fn extract_trace_context(headers: &reqwest::header::HeaderMap) -> opentelemetry::Context {
    use opentelemetry::propagation::TextMapPropagator;
    use opentelemetry_sdk::propagation::TraceContextPropagator;
    use std::collections::HashMap;

    let propagator = TraceContextPropagator::new();
    let mut headers_map = HashMap::new();

    for (key, value) in headers.iter() {
        if let Ok(v) = value.to_str() {
            headers_map.insert(key.as_str().to_string(), v.to_string());
        }
    }

    propagator.extract(&headers_map)
}
