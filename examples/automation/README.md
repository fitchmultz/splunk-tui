# Automation Examples

Scripts for CI/CD integration, scheduled tasks, and bulk operations.

## Scripts

### scheduled-reports.sh

Generate and save scheduled reports from saved searches or ad-hoc queries.

```bash
# Run a saved search and save results
./scheduled-reports.sh --report "Daily Error Summary" --output-dir /var/reports/splunk/

# Run with specific format
./scheduled-reports.sh --report "Security Events" --format csv --output-dir ./reports/

# Run multiple reports from a list
./scheduled-reports.sh --report-list reports.txt --output-dir /var/reports/

# Ad-hoc report with custom SPL
./scheduled-reports.sh --search "index=main | stats count by host" --output-file report.json
```

**Features:**
- Timestamped output filenames
- Multiple output formats (json, csv, xml)
- Saved search execution
- Ad-hoc SPL query execution
- Email notification support (with optional sendmail configuration)

### bulk-operations.sh

Perform bulk operations on Splunk resources safely.

```bash
# Preview operations (dry-run, default)
./bulk-operations.sh --operation disable-searches --file old-searches.txt

# Execute the operations
./bulk-operations.sh --operation disable-searches --file old-searches.txt --execute

# Available operations:
# - disable-searches / enable-searches / delete-searches
# - disable-apps / enable-apps
# - delete-lookup-files

# Force execution without confirmation
./bulk-operations.sh --operation delete-searches --file obsolete.txt --execute --force
```

**Safety Features:**
- Dry-run mode by default
- Batch confirmation prompts
- Progress reporting
- Detailed logging
- Rollback capability (for some operations)

### data-onboarding.sh

Automate the complete data onboarding workflow.

```bash
# Basic onboarding - create index only
./data-onboarding.sh --index newapp

# Full onboarding with HEC
./data-onboarding.sh --index newapp --sourcetype newapp:logs --hec

# With custom retention
./data-onboarding.sh --index newapp --sourcetype newapp:logs --retention 90

# Validate data is flowing
./data-onboarding.sh --index newapp --sourcetype newapp:logs --validate
```

**Workflow:**
1. Create index (if not exists)
2. Configure HEC input (if --hec specified)
3. Send test event for validation
4. Create monitoring saved searches
5. Generate onboarding documentation

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

# Email reports (if mail configured)
if [ -f "$REPORT_DIR/Failed-Logins-Summary.csv" ]; then
    mail -s "Daily Splunk Reports $(date +%Y-%m-%d)" \
         -A "$REPORT_DIR/Failed-Logins-Summary.csv" \
         security-team@example.com < /dev/null
fi
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

# Clean up old lookup files
splunk-cli lookups list --output json | \
    jq -r '.[] | select(.size > 104857600) | .name' > /tmp/large-lookups.txt

./bulk-operations.sh \
    --operation delete-lookup-files \
    --file /tmp/large-lookups.txt
```

### New Application Onboarding

```bash
#!/bin/bash
# onboard-new-app.sh <app_name> <sourcetype> [retention_days]

APP_NAME="$1"
SOURCETYPE="$2"
RETENTION="${3:-90}"

if [ -z "$APP_NAME" ] || [ -z "$SOURCETYPE" ]; then
    echo "Usage: $0 <app_name> <sourcetype> [retention_days]"
    exit 1
fi

# Run onboarding workflow
./data-onboarding.sh \
    --index "$APP_NAME" \
    --sourcetype "$SOURCETYPE" \
    --retention "$RETENTION" \
    --hec \
    --validate

# Create standard saved searches
./scheduled-reports.sh \
    --search "index=$APP_NAME | stats count by host, source | sort -count" \
    --output-file "/opt/splunk-saved-searches/${APP_NAME}-overview.json"

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
              --report-list reports.txt \
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
                        --report-list config/reports.txt \
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
