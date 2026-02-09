# Observability Guide

Splunk TUI and CLI support OpenTelemetry distributed tracing for production debugging and performance analysis.

## Quick Start

### Run with Jaeger (Local Development)

1. Start Jaeger:
   ```bash
   docker run -d --name jaeger \
     -p 16686:16686 \
     -p 4317:4317 \
     jaegertracing/all-in-one:1.50
   ```

2. Run CLI with tracing:
   ```bash
   splunk-cli --otlp-endpoint http://localhost:4317 search 'index=_internal | head 10' --wait
   ```

3. View traces at http://localhost:16686

### Run TUI with Tracing

```bash
splunk-tui --otlp-endpoint http://localhost:4317
```

## Configuration

### Environment Variables

| Variable | Description | Example |
|----------|-------------|---------|
| `SPLUNK_OTLP_ENDPOINT` | OTLP/gRPC endpoint for trace export | `http://localhost:4317` |
| `SPLUNK_OTEL_SERVICE_NAME` | Service name in traces | `splunk-tui-prod` |
| `RUST_LOG` | Log level filter | `info,splunk_client=debug` |

### Command-Line Flags

**CLI:**
```bash
splunk-cli --otlp-endpoint http://tempo:4317 --otel-service-name my-instance search '...'
```

**TUI:**
```bash
splunk-tui --otlp-endpoint http://jaeger:4317 --otel-service-name splunk-tui-prod
```

## Supported Backends

- **Jaeger**: Native OTLP support (v1.35+)
- **Grafana Tempo**: Full OTLP/gRPC support
- **Honeycomb**: OTLP endpoint support
- **Splunk Observability Cloud**: OTLP ingest

## Trace Structure

### Spans

**API Request Spans** (`http.request`):
- `endpoint` - API path (e.g., `/services/search/jobs`)
- `method` - HTTP method
- `status` - HTTP status code
- `duration_ms` - Request duration
- `attempt` - Retry attempt number
- `trace_id` - Correlation ID for Splunk server logs

**TUI Action Spans** (`tui.handle_action`):
- `action_type` - Action variant name
- `duration_ms` - Total handling duration

**Search Spans** (`search.execute`):
- `query_hash` - Query identifier (for correlation)
- `search_mode` - Search mode (normal, realtime)
- `sid` - Search job ID

## Trace Context Propagation

Traces include W3C Trace Context headers (`traceparent`) in all HTTP requests
to Splunk. This enables correlating client behavior with Splunk server logs.

### Splunk Server Configuration

Enable trace ID logging in Splunk:

```ini
# props.conf
[default]
TRUNCATE = 999999

# transforms.conf
[traceid-extract]
REGEX = traceparent: (\d+)-([a-f0-9]+)-([a-f0-9]+)-(\d+)
FORMAT = trace_id::$2
```

## Performance Impact

- **Minimal overhead** when OTLP endpoint is not configured
- **~1-5% overhead** when tracing is enabled (mostly network I/O)
- Spans are batched and sent asynchronously
- Use sampling in production for high-volume scenarios

## Troubleshooting

### No traces appearing

1. Check endpoint connectivity:
   ```bash
   grpcurl -plaintext localhost:4317 list jaeger.api_v2.CollectorService
   ```

2. Verify `RUST_LOG` includes `info` level:
   ```bash
   RUST_LOG=info splunk-cli --otlp-endpoint ...
   ```

3. Check for errors in application logs

### High memory usage

Spans are batched in memory before export. For long-running TUI sessions,
consider increasing the batch timeout or enabling sampling.

## Metrics vs Tracing

The project uses both:

- **Metrics** (`metrics` crate): Prometheus-compatible counters/gauges for dashboards
- **Tracing** (OpenTelemetry): Distributed request flows for debugging

Use metrics for monitoring, tracing for investigation.
