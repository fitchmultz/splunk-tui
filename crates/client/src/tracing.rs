//! Low-level trace context propagation helpers for client requests.
//!
//! Responsibilities:
//! - Inject the current trace context into outgoing HTTP headers.
//! - Extract trace context from HTTP response headers when needed.
//!
//! Does NOT handle:
//! - Subscriber initialization.
//! - OTLP exporter bootstrapping.
//! - Metrics recorder installation.
//!
//! Invariants:
//! - Uses W3C Trace Context propagation for interoperability.

/// Propagate trace context to HTTP request headers.
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
