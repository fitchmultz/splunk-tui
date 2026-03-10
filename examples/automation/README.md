# Automation Examples

Scripts for CI/CD integration, scheduled tasks, and bulk operations.

## Scripts

### scheduled-reports.sh

Generate and save scheduled reports from saved searches.

```bash
# Run a saved search and save results
./scheduled-reports.sh --report "Daily Error Summary" --output-dir /var/reports/splunk/

# Run with specific format
./scheduled-reports.sh --report "Security Events" --format csv --output-dir ./reports/

# Run another report to a separate directory
./scheduled-reports.sh --report "License Usage Daily" --output-dir /var/reports/licenses/
```

**Features:**
- Timestamped output filenames
- Multiple output formats (json, csv)
- Saved search execution
- Output directories created on demand
- Uses the supported `saved-searches run --wait` flow under the hood

### bulk-operations.sh

Perform bulk operations on Splunk resources safely.

```bash
# Preview operations (dry-run, default)
./bulk-operations.sh --operation disable-searches --file old-searches.txt

# Execute the operations
./bulk-operations.sh --operation disable-searches --file old-searches.txt --execute

# Available operations:
# - disable-searches / enable-searches / delete-searches

# Execute deletions after review
./bulk-operations.sh --operation delete-searches --file obsolete.txt --execute
```

**Safety Features:**
- Dry-run mode by default
- Explicit `--execute` requirement before making changes
- Progress reporting
- Per-search existence checks use `saved-searches info`, not table parsing

### data-onboarding.sh

Automate the complete data onboarding workflow.

```bash
# Basic onboarding
./data-onboarding.sh --index newapp --sourcetype newapp:logs

# Full onboarding with HEC
./data-onboarding.sh --index newapp --sourcetype newapp:logs --hec

# Skip the built-in ingestion validation step
./data-onboarding.sh --index newapp --sourcetype newapp:logs --skip-validation
```

**Workflow:**
1. Create index (if not exists)
2. Configure HEC input (if --hec specified)
3. Send test event for validation
4. Create monitoring saved searches
5. Print next-step guidance for manual source configuration

## Common Workflows

### Daily Reporting Pipeline

```bash
#!/bin/bash
# daily-reports.sh - Run from cron at 8 AM

REPORT_DIR="/var/reports/splunk/$(date +%Y/%m/%d)"
mkdir -p "$REPORT_DIR"

# Security reports
./scheduled-reports.sh \
    --report "Failed Logins Summary" \
    --output-dir "$REPORT_DIR" \
    --format csv

./scheduled-reports.sh \
    --report "High Severity Alerts" \
    --output-dir "$REPORT_DIR" \
    --format json

# Performance reports
./scheduled-reports.sh \
    --report "License Usage Daily" \
    --output-dir "$REPORT_DIR" \
    --format csv

# Optional: hand off generated files to your own mail or archive tooling here
```

### CI/CD SPL Validation

```bash
#!/bin/bash
# validate-spl-in-ci.sh

# Run in CI pipeline to validate all SPL files
FAILED=0

for spl_file in queries/*.spl; do
    echo "Validating: $spl_file"
    if ! splunk-cli search validate --file "$spl_file"; then
        echo "FAILED: $spl_file"
        FAILED=1
    fi
done

exit $FAILED
```

### Automated Index Maintenance

```bash
#!/bin/bash
# monthly-index-maintenance.sh

# Create list of old saved searches to review
splunk-cli saved-searches list --output json | \
    jq -r '.[] | select(.updated | fromdateiso8601 < (now - 2592000)) | .name' > /tmp/old-searches.txt

# Disable searches not updated in 30 days (dry-run first)
echo "Searches to disable:"
cat /tmp/old-searches.txt

# After review, execute
./bulk-operations.sh \
    --operation disable-searches \
    --file /tmp/old-searches.txt \
    --execute

# Review or script lookup cleanup separately; bulk-operations.sh only handles saved searches
```

### New Application Onboarding

```bash
#!/bin/bash
# onboard-new-app.sh <app_name> <sourcetype>

APP_NAME="$1"
SOURCETYPE="$2"
if [ -z "$APP_NAME" ] || [ -z "$SOURCETYPE" ]; then
    echo "Usage: $0 <app_name> <sourcetype>"
    exit 1
fi

# Run onboarding workflow
./data-onboarding.sh \
    --index "$APP_NAME" \
    --sourcetype "$SOURCETYPE" \
    --hec

# Export a standard saved-search report
./scheduled-reports.sh \
    --report "${APP_NAME}_monitor_volume" \
    --output-dir "/opt/splunk-saved-searches"

echo "Onboarding complete for $APP_NAME"
echo "HEC endpoint: https://your-splunk:8088"
echo "Index: $APP_NAME"
echo "Sourcetype: $SOURCETYPE"
```

## CI/CD Integration

### GitHub Actions

```yaml
# .github/workflows/splunk-reports.yml
name: Splunk Scheduled Reports

on:
  schedule:
    - cron: '0 8 * * *'  # Daily at 8 AM

jobs:
  reports:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v3
      
      - name: Install splunk-cli
        run: |
          cargo build --release
          sudo cp target/release/splunk-cli /usr/local/bin/
      
      - name: Generate Reports
        env:
          SPLUNK_BASE_URL: ${{ secrets.SPLUNK_URL }}
          SPLUNK_API_TOKEN: ${{ secrets.SPLUNK_TOKEN }}
        run: |
          ./examples/automation/scheduled-reports.sh \
              --report "Failed Logins Summary" \
              --output-dir ./reports
      
      - name: Upload Reports
        uses: actions/upload-artifact@v3
        with:
          name: splunk-reports
          path: ./reports/
```

### GitLab CI

```yaml
# .gitlab-ci.yml
stages:
  - validate
  - report

validate_spl:
  stage: validate
  script:
    - cargo build --release
    - export PATH="$PATH:$PWD/target/release"
    - |
      for file in queries/*.spl; do
        splunk-cli search validate --file "$file" || exit 1
      done

scheduled_report:
  stage: report
  only:
    - schedules
  script:
    - cargo build --release
    - export PATH="$PATH:$PWD/target/release"
    - ./examples/automation/scheduled-reports.sh --report "Daily Summary"
```

### Jenkins Pipeline

```groovy
// Jenkinsfile
pipeline {
    agent any
    
    environment {
        SPLUNK_BASE_URL = credentials('splunk-url')
        SPLUNK_API_TOKEN = credentials('splunk-token')
    }
    
    stages {
        stage('Build CLI') {
            steps {
                sh 'cargo build --release'
            }
        }
        
        stage('Validate SPL') {
            steps {
                sh '''
                    for file in queries/*.spl; do
                        ./target/release/splunk-cli search validate --file "$file"
                    done
                '''
            }
        }
        
        stage('Generate Reports') {
            when {
                triggeredBy 'TimerTrigger'
            }
            steps {
                sh '''
                    export PATH="$PATH:$PWD/target/release"
                    ./examples/automation/scheduled-reports.sh \
                        --report "Daily Summary" \
                        --output-dir reports/
                '''
            }
            post {
                always {
                    archiveArtifacts artifacts: 'reports/**/*', fingerprint: true
                }
            }
        }
    }
}
```

## Ansible Integration

### Playbook Example

```yaml
# splunk-maintenance.yml
---
- name: Splunk Maintenance Tasks
  hosts: localhost
  gather_facts: no
  vars:
    splunk_examples_path: "/opt/splunk-tui/examples"
  
  tasks:
    - name: Run health check
      command: "{{ splunk_examples_path }}/daily-ops/health-check.sh --json"
      register: health_check
      changed_when: false
    
    - name: Alert on health issues
      debug:
        msg: "Health check found issues!"
      when: (health_check.stdout | from_json).summary.issues > 0
    
    - name: Clean up old jobs
      command: "{{ splunk_examples_path }}/daily-ops/job-cleanup.sh --older-than 48 --execute --force"
      register: cleanup_result
      changed_when: "'jobs cleaned' in cleanup_result.stdout"
    
    - name: Generate reports
      command: >
        {{ splunk_examples_path }}/automation/scheduled-reports.sh
        --report "{{ item }}"
        --output-dir /var/reports/splunk/
      loop:
        - "Daily Error Summary"
        - "License Usage"
      changed_when: false
```

## Terraform Integration

### External Data Source

```hcl
# Check Splunk health before applying
variable "splunk_health_check" {
  default = true
}

data "external" "splunk_health" {
  count = var.splunk_health_check ? 1 : 0
  
  program = ["bash", "-c", <<EOF
    /opt/splunk-tui/examples/daily-ops/health-check.sh --json | \
    jq '{healthy: (.summary.issues == 0 | tostring)}'
EOF
  ]
}

resource "null_resource" "splunk_validation" {
  count = var.splunk_health_check && data.external.splunk_health[0].result.healthy != "true" ? 0 : 1
  
  triggers = {
    always_run = timestamp()
  }
  
  provisioner "local-exec" {
    command = "echo 'Splunk is healthy, proceeding with deployment'"
  }
}
```

## See Also

- [Main Examples README](../README.md) - Overview of all example categories
- [Workflow Guide](../../docs/workflows.md) - Detailed workflow explanations

**Validation note:** these examples are checked offline for CLI contract drift and shell syntax, but any workflow that talks to a real Splunk instance still requires valid credentials and a reachable server.
