# Container Guide

This guide covers the supported container workflows for `splunk-cli` and `splunk-tui`.

## Supported Surface

- Local Docker image builds
- Local Docker Compose orchestration for a development Splunk instance
- Running `splunk-cli` or `splunk-tui` inside the project image

Hosted orchestration manifests are intentionally out of scope for this repository.

## Build the Runtime Image

```bash
make docker-build
```

The runtime image contains only:

- `/usr/local/bin/splunk-cli`
- `/usr/local/bin/splunk-tui`

The internal `generate-tui-docs` tool is not shipped in the runtime image.

## Run the CLI

Provide connection metadata with environment variables and load secrets from a local `.env` file, shell secret store, or CI secret injection.

```bash
docker run --rm -it \
  -e SPLUNK_BASE_URL \
  -e SPLUNK_USERNAME \
  -e SPLUNK_PASSWORD \
  -e SPLUNK_API_TOKEN \
  -e SPLUNK_SKIP_VERIFY \
  splunk-tui:latest search 'index=_internal | head 10'
```

## Run the TUI

```bash
docker run --rm -it \
  -e SPLUNK_BASE_URL \
  -e SPLUNK_USERNAME \
  -e SPLUNK_PASSWORD \
  -e SPLUNK_API_TOKEN \
  -e SPLUNK_SKIP_VERIFY \
  --entrypoint /usr/local/bin/splunk-tui \
  splunk-tui:latest
```

## Docker Compose Development Flow

Start the local Splunk container:

```bash
make docker-compose-up
```

Run the CLI against the compose environment:

```bash
make docker-compose-cli ARGS="doctor"
```

Run the TUI against the compose environment:

```bash
make docker-compose-tui
```

## Secret Handling

- Do not place literal passwords or tokens directly in shell history.
- Prefer a local `.env` file, shell startup that hydrates variables from a keyring, or CI-secret injection.
- Keep `.env` and `.env.test` untracked.

Example `.env`:

```bash
SPLUNK_BASE_URL=https://localhost:8089
SPLUNK_USERNAME=replace-with-your-username
SPLUNK_SKIP_VERIFY=true
# Load SPLUNK_PASSWORD or SPLUNK_API_TOKEN from your shell/keyring workflow.
```

## Validation

```bash
make docker-build
make docker-compose-up
make docker-compose-cli ARGS="doctor --output json"
```

## Exit Codes

Containerized CLI runs return the same structured exit codes as local `splunk-cli` runs.

| Code | Meaning |
| --- | --- |
| 0 | Success |
| 1 | General error |
| 2 | Authentication failure |
| 3 | Connection error |
| 4 | Resource not found |
| 5 | Validation error |
| 6 | Permission denied |
| 7 | Rate limited |
| 8 | Service unavailable |
| 130 | Interrupted (Ctrl+C) |
