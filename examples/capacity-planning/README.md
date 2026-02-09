# Capacity Planning Examples

Scripts for analyzing growth trends, license usage, and planning Splunk capacity.

## Scripts

### index-growth.sh

Analyze index growth trends and forecast capacity needs.

```bash
# Basic growth analysis
./index-growth.sh

# Detailed per-index breakdown
./index-growth.sh --detailed

# Custom projection periods
./index-growth.sh --project 30,60,90,180

# Output JSON for trending systems
./index-growth.sh --json > index-growth-$(date +%Y%m%d).json
```

**Analysis Includes:**
- Current index sizes and event counts
- Daily ingestion rate (GB/day, events/day)
- 30/60/90-day growth projections
- Fastest growing indexes identification
- Recommendations for capacity planning

### license-usage.sh

Analyze license usage patterns and identify peak usage.

```bash
# Current license status
./license-usage.sh

# Detailed daily breakdown
./license-usage.sh --detailed

# Show top license consumers
./license-usage.sh --top 20

# Alert if usage exceeds threshold
./license-usage.sh --threshold 85 || echo "License threshold exceeded!"
```

**Reports:**
- Current license allocation and usage
- Daily license consumption trends
- Peak usage analysis by day/hour
- Top consuming indexes and source types
- License violation history

### retention-analysis.sh

Review data retention policies and identify optimization opportunities.

```bash
# Retention policy summary
./retention-analysis.sh

# Identify cost optimization candidates
./retention-analysis.sh --optimize

# Show indexes with non-default retention
./retention-analysis.sh --custom-only

# Export for documentation
./retention-analysis.sh --json > retention-report.json
```

**Analysis Includes:**
- Retention policy summary per index
- Indexes with custom retention settings
- Frozen time period analysis
- Data age vs retention comparison
- Cost optimization recommendations

## Common Workflows

### Monthly Capacity Review

```bash
#!/bin/bash
# monthly-capacity-review.sh

REPORT_DATE=$(date +%Y-%m)
OUTPUT_DIR="./capacity-reports/${REPORT_DATE}"
mkdir -p "$OUTPUT_DIR"

echo "=== Monthly Capacity Review: $REPORT_DATE ==="

# Index growth analysis
echo "Generating index growth report..."
./index-growth.sh --json > "$OUTPUT_DIR/index-growth.json"

# License usage analysis
echo "Generating license usage report..."
./license-usage.sh --json > "$OUTPUT_DIR/license-usage.json"

# Retention analysis
echo "Generating retention analysis..."
./retention-analysis.sh --json > "$OUTPUT_DIR/retention.json"

# Generate summary report
{
    echo "# Capacity Report: $REPORT_DATE"
    echo ""
    echo "## Index Growth Projections"
    cat "$OUTPUT_DIR/index-growth.json" | jq -r '.projections | to_entries[] | "- \(.key): \(.value.expected_size_gb) GB"'
    echo ""
    echo "## License Utilization"
    cat "$OUTPUT_DIR/license-usage.json" | jq -r '"Current: \(.current_usage_gb)GB / \(.license_quota_gb)GB (\(.utilization_percent)%)"'
    echo ""
    echo "## Optimization Opportunities"
    cat "$OUTPUT_DIR/retention.json" | jq -r '.recommendations[] | "- \(.index): \(.recommendation)"'
} > "$OUTPUT_DIR/summary.md"

echo "Report complete: $OUTPUT_DIR/summary.md"
```

### Pre-Purchase Planning

```bash
#!/bin/bash
# license-upgrade-analysis.sh

# Get current usage trends
current_usage=$(./license-usage.sh --json | jq '.current_usage_gb')
peak_usage=$(./license-usage.sh --json | jq '.peak_usage_30d_gb')
current_quota=$(./license-usage.sh --json | jq '.license_quota_gb')

# Get growth projections
growth_90d=$(./index-growth.sh --json | jq '.projections["90d"].growth_percent // 0')

# Calculate recommended license size
recommended=$(echo "$peak_usage * 1.3 / 1" | bc)  # 30% headroom
echo "Current License: ${current_quota}GB"
echo "Current Usage: ${current_usage}GB"
echo "Peak Usage (30d): ${peak_usage}GB"
echo "90-day Growth: ${growth_90d}%"
echo ""
echo "Recommended License Size: ${recommended}GB"
echo "Recommended Upgrade: $(( (recommended - current_quota + 49) / 50 * 50 ))GB"  # Round to 50GB
```

### Storage Forecasting

```bash
#!/bin/bash
# storage-forecast.sh <days>

FORECAST_DAYS="${1:-90}"

echo "=== Storage Forecast: ${FORECAST_DAYS} Days ==="

# Get current storage
./index-growth.sh --json | jq -r '
    .indexes | to_entries | sort_by(.value.current_size_gb) | reverse | .[:10] |
    .[] | "\(.key): \(.value.current_size_gb)GB → \(.value.projections["'"$FORECAST_DAYS"'d"].expected_size_gb // "N/A")GB"
'

# Calculate total
total_current=$(./index-growth.sh --json | jq '[.indexes[].current_size_gb] | add')
total_projected=$(./index-growth.sh --json | jq '[.indexes[].projections["'"$FORECAST_DAYS"'d"].expected_size_gb // .indexes[].current_size_gb] | add')

echo ""
echo "Total Current: ${total_current}GB"
echo "Total Projected (${FORECAST_DAYS}d): ${total_projected}GB"
echo "Additional Storage Needed: $(( total_projected - total_current ))GB"
```

## Integration with Monitoring

### Prometheus Metrics

```bash
#!/bin/bash
# Generate Prometheus metrics for license usage

OUTPUT_FILE="/var/lib/node_exporter/textfile_collector/splunk_license.prom"

# License metrics
./license-usage.sh --json | jq -r '
    "splunk_license_quota_bytes " + (.license_quota_gb * 1024 * 1024 * 1024 | tostring),
    "splunk_license_used_bytes " + (.current_usage_gb * 1024 * 1024 * 1024 | tostring),
    "splunk_license_utilization_ratio " + (.utilization_percent / 100 | tostring),
    "splunk_license_peak_30d_bytes " + (.peak_usage_30d_gb * 1024 * 1024 * 1024 | tostring)
' > "$OUTPUT_FILE"

# Index size metrics
./index-growth.sh --json | jq -r '
    .indexes | to_entries[] | 
    "splunk_index_current_size_bytes{name=\"" + .key + "\"} " + (.value.current_size_gb * 1024 * 1024 * 1024 | tostring),
    "splunk_index_event_count{name=\"" + .key + "\"} " + (.value.current_events // 0 | tostring)
' >> "$OUTPUT_FILE"
```

### Grafana Dashboard Data

```bash
#!/bin/bash
# Generate JSON for Grafana

./index-growth.sh --json | jq '{
    dashboard: {
        title: "Splunk Capacity Planning",
        panels: [
            {
                title: "Index Growth",
                targets: [.indexes | to_entries[] | {
                    target: .key,
                    datapoints: [
                        [.value.current_size_gb, now],
                        [.value.projections["30d"].expected_size_gb, now + 2592000],
                        [.value.projections["90d"].expected_size_gb, now + 7776000]
                    ]
                }]
            }
        ]
    }
}' > /var/lib/grafana/dashboards/capacity.json
```

## Cost Optimization

### Identify Cold Data

```bash
#!/bin/bash
# identify-cold-data.sh

# Find indexes with old data but low search frequency
./retention-analysis.sh --json | jq -r '
    .indexes | to_entries[] | select(.value.retention_days > 90 and .value.search_frequency == "low") |
    "\(.key): \(.value.current_size_gb)GB retained for \(.value.retention_days) days"
'
```

### Right-Size Indexes

```bash
#!/bin/bash
# right-size-indexes.sh

# Indexes using < 10% of max size may be over-provisioned
./index-growth.sh --json | jq -r '
    .indexes | to_entries[] | select(.value.utilization_percent < 10) |
    "\(.key): Using \(.value.current_size_gb)GB of \(.value.max_size_gb // "unlimited")GB (\(.value.utilization_percent)%)"
'
```

## See Also

- [Main Examples README](../README.md) - Overview of all example categories
- [Workflow Guide](../../docs/workflows.md) - Detailed workflow explanations
