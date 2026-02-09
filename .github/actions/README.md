# Splunk GitHub Actions

Official GitHub Actions for Splunk operations in CI/CD pipelines. These actions enable workflows that interact with Splunk Enterprise for validation, testing, monitoring, and data ingestion.

## Available Actions

| Action | Description |
|--------|-------------|
| [`splunk-search`](./splunk-search/) | Execute SPL queries and export results as artifacts |
| [`splunk-saved-search-run`](./splunk-saved-search-run/) | Trigger saved searches and wait for completion |
| [`splunk-health-check`](./splunk-health-check/) | Verify Splunk instance health before deployment |
| [`splunk-config-validate`](./splunk-config-validate/) | Validate SPL queries and configuration files |
| [`splunk-hec-send`](./splunk-hec-send/) | Send events to Splunk via HTTP Event Collector (HEC) |

## Quick Start

```yaml
name: Splunk CI/CD

on: [push]

jobs:
  validate:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4

      - name: Check Splunk Health
        uses: ./.github/actions/splunk-health-check
        env:
          SPLUNK_BASE_URL: ${{ secrets.SPLUNK_BASE_URL }}
          SPLUNK_API_TOKEN: ${{ secrets.SPLUNK_API_TOKEN }}

      - name: Validate SPL
        uses: ./.github/actions/splunk-config-validate
        with:
          file: './queries/smoke-test.spl'
        env:
          SPLUNK_BASE_URL: ${{ secrets.SPLUNK_BASE_URL }}
          SPLUNK_API_TOKEN: ${{ secrets.SPLUNK_API_TOKEN }}

      - name: Run Search
        uses: ./.github/actions/splunk-search
        with:
          query: 'index=_internal | head 10'
          output-file: 'results.json'
        env:
          SPLUNK_BASE_URL: ${{ secrets.SPLUNK_BASE_URL }}
          SPLUNK_API_TOKEN: ${{ secrets.SPLUNK_API_TOKEN }}

      - name: Upload Results
        uses: actions/upload-artifact@v4
        with:
          name: search-results
          path: results.json
```

## Authentication

All actions support two methods for providing Splunk credentials:

### Method 1: Environment Variables (Recommended)

Set these in your workflow or repository secrets:

```yaml
env:
  SPLUNK_BASE_URL: ${{ secrets.SPLUNK_BASE_URL }}
  SPLUNK_API_TOKEN: ${{ secrets.SPLUNK_API_TOKEN }}
```

Supported environment variables:
- `SPLUNK_BASE_URL` - Splunk REST API URL (e.g., `https://splunk.example.com:8089`)
- `SPLUNK_API_TOKEN` - Splunk API token (preferred)
- `SPLUNK_USERNAME` - Username for session auth
- `SPLUNK_PASSWORD` - Password for session auth
- `SPLUNK_SKIP_VERIFY` - Skip TLS verification (`true`/`false`)

### Method 2: Action Inputs

Override environment variables per action:

```yaml
- uses: ./.github/actions/splunk-search
  with:
    base-url: 'https://splunk.example.com:8089'
    api-token: ${{ secrets.SPLUNK_API_TOKEN }}
```

## Container Image

All actions use the `splunk-cli` container image built from this repository. The image is automatically built and published to GitHub Container Registry via the [docker.yml](../workflows/docker.yml) workflow.

## Security

- All sensitive values (API tokens, passwords) are masked in GitHub Actions logs
- Credentials are passed via environment variables, not command-line arguments
- TLS verification is enabled by default (use `skip-verify: 'true'` only for self-signed certs in dev)

## Documentation

See the [GitHub Actions Documentation](../../docs/github-actions.md) for detailed usage examples and best practices.
