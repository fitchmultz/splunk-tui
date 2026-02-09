# GitHub Actions for Splunk

Official GitHub Actions for integrating Splunk operations into CI/CD pipelines. These actions enable teams to validate configurations, run searches, check health, and send events to Splunk directly from GitHub workflows.

## Table of Contents

- [Overview](#overview)
- [Quick Start](#quick-start)
- [Authentication](#authentication)
- [Available Actions](#available-actions)
  - [splunk-search](#splunk-search)
  - [splunk-saved-search-run](#splunk-saved-search-run)
  - [splunk-health-check](#splunk-health-check)
  - [splunk-config-validate](#splunk-config-validate)
  - [splunk-hec-send](#splunk-hec-send)
- [Example Workflows](#example-workflows)
- [Best Practices](#best-practices)
- [Troubleshooting](#troubleshooting)

## Overview

These GitHub Actions provide a seamless way to integrate Splunk into your CI/CD workflows:

| Use Case | Action | Description |
|----------|--------|-------------|
| **Validation** | `splunk-config-validate` | Validate SPL syntax before deployment |
| **Testing** | `splunk-saved-search-run` | Run smoke tests via saved searches |
| **Monitoring** | `splunk-health-check` | Verify Splunk health before/after deployments |
| **Reporting** | `splunk-search` | Execute SPL and export results as artifacts |
| **Observability** | `splunk-hec-send` | Send CI/CD events to Splunk via HEC |

## Quick Start

Add a Splunk health check to your workflow:

```yaml
name: Deploy
on: [push]

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Check Splunk Health
        uses: ./.github/actions/splunk-health-check
        env:
          SPLUNK_BASE_URL: ${{ secrets.SPLUNK_BASE_URL }}
          SPLUNK_API_TOKEN: ${{ secrets.SPLUNK_API_TOKEN }}
```

## Authentication

### Environment Variables (Recommended)

Configure these in your repository secrets (Settings → Secrets and variables → Actions):

| Variable | Description | Required |
|----------|-------------|----------|
| `SPLUNK_BASE_URL` | Splunk REST API URL (e.g., `https://splunk.example.com:8089`) | Yes |
| `SPLUNK_API_TOKEN` | Splunk API token (preferred auth method) | Yes* |
| `SPLUNK_USERNAME` | Username for session authentication | Yes* |
| `SPLUNK_PASSWORD` | Password for session authentication | Yes* |
| `SPLUNK_SKIP_VERIFY` | Skip TLS verification (`true`/`false`) | No |

\* Either API token OR username/password is required.

### Input Parameters

Override environment variables for specific actions:

```yaml
- uses: ./.github/actions/splunk-search
  with:
    base-url: 'https://splunk.example.com:8089'
    api-token: ${{ secrets.SPLUNK_API_TOKEN }}
    query: 'index=_internal | head 10'
```

### HEC Authentication

For HEC operations, use these secrets:

| Variable | Description |
|----------|-------------|
| `SPLUNK_HEC_URL` | HEC endpoint URL (e.g., `https://splunk.example.com:8088`) |
| `SPLUNK_HEC_TOKEN` | HEC authentication token |

## Available Actions

### splunk-search

Execute SPL queries and export results as artifacts.

```yaml
- uses: ./.github/actions/splunk-search
  with:
    query: 'index=_internal | stats count by sourcetype | head 10'
    earliest: '-24h'
    latest: 'now'
    count: 1000
    output-format: 'json'
    output-file: 'results.json'
```

#### Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `query` | SPL query to execute | Yes | - |
| `earliest` | Earliest time | No | `-24h` |
| `latest` | Latest time | No | `now` |
| `count` | Maximum results | No | `1000` |
| `output-format` | Output format | No | `json` |
| `output-file` | Output file path | No | `search-results.json` |
| `base-url` | Splunk base URL | No | - |
| `api-token` | Splunk API token | No | - |
| `username` | Splunk username | No | - |
| `password` | Splunk password | No | - |
| `skip-verify` | Skip TLS verification | No | `false` |

#### Outputs

| Output | Description |
|--------|-------------|
| `result-file` | Path to the result file |
| `result-count` | Number of results returned |

### splunk-saved-search-run

Trigger saved searches and wait for completion.

```yaml
- uses: ./.github/actions/splunk-saved-search-run
  with:
    name: 'CI Smoke Test'
    earliest: '-1h'
    output-file: 'smoke-test-results.json'
```

#### Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `name` | Name of the saved search | Yes | - |
| `earliest` | Override earliest time | No | - |
| `latest` | Override latest time | No | - |
| `count` | Maximum results | No | `1000` |
| `output-format` | Output format | No | `json` |
| `output-file` | Output file path | No | `saved-search-results.json` |
| `base-url` | Splunk base URL | No | - |
| `api-token` | Splunk API token | No | - |
| `username` | Splunk username | No | - |
| `password` | Splunk password | No | - |
| `skip-verify` | Skip TLS verification | No | `false` |

#### Outputs

| Output | Description |
|--------|-------------|
| `result-file` | Path to the result file |
| `result-count` | Number of results returned |
| `search-name` | Name of the saved search executed |

### splunk-health-check

Verify Splunk instance health before deployment.

```yaml
- uses: ./.github/actions/splunk-health-check
  with:
    fail-on-degraded: 'true'
    output-file: 'health-report.json'
```

#### Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `fail-on-degraded` | Fail if health is degraded | No | `true` |
| `fail-on-unhealthy` | Fail if health is unhealthy | No | `true` |
| `output-file` | Output file for health report | No | `health-report.json` |
| `output-format` | Output format | No | `json` |
| `base-url` | Splunk base URL | No | - |
| `api-token` | Splunk API token | No | - |
| `username` | Splunk username | No | - |
| `password` | Splunk password | No | - |
| `skip-verify` | Skip TLS verification | No | `false` |

#### Outputs

| Output | Description |
|--------|-------------|
| `status` | Health status (healthy, degraded, unhealthy, unknown) |
| `output-file` | Path to health report |
| `healthy` | Whether health check passed (true/false) |

### splunk-config-validate

Validate SPL queries against Splunk.

```yaml
- uses: ./.github/actions/splunk-config-validate
  with:
    file: './queries/smoke-test.spl'
    fail-on-warning: 'true'
```

#### Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `query` | SPL query string to validate | No* | - |
| `file` | Path to SPL file to validate | No* | - |
| `fail-on-warning` | Fail if warnings present | No | `false` |
| `output-file` | Output file path | No | `validation-results.json` |
| `output-format` | Output format | No | `json` |
| `base-url` | Splunk base URL | No | - |
| `api-token` | Splunk API token | No | - |
| `username` | Splunk username | No | - |
| `password` | Splunk password | No | - |
| `skip-verify` | Skip TLS verification | No | `false` |

\* Either `query` or `file` is required.

#### Outputs

| Output | Description |
|--------|-------------|
| `valid` | Whether validation passed (true/false) |
| `errors` | Number of errors found |
| `warnings` | Number of warnings found |
| `output-file` | Path to validation results |

### splunk-hec-send

Send events to Splunk via HTTP Event Collector (HEC).

```yaml
# Send single event
- uses: ./.github/actions/splunk-hec-send
  with:
    hec-url: ${{ secrets.SPLUNK_HEC_URL }}
    hec-token: ${{ secrets.SPLUNK_HEC_TOKEN }}
    event: |
      {"message": "Deployment completed", "status": "success"}
    index: 'ci_cd_events'
    sourcetype: 'github:deployment'

# Send batch from file
- uses: ./.github/actions/splunk-hec-send
  with:
    hec-url: ${{ secrets.SPLUNK_HEC_URL }}
    hec-token: ${{ secrets.SPLUNK_HEC_TOKEN }}
    events-file: './events.json'
    index: 'github_events'
```

#### Inputs

| Input | Description | Required | Default |
|-------|-------------|----------|---------|
| `hec-url` | HEC endpoint URL | Yes | - |
| `hec-token` | HEC authentication token | Yes | - |
| `event` | Event data (JSON string or `@file.json`) | No* | - |
| `events-file` | Path to file with events array | No* | - |
| `index` | Destination index | No | - |
| `source` | Source field | No | - |
| `sourcetype` | Sourcetype field | No | - |
| `host` | Host field | No | - |
| `time` | Event timestamp (Unix epoch) | No | - |
| `ndjson` | Use NDJSON format for batch | No | `false` |
| `skip-verify` | Skip TLS verification | No | `false` |
| `output-file` | Output file for response | No | `hec-response.json` |

\* Either `event` or `events-file` is required.

#### Outputs

| Output | Description |
|--------|-------------|
| `success` | Whether send succeeded |
| `ack-id` | Acknowledgment ID (single event) |
| `ack-ids` | Comma-separated acknowledgment IDs (batch) |
| `output-file` | Path to response file |

## Example Workflows

### Complete CI/CD Pipeline

```yaml
name: Splunk CI/CD Pipeline

on:
  push:
    branches: [main]

env:
  SPLUNK_BASE_URL: ${{ secrets.SPLUNK_BASE_URL }}
  SPLUNK_API_TOKEN: ${{ secrets.SPLUNK_API_TOKEN }}

jobs:
  deploy:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      # 1. Validate SPL queries
      - uses: ./.github/actions/splunk-config-validate
        with:
          file: './config/validation.spl'

      # 2. Pre-deployment health check
      - uses: ./.github/actions/splunk-health-check
        id: pre-health

      # 3. Run smoke tests
      - uses: ./.github/actions/splunk-saved-search-run
        with:
          name: 'CI Smoke Test'
          output-file: 'smoke-test.json'

      # 4. Your deployment steps here...

      # 5. Post-deployment health check
      - uses: ./.github/actions/splunk-health-check
        id: post-health

      # 6. Log deployment event
      - uses: ./.github/actions/splunk-hec-send
        with:
          hec-url: ${{ secrets.SPLUNK_HEC_URL }}
          hec-token: ${{ secrets.SPLUNK_HEC_TOKEN }}
          event: |
            {"message": "Deployment completed", "ref": "${{ github.ref }}"}
          index: 'ci_cd_events'
```

### Scheduled Security Report

```yaml
name: Daily Security Report

on:
  schedule:
    - cron: '0 6 * * *'

jobs:
  report:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - uses: ./.github/actions/splunk-search
        with:
          query: 'index=security | stats count by severity'
          earliest: '-24h'
          output-file: 'security-report.json'
        env:
          SPLUNK_BASE_URL: ${{ secrets.SPLUNK_BASE_URL }}
          SPLUNK_API_TOKEN: ${{ secrets.SPLUNK_API_TOKEN }}

      - uses: actions/upload-artifact@v4
        with:
          name: security-report
          path: security-report.json
```

## Best Practices

### Security

1. **Use Repository Secrets**: Store all credentials in GitHub Secrets, never in code
2. **Mask Sensitive Values**: Actions automatically mask API tokens and passwords
3. **Enable TLS Verification**: Only use `skip-verify: 'true'` for self-signed certificates in development
4. **Use API Tokens**: Prefer API tokens over username/password authentication

### Performance

1. **Limit Result Counts**: Use reasonable `count` values to avoid memory issues
2. **Use Appropriate Time Ranges**: Narrow `earliest`/`latest` windows for faster searches
3. **Save Results**: Use `output-file` and `actions/upload-artifact` to persist results

### Reliability

1. **Health Checks**: Always verify Splunk health before critical operations
2. **Error Handling**: Use `continue-on-error` for non-critical checks
3. **Time Limits**: Set appropriate job timeouts for long-running searches

```yaml
- uses: ./.github/actions/splunk-search
  continue-on-error: true
  timeout-minutes: 10
  with:
    query: 'index=main | head 100'
```

## Troubleshooting

### Connection Refused

**Error**: `Connection refused` or `Cannot connect to Splunk`

**Solutions**:
- Verify `SPLUNK_BASE_URL` is correct and includes the port (e.g., `:8089`)
- Check that the Splunk server is accessible from GitHub Actions runners
- For self-hosted runners, ensure network connectivity to Splunk

### Authentication Failed

**Error**: `401 Unauthorized` or `Authentication failed`

**Solutions**:
- Verify API token or username/password in secrets
- Ensure the token has appropriate permissions
- Check that the token hasn't expired

### TLS Certificate Errors

**Error**: `certificate verify failed` or `TLS handshake error`

**Solutions**:
- For production: Use valid certificates signed by a trusted CA
- For development: Use `skip-verify: 'true'` (not recommended for production)

### Search Returns No Results

**Error**: Search completes but `result-count` is 0

**Solutions**:
- Check time range (`earliest`/`latest`) covers data existence period
- Verify the index exists and contains data
- Validate SPL syntax with `splunk-config-validate`

### HEC Connection Issues

**Error**: `HEC health check failed`

**Solutions**:
- HEC uses port 8088 by default (not 8089 for REST API)
- Verify HEC is enabled on the Splunk server
- Check that the HEC token is valid and not disabled

## Additional Resources

- [Example Workflows](../.github/workflows/examples/)
- [Splunk CLI Documentation](./usage.md)
- [Container Deployment Guide](./containers.md)
