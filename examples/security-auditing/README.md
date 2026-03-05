# Security Auditing Examples

Scripts for security compliance, access reviews, and configuration change monitoring.

## Scripts

### login-tracking.sh

Track login activity and detect authentication anomalies.

```bash
# Analyze last 24 hours of login activity
./login-tracking.sh

# Extended analysis (last 7 days)
./login-tracking.sh --hours 168

# Focus on failed attempts
./login-tracking.sh --focus failed

# Output detailed JSON for SIEM integration
./login-tracking.sh --json > login-audit-$(date +%Y%m%d).json
```

**Analysis Includes:**
- Login success/failure counts by user
- Failed login attempts by source IP
- Off-hours login detection
- New source IP identification
- Account lockout events
- Geographic anomalies (if geo data available)

### permission-review.sh

Review user permissions and roles for access control audits.

```bash
# Full permission review
./permission-review.sh

# Focus on admin privileges
./permission-review.sh --role admin

# Check for inactive accounts
./permission-review.sh --inactive-days 60

# Export for compliance documentation
./permission-review.sh --json > permissions-$(date +%Y%m%d).json
```

**Reviews:**
- All users and their assigned roles
- Role capabilities and inheritance
- Users with admin/power privileges
- Inactive accounts (configurable threshold)
- Privilege escalation paths
- Service accounts vs human users

### config-changes.sh

Track configuration changes for compliance auditing.

```bash
# Show recent config changes (last 24h)
./config-changes.sh

# Extended window
./config-changes.sh --hours 168

# Group changes by user
./config-changes.sh --by-user

# Show only critical changes (users, roles, auth)
./config-changes.sh --critical-only

# Export for compliance evidence
./config-changes.sh --json > config-changes-$(date +%Y%m%d).json
```

**Tracks:**
- User/role modifications
- Authentication setting changes
- Index configuration changes
- App installations/enables/disables
- Saved search modifications
- Critical conf file changes

## Common Workflows

### Quarterly Access Review

```bash
#!/bin/bash
# quarterly-access-review.sh

REPORT_DATE=$(date +%Y-%m)
OUTPUT_DIR="./access-reviews/${REPORT_DATE}"
mkdir -p "$OUTPUT_DIR"

echo "=== Quarterly Access Review: $REPORT_DATE ==="

# Generate permission report
echo "Generating permission review..."
./permission-review.sh --json > "$OUTPUT_DIR/permissions.json"

# Check for inactive accounts
echo "Identifying inactive accounts..."
./permission-review.sh --inactive-days 90 --json > "$OUTPUT_DIR/inactive-accounts.json"

# Review admin access
echo "Reviewing admin access..."
./permission-review.sh --role admin --json > "$OUTPUT_DIR/admin-access.json"

# Generate summary
{
    echo "# Quarterly Access Review: $REPORT_DATE"
    echo ""
    echo "## Summary"
    echo "- Total Users: $(jq '.users | length' "$OUTPUT_DIR/permissions.json")"
    echo "- Admin Users: $(jq '[.users[] | select(.roles | contains(["admin"]))] | length' "$OUTPUT_DIR/permissions.json")"
    echo "- Inactive Accounts (>90d): $(jq '.users | length' "$OUTPUT_DIR/inactive-accounts.json")"
    echo ""
    echo "## Recommendations"
    echo "Review inactive accounts for potential deactivation:"
    jq -r '.users[] | "- \(.name) (last login: \(.last_login // "never"))"' "$OUTPUT_DIR/inactive-accounts.json"
} > "$OUTPUT_DIR/summary.md"

echo "Review complete: $OUTPUT_DIR/summary.md"
```

### SOC 2 Compliance Monitoring

```bash
#!/bin/bash
# soc2-daily-check.sh

OUTPUT_DIR="/var/log/compliance/$(date +%Y/%m/%d)"
mkdir -p "$OUTPUT_DIR"

# Monitor login anomalies
echo "Checking login anomalies..."
./login-tracking.sh --hours 24 --json > "$OUTPUT_DIR/login-audit.json"

# Track config changes
echo "Tracking configuration changes..."
./config-changes.sh --hours 24 --json > "$OUTPUT_DIR/config-changes.json"

# Alert on findings
FAILED_LOGINS=$(jq '.failed_login_count // 0' "$OUTPUT_DIR/login-audit.json")
CONFIG_CHANGES=$(jq '.changes | length' "$OUTPUT_DIR/config-changes.json")

if [ "$FAILED_LOGINS" -gt 100 ]; then
    echo "ALERT: High number of failed logins: $FAILED_LOGINS"
    # Send alert to SOC
fi

if [ "$CONFIG_CHANGES" -gt 0 ]; then
    echo "INFO: $CONFIG_CHANGES configuration changes detected"
    # Log for review
fi
```

### Insider Threat Detection

```bash
#!/bin/bash
# insider-threat-detection.sh

OUTPUT_DIR="./threat-hunt-$(date +%Y%m%d-%H%M)"
mkdir -p "$OUTPUT_DIR"

# Unusual login patterns
echo "Checking for unusual login patterns..."
./login-tracking.sh --hours 72 --focus off-hours --json > "$OUTPUT_DIR/off-hours-logins.json"

# Privileged access changes
echo "Checking for privilege changes..."
./config-changes.sh --hours 72 --critical-only --json > "$OUTPUT_DIR/privilege-changes.json"

# Large data exports (via saved search modifications)
echo "Checking for suspicious saved search changes..."
./config-changes.sh --hours 72 | grep -i "saved.*search" > "$OUTPUT_DIR/saved-search-changes.txt"

# Generate alert if suspicious activity found
OFF_HOURS=$(jq '.off_hours_logins | length' "$OUTPUT_DIR/off-hours-logins.json")
PRIV_CHANGES=$(jq '.changes | length' "$OUTPUT_DIR/privilege-changes.json")

if [ "$OFF_HOURS" -gt 5 ] || [ "$PRIV_CHANGES" -gt 0 ]; then
    echo "Potential insider threat activity detected!"
    echo "Off-hours logins: $OFF_HOURS"
    echo "Privilege changes: $PRIV_CHANGES"
    # Trigger investigation workflow
fi
```

## Compliance Frameworks

### PCI DSS

Requirements covered:
- **Req 8.1.6**: Limit repeated access attempts (`login-tracking.sh`)
- **Req 8.1.8**: Set idle timeout (`permission-review.sh`)
- **Req 10.2.5**: Use and changes to identification mechanisms (`config-changes.sh`)
- **Req 10.2.6**: Initialization, stopping, or pausing of audit logs (`config-changes.sh`)

```bash
#!/bin/bash
# pci-dss-daily-audit.sh

# Failed login monitoring (Req 8.1.6)
./login-tracking.sh --focus failed --hours 24

# Account review (Req 8.1.8)
./permission-review.sh --inactive-days 90

# Configuration change audit (Req 10.2.x)
./config-changes.sh --hours 24 --critical-only
```

### HIPAA

Requirements covered:
- **164.308(a)(3)**: Workforce security (`permission-review.sh`)
- **164.308(a)(4)**: Information access management (`permission-review.sh`)
- **164.312(b)**: Audit controls (`config-changes.sh`, `login-tracking.sh`)

```bash
#!/bin/bash
# hipaa-weekly-audit.sh

# Access review
./permission-review.sh --json > hipaa-access-$(date +%Y%m%d).json

# Login audit
./login-tracking.sh --hours 168 --json > hipaa-logins-$(date +%Y%m%d).json

# Change audit
./config-changes.sh --hours 168 --json > hipaa-changes-$(date +%Y%m%d).json
```

### SOX

Requirements covered:
- **IT General Controls**: Access management (`permission-review.sh`)
- **Change Management**: Configuration tracking (`config-changes.sh`)

```bash
#!/bin/bash
# sox-quarterly-review.sh

# Privileged access review
./permission-review.sh --role admin

# Change management evidence
./config-changes.sh --hours 2160 --by-user  # 90 days
```

## Integration with SIEM

### Splunk ES Correlation

```python
# Custom correlation search for Splunk ES
import subprocess
import json

def check_login_anomalies():
    """Run login tracking and return anomalies."""
    result = subprocess.run(
        ["/path/to/login-tracking.sh", "--json", "--hours", "4"],
        capture_output=True, text=True
    )
    data = json.loads(result.stdout)
    
    anomalies = []
    for login in data.get('off_hours_logins', []):
        anomalies.append({
            'user': login['user'],
            'time': login['time'],
            'source': login['source'],
            'risk_score': 50
        })
    return anomalies
```

### QRadar Integration

```bash
#!/bin/bash
# Send audit events to QRadar

# Convert login tracking to LEEF format
./login-tracking.sh --json | jq -r '
    .failed_logins[] |
    "LEEF:2.0|Splunk|splunk-cli|1.0|FailedLogin|usrName=\(.user)\tsrc=\(.source)\teventCount=\(.count)"
' | while read leef; do
    echo "$leef" | nc qradar-collector 514
done
```

## See Also

- [Main Examples README](../README.md) - Overview of all example categories
- [Workflow Guide](../../docs/workflows.md) - Detailed workflow explanations
