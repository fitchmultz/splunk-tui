//! TUI-local telemetry bootstrap and exporter wiring.
//!
//! Responsibilities:
//! - Initialize TUI tracing/logging output.
//! - Optionally attach OTLP export for traces.
//! - Optionally attach a Prometheus metrics exporter.
//!
//! Does NOT handle:
//! - Request-level trace propagation (handled by `splunk-client`).
//! - UI rendering or event processing.
//!
//! Invariants:
//! - The TUI owns its file logger and telemetry lifecycle.
//! - OTLP and metrics bootstrapping are binary concerns, not client-library concerns.

use metrics_exporter_prometheus::PrometheusBuilder;
use opentelemetry::trace::TracerProvider;
use opentelemetry_sdk::trace::SdkTracerProvider;
use std::net::SocketAddr;
use std::path::Path;
use std::time::Duration;
use tracing_appender::non_blocking::WorkerGuard;
use tracing_subscriber::{EnvFilter, fmt, layer::SubscriberExt, util::SubscriberInitExt};

/// Holds telemetry resources for the TUI process lifetime.
#[allow(dead_code)]
pub struct TelemetryState {
    tracing_guard: Option<TracingGuard>,
    log_guard: Option<WorkerGuard>,
    metrics_exporter: Option<MetricsExporter>,
}

impl TelemetryState {
    pub fn metrics_enabled(&self) -> bool {
        self.metrics_exporter.is_some()
    }
}

/// Initialize TUI-local tracing, file logging, and optional metrics exporting.
pub fn init(
    otlp_endpoint: Option<&str>,
    service_name: Option<&str>,
    log_dir: &Path,
    metrics_bind: Option<&str>,
) -> Result<TelemetryState, String> {
    let (tracing_guard, log_guard) = init_tracing(otlp_endpoint, service_name, log_dir)?;
    let metrics_exporter = if let Some(bind_addr) = metrics_bind {
        Some(MetricsExporter::install(bind_addr).map_err(|e| e.to_string())?)
    } else {
        None
    };

    Ok(TelemetryState {
        tracing_guard,
        log_guard,
        metrics_exporter,
    })
}

fn init_tracing(
    otlp_endpoint: Option<&str>,
    service_name: Option<&str>,
    log_dir: &Path,
) -> Result<(Option<TracingGuard>, Option<WorkerGuard>), String> {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));

    if let Some(endpoint) = otlp_endpoint {
        let provider = create_tracer_provider(
            endpoint,
            service_name.unwrap_or("splunk-tui"),
            env!("CARGO_PKG_VERSION"),
        )?;
        let tracer = provider.tracer("splunk-tui");
        let otel_layer = tracing_opentelemetry::layer().with_tracer(tracer);
        tracing_subscriber::registry()
            .with(env_filter)
            .with(otel_layer)
            .init();
        Ok((
            Some(TracingGuard {
                provider: Some(provider),
            }),
            None,
        ))
    } else {
        let file_appender = tracing_appender::rolling::daily(log_dir, "splunk-tui.log");
        let (file_writer, guard) = tracing_appender::non_blocking(file_appender);

        tracing_subscriber::registry()
            .with(env_filter)
            .with(fmt::layer().with_writer(file_writer))
            .init();

        Ok((None, Some(guard)))
    }
}

fn create_tracer_provider(
    endpoint: &str,
    service_name: &str,
    service_version: &str,
) -> Result<SdkTracerProvider, String> {
    use opentelemetry_otlp::{Protocol, WithExportConfig};
    use opentelemetry_sdk::trace::{BatchConfig, BatchSpanProcessor, Sampler};

    let otlp_exporter = opentelemetry_otlp::SpanExporter::builder()
        .with_tonic()
        .with_endpoint(endpoint)
        .with_timeout(Duration::from_secs(5))
        .with_protocol(Protocol::Grpc)
        .build()
        .map_err(|e| e.to_string())?;

    let batch_processor = BatchSpanProcessor::builder(otlp_exporter)
        .with_batch_config(BatchConfig::default())
        .build();

    let resource = opentelemetry_sdk::Resource::builder()
        .with_attributes(vec![
            opentelemetry::KeyValue::new("service.name", service_name.to_string()),
            opentelemetry::KeyValue::new("service.version", service_version.to_string()),
            opentelemetry::KeyValue::new("telemetry.sdk.name", "opentelemetry-rust"),
            opentelemetry::KeyValue::new("telemetry.sdk.language", "rust"),
        ])
        .build();

    Ok(SdkTracerProvider::builder()
        .with_span_processor(batch_processor)
        .with_resource(resource)
        .with_sampler(Sampler::ParentBased(Box::new(Sampler::TraceIdRatioBased(
            1.0,
        ))))
        .build())
}

struct TracingGuard {
    provider: Option<SdkTracerProvider>,
}

impl Drop for TracingGuard {
    fn drop(&mut self) {
        if let Some(provider) = self.provider.take() {
            let _ = provider.shutdown();
        }
    }
}

struct MetricsExporter {
    bind_addr: SocketAddr,
}

impl MetricsExporter {
    fn install(bind_addr: &str) -> Result<Self, MetricsExporterError> {
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

        tracing::info!(
            "Prometheus metrics exporter started on http://{}/metrics",
            addr
        );

        Ok(Self { bind_addr: addr })
    }
}

impl Drop for MetricsExporter {
    fn drop(&mut self) {
        let _ = self.bind_addr;
    }
}

#[derive(Debug, thiserror::Error)]
enum MetricsExporterError {
    #[error("Invalid bind address '{0}': {1}")]
    InvalidBindAddress(String, std::net::AddrParseError),
    #[error("A metrics recorder is already installed")]
    RecorderAlreadyInstalled,
    #[error("Failed to build Prometheus recorder: {0}")]
    BuildError(String),
}

impl From<metrics_exporter_prometheus::BuildError> for MetricsExporterError {
    fn from(err: metrics_exporter_prometheus::BuildError) -> Self {
        MetricsExporterError::BuildError(err.to_string())
    }
}
