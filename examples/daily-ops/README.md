# Daily Operations Examples

Scripts for day-to-day Splunk administration and monitoring tasks.

## Scripts

### health-check.sh

Comprehensive health check workflow covering all major Splunk subsystems.

```bash
# Run full health check
./health-check.sh

# Output as JSON for monitoring systems
./health-check.sh --json
```

**Checks Performed:**
- Server connectivity and response time
- License usage and expiration
- KVStore status
- Cluster health (if applicable)
- Recent internal errors (last hour)
- Index availability

**Output:**
- Color-coded status indicators
- Detailed check results
- Summary of issues found

### disk-usage-report.sh

Generate disk usage reports for Splunk indexes with trending.

```bash
# Standard report
./disk-usage-report.sh

# Custom threshold (warn at 70% instead of 80%)
./disk-usage-report.sh --threshold 70

# Show only top 10 indexes by size
./disk-usage-report.sh --top 10
```

**Reports:**
- Per-index size (current, max, utilization %)
- Total disk usage across all indexes
- Indexes approaching size limits (above threshold)
- Size change indicators (if historical data available)

### job-cleanup.sh

Safely clean up old or failed search jobs.

```bash
# Show what would be cleaned (dry-run, default)
./job-cleanup.sh --older-than 48

# Clean jobs by status
./job-cleanup.sh --status failed --older-than 24

# Actually perform cleanup
./job-cleanup.sh --older-than 48 --execute

# Force cleanup without confirmation
./job-cleanup.sh --older-than 72 --execute --force
```

**Safety Features:**
- Dry-run mode by default
- Confirmation prompt before deletion
- `--force` flag for automation
- Filters by age and status

## Common Workflows

### Morning Health Check Routine

```bash
#!/bin/bash
# morning-check.sh - Add to cron or run manually

cd /path/to/splunk-tui/examples

# Run health check, capture issues
if ! ./daily-ops/health-check.sh --json | jq -e '.summary.issues == 0' > /dev/null; then
    echo "Health check found issues - review required"
    ./daily-ops/health-check.sh
fi

# Check disk usage
./daily-ops/disk-usage-report.sh --threshold 85

# Clean up old jobs
./daily-ops/job-cleanup.sh --older-than 24 --execute --force
```

### Weekly Maintenance

```bash
#!/bin/bash
# weekly-maintenance.sh

cd /path/to/splunk-tui/examples

# Aggressive cleanup of failed jobs
./daily-ops/job-cleanup.sh --status failed --older-than 1 --execute --force

# Full disk report
./daily-ops/disk-usage-report.sh --threshold 75

# Generate health report for documentation
./daily-ops/health-check.sh --json > /var/log/splunk/health-$(date +%Y%m%d).json
```

## Integration with Monitoring

### Nagios/Icinga Check

```bash
#!/bin/bash
# check_splunk_health.sh - Nagios plugin wrapper

OUTPUT=$(/path/to/splunk-tui/examples/daily-ops/health-check.sh --json)
ISSUES=$(echo "$OUTPUT" | jq '.summary.issues')

if [ "$ISSUES" -eq 0 ]; then
    echo "OK: All health checks passed"
    exit 0
elif [ "$ISSUES" -lt 3 ]; then
    echo "WARNING: $ISSUES health check(s) failed"
    exit 1
else
    echo "CRITICAL: $ISSUES health check(s) failed"
    exit 2
fi
```

### Prometheus Node Exporter Textfile

```bash
#!/bin/bash
# Generate metrics for node_exporter textfile collector

cd /path/to/splunk-tui/examples
./daily-ops/disk-usage-report.sh --json | jq -r '
    .indexes[] | 
    "splunk_index_size_bytes{name=\"\(.name)\"} \(.current_size // 0)
    splunk_index_max_size_bytes{name=\"\(.name)\"} \(.max_size // 0)
    splunk_index_usage_percent{name=\"\(.name)\"} \(.utilization // 0)"
' > /var/lib/node_exporter/textfile_collector/splunk_index_usage.prom
```

## See Also

- [Main Examples README](../README.md) - Overview of all example categories
- [Workflow Guide](../../docs/workflows.md) - Detailed workflow explanations
