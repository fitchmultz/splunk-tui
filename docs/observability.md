# Observability Guide

## Prometheus Metrics

Both `splunk-cli` and `splunk-tui` can expose Prometheus metrics for production monitoring.

### Enabling Metrics

Use the `--metrics-bind` flag or `SPLUNK_METRICS_BIND` environment variable:

```bash
# CLI
splunk-cli --metrics-bind localhost:9090 health

# TUI
splunk-tui --metrics-bind localhost:9090

# Or via environment variable
export SPLUNK_METRICS_BIND=0.0.0.0:9090
splunk-tui
```

### Available Metrics

#### API Metrics

| Metric | Type | Labels | Description |
|--------|------|--------|-------------|
| `splunk_api_request_duration_seconds` | Histogram | `endpoint`, `method`, `status` | Request latency |
| `splunk_api_requests_total` | Counter | `endpoint`, `method` | Total requests |
| `splunk_api_retries_total` | Counter | `endpoint`, `method`, `attempt` | Retry attempts |
| `splunk_api_errors_total` | Counter | `endpoint`, `method`, `error_category` | Error count |
| `splunk_api_cache_hits_total` | Counter | - | Cache hits |
| `splunk_api_cache_misses_total` | Counter | - | Cache misses |
| `splunk_api_cache_size` | Gauge | - | Current cache size |

#### TUI Metrics

| Metric | Type | Description |
|--------|------|-------------|
| `splunk_tui_frame_render_duration_seconds` | Histogram | Frame render time |
| `splunk_tui_action_queue_depth` | Gauge | Action queue depth |

### Error Categories

The `error_category` label can have these values:

- `transport` - Connection/DNS issues
- `http_4xx` - Client errors
- `http_5xx` - Server errors
- `api` - API-level errors
- `timeout` - Request timeouts
- `tls` - TLS/SSL errors
- `unknown` - Unclassified errors

### Example Prometheus Configuration

```yaml
scrape_configs:
  - job_name: 'splunk-cli'
    static_configs:
      - targets: ['localhost:9090']
    scrape_interval: 15s
```

### Metric Buckets

#### Request Duration Buckets (seconds)

- 0.001, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0

#### TUI Frame Render Duration Buckets (seconds)

- 0.0001, 0.0005, 0.001, 0.0025, 0.005, 0.01, 0.025, 0.05, 0.1, 0.25, 0.5

## Grafana Dashboard

A sample Grafana dashboard is provided in [`grafana-dashboard.json`](grafana-dashboard.json).

To import:

1. Open Grafana → Create → Import
2. Upload `grafana-dashboard.json`
3. Select your Prometheus data source
4. Click Import

## Usage Examples

### Viewing Metrics with curl

```bash
# Start CLI with metrics endpoint
splunk-cli --metrics-bind localhost:9090 search 'search index=_internal | head 1'

# In another terminal, scrape metrics
curl -s http://localhost:9090/metrics

# Filter for specific metrics
curl -s http://localhost:9090/metrics | grep splunk_api_requests_total
curl -s http://localhost:9090/metrics | grep splunk_api_request_duration_seconds_bucket
```

### Monitoring Cache Efficiency

```promql
# Cache hit ratio
splunk_api_cache_hits_total / (splunk_api_cache_hits_total + splunk_api_cache_misses_total)

# Cache size over time
splunk_api_cache_size
```

### Monitoring API Performance

```promql
# Request rate by endpoint
rate(splunk_api_requests_total[5m])

# p99 latency by endpoint
histogram_quantile(0.99, rate(splunk_api_request_duration_seconds_bucket[5m]))

# Error rate by category
rate(splunk_api_errors_total[5m])
```

### Monitoring TUI Performance

```promql
# Average frame render time
rate(splunk_tui_frame_render_duration_seconds_sum[5m]) / rate(splunk_tui_frame_render_duration_seconds_count[5m])

# Action queue depth
splunk_tui_action_queue_depth
```

## Security Considerations

- The metrics endpoint does NOT expose sensitive data (no credentials, tokens, or query content)
- Bind to localhost by default for safety
- Use firewall/network policies for remote access
- The metrics endpoint is unauthenticated; secure it via network policies in production

## Troubleshooting

### Metrics Not Appearing

1. Verify the exporter started: Check logs for `Metrics exporter started`
2. Test the endpoint: `curl http://localhost:9090/metrics`
3. Ensure no firewall is blocking the port

### Port Already in Use

If you see "Address already in use":

```bash
# Find the process using the port
lsof -i :9090

# Or use a different port
splunk-cli --metrics-bind localhost:9091 health
```

### High Memory Usage

The Prometheus client keeps all metrics in memory. If memory usage is high:

1. Check for high-cardinality labels (unique values per request)
2. Verify metrics are being scraped regularly (old data is not retained)
3. Consider reducing the number of buckets if histogram memory is high
