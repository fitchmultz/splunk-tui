# Workflow Examples

This guide provides comprehensive workflow examples using `splunk-cli` and the example scripts in the `examples/` directory.

## Table of Contents

1. [Daily Operations Workflows](#daily-operations-workflows)
2. [Incident Response Workflows](#incident-response-workflows)
3. [Capacity Planning Workflows](#capacity-planning-workflows)
4. [Security Auditing Workflows](#security-auditing-workflows)
5. [Automation Workflows](#automation-workflows)
6. [Integration Patterns](#integration-patterns)

---

## Daily Operations Workflows

### Morning Health Check

A typical morning health check ensures your Splunk environment is running optimally.

#### Using the Health Check Script

```bash
# Run comprehensive health check
./examples/daily-ops/health-check.sh

# For monitoring systems, use JSON output
./examples/daily-ops/health-check.sh --json | jq '.checks[] | select(.status != "ok")'
```

**What It Checks:**
- Server connectivity and response time
- License usage and expiration status
- KVStore status
- Cluster health (if clustered)
- Recent internal errors

**Interpreting Results:**
- ✓ Green: Component is healthy
- ⚠ Yellow: Warning (investigate soon)
- ✗ Red: Critical issue (immediate attention required)

#### Manual Health Check

If you prefer to check components individually:

```bash
# Quick connectivity check
splunk-cli doctor

# Server health
splunk-cli health

# License status
splunk-cli license

# Recent errors (last hour)
splunk-cli logs --count 20 --earliest "-1h"
```

[See full script: examples/daily-ops/health-check.sh](../examples/daily-ops/health-check.sh)

### Disk Usage Monitoring

Regular disk monitoring prevents unexpected storage issues.

#### Weekly Disk Report

```bash
# Standard report
./examples/daily-ops/disk-usage-report.sh

# With custom threshold (warn at 70%)
./examples/daily-ops/disk-usage-report.sh --threshold 70

# Top 10 largest indexes
./examples/daily-ops/disk-usage-report.sh --top 10
```

**Understanding the Report:**
- **Current Size**: Actual data stored
- **Max Size**: Configured limit
- **Utilization**: Percentage used
- **Trend**: Growth indicator (if historical data available)

#### Taking Action on Full Indexes

When an index approaches capacity:

```bash
# Check specific index details
splunk-cli indexes list --detailed | grep -A 20 "my_index"

# Reduce retention (example: reduce from 365 to 180 days)
splunk-cli indexes modify my_index --frozen-time-period-secs 15552000

# Or increase max size
splunk-cli indexes modify my_index --max-data-size-mb 50000
```

[See full script: examples/daily-ops/disk-usage-report.sh](../examples/daily-ops/disk-usage-report.sh)

### Search Job Cleanup

Managing search jobs prevents resource exhaustion.

#### Safe Cleanup Workflow

```bash
# Always dry-run first to see what would be cleaned
./examples/daily-ops/job-cleanup.sh --older-than 24 --status done

# Review the output, then execute
./examples/daily-ops/job-cleanup.sh --older-than 24 --status done --execute

# For automation, use --force to skip confirmation
./examples/daily-ops/job-cleanup.sh --older-than 48 --status failed --execute --force
```

**Cleanup Strategy:**
- **Done jobs > 24h**: Safe to delete
- **Failed jobs > 1h**: Usually safe to delete
- **Running jobs**: Never auto-delete; investigate why they're stuck
- **Paused jobs**: Review before deleting

[See full script: examples/daily-ops/job-cleanup.sh](../examples/daily-ops/job-cleanup.sh)

---

## Incident Response Workflows

### Rapid IOC Investigation

When investigating a security incident, speed is critical.

#### Quick IOC Pivot

```bash
# Search for an IP across all security indexes
./examples/incident-response/rapid-search.sh "192.168.1.100"

# Extended search (last 72 hours)
./examples/incident-response/rapid-search.sh "192.168.1.100" --hours 72

# Output JSON for further processing
./examples/incident-response/rapid-search.sh "malware-sha256-hash" --output json | \
    jq -r '.[] | select(.count > 0) | "\(.index): \(.count) events"'
```

**Investigation Steps:**
1. **Start broad**: Search the IOC across all indexes
2. **Identify hot indexes**: Note which indexes have hits
3. **Drill down**: Run targeted searches in relevant indexes
4. **Time correlation**: Check when activity occurred
5. **Expand scope**: Look for related IOCs

#### Manual IOC Search

```bash
# Search specific indexes
splunk-cli search 'index=firewall src_ip=192.168.1.100' --wait --earliest "-72h"
splunk-cli search 'index=proxy dest_ip=192.168.1.100' --wait --earliest "-72h"
splunk-cli search 'index=auth src=192.168.1.100' --wait --earliest "-72h"
```

[See full script: examples/incident-response/rapid-search.sh](../examples/incident-response/rapid-search.sh)

### Alert Investigation

When an alert fires, follow a structured investigation process.

#### Alert Triage Process

```bash
# List recent alerts
./examples/incident-response/alert-investigation.sh --hours 4

# Focus on high/critical severity
./examples/incident-response/alert-investigation.sh --severity high

# Export for documentation
./examples/incident-response/alert-investigation.sh --hours 24 --output-file alerts-$(date +%Y%m%d).json
```

**Triage Questions:**
1. Is this a known/false positive pattern?
2. What triggered the alert?
3. Are there related alerts?
4. What's the potential impact?
5. Who should be notified?

#### Retrieving Alert Results

```bash
# Get the SID from alert info
splunk-cli alerts info "scheduler__admin__search__MyAlert_at_1705852800_123"

# Retrieve the search results
splunk-cli jobs --results 1705852800.123 --result-count 100

# Export to file for analysis
splunk-cli jobs --results 1705852800.123 --output-file alert-results.json
```

[See full script: examples/incident-response/alert-investigation.sh](../examples/incident-response/alert-investigation.sh)

### Evidence Collection

Preserve evidence for forensic analysis or legal proceedings.

#### Exporting Incident Data

```bash
# Export specific timeframe
./examples/incident-response/log-export.sh \
    --earliest "2024-01-15T08:00:00" \
    --latest "2024-01-15T12:00:00" \
    --output-dir ./incident-2024-01-15/

# Export specific indexes
./examples/incident-response/log-export.sh \
    --indexes "firewall,proxy,endpoint" \
    --earliest "-4h" \
    --output-dir ./evidence/
```

**Evidence Integrity:**
- Always document the export time and search parameters
- Calculate and record checksums of exported files
- Maintain chain of custody documentation
- Store in a secure, tamper-evident location

[See full script: examples/incident-response/log-export.sh](../examples/incident-response/log-export.sh)

---

## Capacity Planning Workflows

### Storage Growth Analysis

Understanding growth trends helps prevent capacity issues.

#### Monthly Growth Review

```bash
# Generate growth analysis
./examples/capacity-planning/index-growth.sh

# Detailed per-index breakdown
./examples/capacity-planning/index-growth.sh --detailed

# Custom projection periods
./examples/capacity-planning/index-growth.sh --project 30,60,90,180
```

**Key Metrics:**
- **Daily Ingestion Rate**: GB per day added
- **Growth Trend**: Increasing, decreasing, or stable
- **Time to Full**: Days until index reaches max size
- **Projected Sizes**: Expected sizes at 30/60/90 days

#### Capacity Planning Decision Tree

1. **Is growth > 20% month-over-month?**
   - Investigate unexpected data sources
   - Check for misconfigured forwarders
   - Review retention policies

2. **Are indexes > 80% full?**
   - Plan storage expansion
   - Reduce retention if appropriate
   - Consider data tiering

3. **Is license utilization > 80%?**
   - Review license allocation
   - Optimize data ingestion
   - Plan license upgrade

[See full script: examples/capacity-planning/index-growth.sh](../examples/capacity-planning/index-growth.sh)

### License Management

Proactive license management prevents violations.

#### Weekly License Check

```bash
# Current usage snapshot
./examples/capacity-planning/license-usage.sh

# Detailed daily breakdown
./examples/capacity-planning/license-usage.sh --detailed

# Top consumers
./examples/capacity-planning/license-usage.sh --top 20
```

**Understanding License Usage:**
- **Daily Volume**: Raw data ingested per day
- **Peak Usage**: Highest daily volume in period
- **License Quota**: Daily indexed data allowance
- **Utilization %**: Peak / Quota * 100

**When to Upgrade:**
- Consistent usage > 80% of quota
- Multiple warnings in 30 days
- Business growth requiring more data
- New data sources coming online

[See full script: examples/capacity-planning/license-usage.sh](../examples/capacity-planning/license-usage.sh)

### Retention Policy Review

Optimize storage costs through effective retention policies.

#### Retention Analysis

```bash
# Full retention report
./examples/capacity-planning/retention-analysis.sh

# Find optimization opportunities
./examples/capacity-planning/retention-analysis.sh --optimize

# Custom retention only
./examples/capacity-planning/retention-analysis.sh --custom-only
```

**Optimization Opportunities:**
- Indexes with no retention limit (unbounded growth)
- High-volume indexes with long retention
- Indexes with search frequency mismatch (cold data)
- Duplicate data across indexes

[See full script: examples/capacity-planning/retention-analysis.sh](../examples/capacity-planning/retention-analysis.sh)

---

## Security Auditing Workflows

### Login Activity Monitoring

Detect authentication anomalies and potential breaches.

#### Daily Login Review

```bash
# Standard 24-hour review
./examples/security-auditing/login-tracking.sh

# Extended analysis (7 days)
./examples/security-auditing/login-tracking.sh --hours 168

# Focus on failed attempts
./examples/security-auditing/login-tracking.sh --focus failed
```

**Warning Signs:**
- Multiple failed logins from single IP
- Successful logins after many failures
- Off-hours access by day-shift users
- Logins from unexpected countries/locations
- Service accounts used interactively

[See full script: examples/security-auditing/login-tracking.sh](../examples/security-auditing/login-tracking.sh)

### Access Control Review

Regular access reviews ensure least-privilege compliance.

#### Quarterly Access Review

```bash
# Full permission review
./examples/security-auditing/permission-review.sh

# Focus on admin users
./examples/security-auditing/permission-review.sh --role admin

# Identify stale accounts
./examples/security-auditing/permission-review.sh --inactive-days 90
```

**Review Checklist:**
- [ ] All admin accesses justified and documented
- [ ] Former employees have no access
- [ ] Role assignments match job functions
- [ ] Service accounts are clearly identified
- [ ] Privileged accounts have MFA enabled
- [ ] No shared accounts (except documented service accounts)

[See full script: examples/security-auditing/permission-review.sh](../examples/security-auditing/permission-review.sh)

### Configuration Change Tracking

Monitor changes for compliance and security.

#### Daily Change Review

```bash
# Recent changes (24 hours)
./examples/security-auditing/config-changes.sh

# Extended window (7 days)
./examples/security-auditing/config-changes.sh --hours 168

# Group by user
./examples/security-auditing/config-changes.sh --by-user

# Critical changes only
./examples/security-auditing/config-changes.sh --critical-only
```

**Critical Changes to Monitor:**
- User/role additions or modifications
- Authentication configuration changes
- SSL/certificate changes
- Audit logging configuration changes
- Index deletion
- App installation from untrusted sources

[See full script: examples/security-auditing/config-changes.sh](../examples/security-auditing/config-changes.sh)

---

## Automation Workflows

### Scheduled Reporting

Automate routine report generation.

#### Daily Report Generation

```bash
# Run a saved search and save results
./examples/automation/scheduled-reports.sh \
    --report "Daily Error Summary" \
    --output-dir /var/reports/splunk/ \
    --format csv

# Multiple reports from a list
./examples/automation/scheduled-reports.sh \
    --report-list daily-reports.txt \
    --output-dir /var/reports/splunk/
```

**Cron Setup:**

```bash
# Add to crontab
0 8 * * * /path/to/splunk-tui/examples/automation/scheduled-reports.sh \
    --report "Daily Summary" \
    --output-dir /var/reports/splunk/$(date +\%Y/\%m/\%d)
```

[See full script: examples/automation/scheduled-reports.sh](../examples/automation/scheduled-reports.sh)

### Bulk Operations

Safely perform batch operations.

#### Cleanup Workflow

```bash
# Create list of items to process
splunk-cli saved-searches list --output json | \
    jq -r '.[] | select(.updated | fromdateiso8601 < (now - 7776000)) | .name' > old-searches.txt

# Preview operations (dry-run)
./examples/automation/bulk-operations.sh \
    --operation disable-searches \
    --file old-searches.txt

# Execute after review
./examples/automation/bulk-operations.sh \
    --operation disable-searches \
    --file old-searches.txt \
    --execute
```

**Safety First:**
- Always dry-run before executing
- Review the list of affected items
- Have a rollback plan
- Document the change

[See full script: examples/automation/bulk-operations.sh](../examples/automation/bulk-operations.sh)

### Data Onboarding

Streamline new data source onboarding.

#### Complete Onboarding Workflow

```bash
# Full onboarding with validation
./examples/automation/data-onboarding.sh \
    --index new_application \
    --sourcetype newapp:logs \
    --retention 90 \
    --hec \
    --validate

# Creates:
# - Index with specified retention
# - HEC token (if --hec specified)
# - Validation test event
# - Monitoring saved searches
```

**Onboarding Checklist:**
- [ ] Index created with appropriate retention
- [ ] Sourcetype defined in props.conf
- [ ] Parsing verified with sample data
- [ ] CIM compliance checked (if applicable)
- [ ] Monitoring alerts configured
- [ ] Documentation updated
- [ ] Handoff to operations team

[See full script: examples/automation/data-onboarding.sh](../examples/automation/data-onboarding.sh)

---

## Integration Patterns

### CI/CD Pipeline Integration

```bash
#!/bin/bash
# ci-spl-validation.sh

# Validate SPL files before deployment
for file in queries/*.spl; do
    echo "Validating: $file"
    if ! splunk-cli search validate --file "$file"; then
        echo "SPL validation failed: $file"
        exit 1
    fi
done

# Run health check before deployment
if ! ./examples/daily-ops/health-check.sh --json | jq -e '.summary.issues == 0' > /dev/null; then
    echo "Splunk health check failed - aborting deployment"
    exit 1
fi

echo "All checks passed - safe to deploy"
```

### Monitoring Integration

```bash
#!/bin/bash
# nagios-splunk-check.sh

# Health check for Nagios/Icinga
OUTPUT=$(./examples/daily-ops/health-check.sh --json)
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

### SOAR Integration

```python
# Example Python wrapper for SOAR platforms
def investigate_ioc(ioc_value, hours=24):
    """Run rapid search for an IOC."""
    import subprocess
    import json
    
    result = subprocess.run(
        ["./examples/incident-response/rapid-search.sh", 
         ioc_value, "--hours", str(hours), "--output", "json"],
        capture_output=True, text=True
    )
    
    return json.loads(result.stdout)
```

---

## Best Practices Summary

1. **Always dry-run first** when using scripts that modify data
2. **Use JSON output** for integration with other tools
3. **Set up monitoring** for critical health checks
4. **Document your workflows** for team consistency
5. **Review and test** scripts in non-production first
6. **Respect rate limits** when running frequent queries
7. **Secure credentials** using environment variables or config files
8. **Version control** your custom scripts and queries

## Additional Resources

- [Examples README](../examples/README.md) - All example scripts
- [Usage Guide](./usage.md) - Complete CLI reference
- [User Guide](./user-guide.md) - TUI documentation
