# Incident Response Examples

Scripts for security incident investigation and response workflows.

## Scripts

### rapid-search.sh

Quickly search across multiple indexes for IOCs (IPs, hashes, usernames).

```bash
# Search for an IP address across all security indexes
./rapid-search.sh "192.168.1.100"

# Search with extended time window (last 72 hours)
./rapid-search.sh "malware-hash" --hours 72

# Output JSON for further processing
./rapid-search.sh "suspicious-user" --output json | jq '.[].count' | awk '{sum+=$1} END {print sum}'
```

**Searches Across:**
- Authentication logs (`index=auth` or `index=os`)
- Network traffic (`index=network` or `index=firewall`)
- Endpoint events (`index=endpoint` or `index=sysmon`)
- Proxy/web logs (`index=proxy` or `index=web`)
- Windows events (`index=windows`)

**Output:**
- Summary of hits per index
- Event counts and time ranges
- Quick triage information

### alert-investigation.sh

Investigate fired alerts and retrieve their search results.

```bash
# List alerts from last 4 hours
./alert-investigation.sh

# Investigate specific time window
./alert-investigation.sh --hours 24

# Focus on high/critical severity alerts
./alert-investigation.sh --severity high

# Export alert details to file
./alert-investigation.sh --hours 8 --output-file alerts-$(date +%Y%m%d).json
```

**Workflow:**
1. Lists all fired alerts in time window
2. Identifies high/critical severity alerts
3. Shows alert details and trigger conditions
4. Optionally retrieves underlying search results

### log-export.sh

Export logs for incident evidence preservation.

```bash
# Export last 4 hours to default directory
./log-export.sh

# Export specific timeframe
./log-export.sh --earliest "-24h" --latest "now"

# Custom output directory
./log-export.sh --earliest "-4h" --output-dir ./incident-$(date +%Y%m%d-%H%M)

# Export specific indexes only
./log-export.sh --indexes "auth,firewall,endpoint" --earliest "-1h"
```

**Exports:**
- Separate files per index
- Metadata file with search parameters
- CSV and JSON formats available
- Timestamps in filenames

## Common Workflows

### Phishing Investigation

```bash
#!/bin/bash
# investigate-phishing.sh <sender_email> <attachment_hash>

SENDER="$1"
HASH="$2"
OUTPUT_DIR="./phishing-$(date +%Y%m%d-%H%M)"

# Search for the sender
./rapid-search.sh "$SENDER" --hours 168 --output json > "$OUTPUT_DIR/sender-hits.json"

# Search for the attachment hash
./rapid-search.sh "$HASH" --hours 168 --output json > "$OUTPUT_DIR/hash-hits.json"

# Export email logs
./log-export.sh --indexes "email,o365" --earliest "-168h" --output-dir "$OUTPUT_DIR/emails"

# Check for related alerts
./alert-investigation.sh --hours 168 --severity medium > "$OUTPUT_DIR/alerts.txt"

echo "Investigation complete. Results in: $OUTPUT_DIR"
```

### Lateral Movement Detection

```bash
#!/bin/bash
# detect-lateral-movement.sh <compromised_host>

HOST="$1"
OUTPUT_DIR="./lateral-$(date +%Y%m%d-%H%M)"
mkdir -p "$OUTPUT_DIR"

# Search for authentication from compromised host
./rapid-search.sh "$HOST" --hours 24 --output json | \
    jq -r '.[] | select(.index | contains("auth")) | .sample_events[]' > "$OUTPUT_DIR/auth-events.json"

# Export network connections
./log-export.sh --indexes "network,firewall" --earliest "-24h" --output-dir "$OUTPUT_DIR/network"

# Check for new login patterns
./rapid-search.sh "*" --hours 6 --output json | \
    jq -r '.[] | select(.index | contains("auth")) | .unique_sources[]' | sort | uniq > "$OUTPUT_DIR/unique-sources.txt"

echo "Lateral movement analysis complete. Review: $OUTPUT_DIR"
```

### Ransomware Response

```bash
#!/bin/bash
# ransomware-response.sh

OUTPUT_DIR="./ransomware-$(date +%Y%m%d-%H%M)"
mkdir -p "$OUTPUT_DIR"

# Rapid search for common ransomware indicators
INDICATORS=(
    "*.encrypted"
    "README_RESTORE.txt"
    "vssadmin delete shadows"
    "wbadmin delete catalog"
)

for indicator in "${INDICATORS[@]}"; do
    echo "Searching for: $indicator"
    ./rapid-search.sh "$indicator" --hours 4 >> "$OUTPUT_DIR/indicators.txt"
done

# Export all endpoint logs
./log-export.sh --indexes "endpoint,windows,sysmon" --earliest "-4h" --output-dir "$OUTPUT_DIR/endpoint"

# Check recent high-severity alerts
./alert-investigation.sh --hours 4 --severity high > "$OUTPUT_DIR/alerts.txt"

echo "Ransomware investigation complete: $OUTPUT_DIR"
```

## Integration with SIEM/SOAR

### Splunk SOAR Playbook Integration

```python
# Python snippet for Splunk SOAR custom function
import subprocess
import json

def investigate_ioc(ioc, hours=24):
    """Run rapid-search.sh for an IOC and return structured results."""
    cmd = [
        "/path/to/splunk-tui/examples/incident-response/rapid-search.sh",
        ioc,
        "--hours", str(hours),
        "--output", "json"
    ]
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    return json.loads(result.stdout)

# Use in playbook
results = investigate_ioc(container.get('data', 'src_ip'), hours=72)
if any(r['count'] > 0 for r in results):
    # Escalate if hits found
    pass
```

### TheHive Custom Observable Analyzer

```python
# Analyzer for TheHive
#!/usr/bin/env python3
import subprocess
import json

def analyze(observable):
    """Analyze an observable using rapid-search.sh."""
    value = observable['data']
    
    cmd = [
        "/path/to/rapid-search.sh",
        value,
        "--hours", "168",
        "--output", "json"
    ]
    
    result = subprocess.run(cmd, capture_output=True, text=True)
    data = json.loads(result.stdout)
    
    total_hits = sum(r['count'] for r in data)
    
    return {
        'summary': f"Found {total_hits} hits across {len([r for r in data if r['count'] > 0])} indexes",
        'taxonomies': [{
            'level': 'malicious' if total_hits > 100 else 'suspicious' if total_hits > 0 else 'safe',
            'namespace': 'Splunk',
            'predicate': 'Hits',
            'value': str(total_hits)
        }]
    }
```

## Evidence Handling

### Chain of Custody

The `log-export.sh` script helps maintain evidence integrity:

```bash
# Export with metadata for legal proceedings
./log-export.sh \
    --earliest "2024-01-15T08:00:00" \
    --latest "2024-01-15T12:00:00" \
    --output-dir "./case-2024-001-evidence"

# The metadata.json includes:
# - Export timestamp
# - Search parameters
# - Result counts
# - User who performed export
```

### Evidence Packaging

```bash
#!/bin/bash
# package-evidence.sh <case_number>

CASE="$1"
EXPORT_DIR="./${CASE}-evidence-$(date +%Y%m%d)"

# Export logs
./log-export.sh --earliest "-24h" --output-dir "$EXPORT_DIR/logs"

# Create manifest
{
    echo "Case: $CASE"
    echo "Exported: $(date -Iseconds)"
    echo "Investigator: $(whoami)"
    echo ""
    echo "Files:"
    find "$EXPORT_DIR" -type f -exec sha256sum {} \;
} > "$EXPORT_DIR/manifest.txt"

# Create archive
tar czf "${EXPORT_DIR}.tar.gz" "$EXPORT_DIR"
echo "Evidence package: ${EXPORT_DIR}.tar.gz"
```

## See Also

- [Main Examples README](../README.md) - Overview of all example categories
- [Workflow Guide](../../docs/workflows.md) - Detailed workflow explanations
