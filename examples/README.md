# Splunk CLI Examples

This directory contains executable workflow scripts demonstrating real-world Splunk operations using `splunk-cli`. These examples serve as copy-paste starting points for your own automation and operational workflows.

## Overview

Each example script is:
- **Executable**: Ready to run with `./script-name.sh`
- **Documented**: Comprehensive header with purpose and prerequisites
- **Safe**: Dry-run mode by default where applicable
- **Portable**: Works on any system with bash and splunk-cli

## Prerequisites

### Required Tools

- `splunk-cli` installed and available in PATH
- `jq` for JSON processing (most scripts)
- `bash` 4.0+ for associative array support

### Configuration

Scripts use the same configuration as `splunk-cli`:

1. **Environment variables** (recommended for CI/CD):
   ```bash
   export SPLUNK_BASE_URL="https://your-splunk:8089"
   export SPLUNK_API_TOKEN="your-api-token"
   # or
   export SPLUNK_USERNAME="admin"
   export SPLUNK_PASSWORD="changeme"
   ```

2. **Configuration file** (`~/.config/splunk-tui/config.json`):
   ```json
   {
     "profiles": {
       "default": {
         "base_url": "https://your-splunk:8089",
         "api_token": "your-api-token"
       }
     }
   }
   ```

3. **.env file** in current directory:
   ```bash
   SPLUNK_BASE_URL=https://your-splunk:8089
   SPLUNK_API_TOKEN=your-api-token
   ```

## Quick Start

```bash
# 1. Clone or navigate to the repository
cd /path/to/splunk-tui

# 2. Set up your environment
export SPLUNK_BASE_URL="https://localhost:8089"
export SPLUNK_API_TOKEN="your-token"

# 3. Run an example
./examples/daily-ops/health-check.sh --help
./examples/daily-ops/health-check.sh
```

## Example Categories

### [Daily Operations](./daily-ops/)

Scripts for day-to-day Splunk administration tasks.

| Script | Purpose | Key Commands |
|--------|---------|--------------|
| [health-check.sh](./daily-ops/health-check.sh) | Comprehensive health check | `doctor`, `health`, `license`, `logs` |
| [disk-usage-report.sh](./daily-ops/disk-usage-report.sh) | Index disk usage reporting | `indexes list --detailed` |
| [job-cleanup.sh](./daily-ops/job-cleanup.sh) | Clean old/failed search jobs | `jobs --list`, `--delete` |

**Use Cases:**
- Morning health checks
- Disk space monitoring
- Search job housekeeping

### [Incident Response](./incident-response/)

Scripts for security incident investigation and response.

| Script | Purpose | Key Commands |
|--------|---------|--------------|
| [rapid-search.sh](./incident-response/rapid-search.sh) | Quick IOC investigation | `search` across multiple indexes |
| [alert-investigation.sh](./incident-response/alert-investigation.sh) | Investigate fired alerts | `alerts list`, `jobs --results` |
| [log-export.sh](./incident-response/log-export.sh) | Export evidence logs | `search --output-file` |

**Use Cases:**
- IOC pivoting across data sources
- Alert triage and investigation
- Evidence collection and export

### [Capacity Planning](./capacity-planning/)

Scripts for analyzing growth trends and planning capacity.

| Script | Purpose | Key Commands |
|--------|---------|--------------|
| [index-growth.sh](./capacity-planning/index-growth.sh) | Analyze index growth trends | `indexes list`, `_internal` search |
| [license-usage.sh](./capacity-planning/license-usage.sh) | License usage analysis | `license`, `license_logs` search |
| [retention-analysis.sh](./capacity-planning/retention-analysis.sh) | Review retention policies | `indexes list --detailed` |

**Use Cases:**
- Storage growth forecasting
- License utilization monitoring
- Retention policy compliance

### [Security Auditing](./security-auditing/)

Scripts for security compliance and auditing.

| Script | Purpose | Key Commands |
|--------|---------|--------------|
| [login-tracking.sh](./security-auditing/login-tracking.sh) | Track login anomalies | `search` for `_audit` events |
| [permission-review.sh](./security-auditing/permission-review.sh) | Review user permissions | `users list`, `roles list` |
| [config-changes.sh](./security-auditing/config-changes.sh) | Track config changes | `audit list`, `_audit` search |

**Use Cases:**
- Authentication monitoring
- Access control reviews
- Change management auditing

### [Automation](./automation/)

Scripts for CI/CD integration and scheduled tasks.

| Script | Purpose | Key Commands |
|--------|---------|--------------|
| [scheduled-reports.sh](./automation/scheduled-reports.sh) | Generate scheduled reports | `saved-searches run`, `search` |
| [bulk-operations.sh](./automation/bulk-operations.sh) | Batch resource operations | `saved-searches disable/enable` |
| [data-onboarding.sh](./automation/data-onboarding.sh) | Automate data onboarding | `indexes create`, `hec send` |

**Use Cases:**
- Automated report generation
- Bulk configuration changes
- New data source onboarding

## Common Patterns

### Safety First (Dry-Run Mode)

Most destructive scripts default to dry-run mode:

```bash
# Show what would be cleaned (safe)
./examples/daily-ops/job-cleanup.sh --older-than 48

# Actually perform cleanup (requires --execute)
./examples/daily-ops/job-cleanup.sh --older-than 48 --execute
```

### JSON Output for Piping

Many scripts support JSON output for integration with `jq`:

```bash
./examples/daily-ops/health-check.sh --json | jq '.checks[] | select(.status != "ok")'
```

### Time Range Selection

Incident response scripts typically support time ranges:

```bash
# Search last 4 hours (default)
./examples/incident-response/rapid-search.sh "192.168.1.100"

# Search last 72 hours
./examples/incident-response/rapid-search.sh "192.168.1.100" --hours 72
```

### Cron Integration

Schedule reports using cron:

```bash
# Add to crontab
0 8 * * * /path/to/splunk-tui/examples/automation/scheduled-reports.sh \
  --report "daily-summary" \
  --output-dir /var/reports/splunk/
```

## Script Conventions

### Header Structure

All scripts follow a consistent header format:

```bash
#!/usr/bin/env bash
# Brief description
#
# RESPONSIBILITY:
#   What this script does
#
# DOES NOT:
#   What this script explicitly avoids
#
# PREREQUISITES:
#   - Required tools and configuration
#
# USAGE:
#   ./script-name.sh [options]
```

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error or check failed |
| 2 | No data found or no action needed |
| 130 | Cancelled by user (Ctrl+C) |

### Color Support

All scripts respect the `NO_COLOR` environment variable:

```bash
# Disable colored output
NO_COLOR=1 ./examples/daily-ops/health-check.sh
```

## Customizing Examples

These scripts are starting points. Common customizations:

1. **Adjust thresholds**: Modify default thresholds (e.g., `--threshold 90` for disk usage)
2. **Add indexes**: Update default index lists in search scripts
3. **Change formats**: Use `--output json` for machine-readable output
4. **Add notifications**: Pipe results to email/Slack/webhook scripts

## Troubleshooting

### "splunk-cli: command not found"

Install splunk-cli and ensure it's in your PATH:

```bash
# Build and install
make release

# Verify installation
which splunk-cli
splunk-cli --version
```

### "jq: command not found"

Install jq for JSON processing:

```bash
# macOS
brew install jq

# Ubuntu/Debian
sudo apt-get install jq

# RHEL/CentOS
sudo yum install jq
```

### Authentication Errors

Scripts check for required environment variables. Set them explicitly:

```bash
export SPLUNK_BASE_URL="https://your-splunk:8089"
export SPLUNK_API_TOKEN="your-api-token"
```

Or use a configuration file:

```bash
splunk-cli config init  # Interactive setup
```

## Contributing

When adding new examples:

1. Follow the existing script header pattern
2. Include `show_help()` and `check_prerequisites()` functions
3. Use `set -euo pipefail` for error handling
4. Respect `NO_COLOR` for color output
5. Default to dry-run for destructive operations
6. Add the script to the appropriate category README
7. Update this main README with the new script

## Related Documentation

- [Workflow Guide](../docs/workflows.md) - Detailed workflow explanations
- [Usage Guide](../docs/usage.md) - Complete CLI reference
- [User Guide](../docs/user-guide.md) - TUI documentation

## License

These example scripts are provided under the same license as the splunk-tui project.
