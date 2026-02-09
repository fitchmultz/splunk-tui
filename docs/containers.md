# Container Usage Guide

This guide covers running Splunk TUI and CLI in containers using Docker, Docker Compose, and Kubernetes (Helm).

## Overview

| Attribute | Value |
|-----------|-------|
| Base Image | `gcr.io/distroless/cc-debian12:nonroot` |
| User | `nonroot` (UID 65532) |
| Platforms | `linux/amd64`, `linux/arm64` |
| Registry | `ghcr.io/fitchmultz/splunk-tui` |

## Quick Start

### Pull and Run

```bash
# Pull the latest image
docker pull ghcr.io/fitchmultz/splunk-tui:latest

# Run a quick health check
docker run --rm \
  -e SPLUNK_BASE_URL=https://your-splunk:8089 \
  -e SPLUNK_API_TOKEN=your-token \
  ghcr.io/fitchmultz/splunk-tui:latest health

# List Splunk jobs
docker run --rm \
  -e SPLUNK_BASE_URL=https://your-splunk:8089 \
  -e SPLUNK_USERNAME=admin \
  -e SPLUNK_PASSWORD=changeme \
  ghcr.io/fitchmultz/splunk-tui:latest jobs list
```

### Interactive TUI Mode

```bash
# Run the TUI (requires interactive terminal)
docker run --rm -it \
  -e SPLUNK_BASE_URL=https://your-splunk:8089 \
  -e SPLUNK_API_TOKEN=your-token \
  --entrypoint /usr/local/bin/splunk-tui \
  ghcr.io/fitchmultz/splunk-tui:latest
```

### Using Make (Recommended for Local Development)

The project includes convenient Makefile targets for common container operations:

```bash
# Build the Docker image
make docker-build

# Run CLI with environment variables from your shell
make docker-run-cli ARGS="jobs list"
make docker-run-cli ARGS="health"

# Run TUI interactively
make docker-run-tui

# Start docker-compose environment (Splunk only)
make docker-compose-up

# Run CLI via docker-compose
make docker-compose-cli ARGS="jobs list"

# Run TUI via docker-compose
make docker-compose-tui

# Clean up Docker images
make docker-clean

# Install Helm chart (requires SPLUNK_BASE_URL, SPLUNK_USERNAME, SPLUNK_PASSWORD)
make helm-install

# Upgrade or uninstall Helm release
make helm-upgrade
make helm-uninstall
```

## Building Locally

### Basic Build

```bash
# Build the Docker image
docker build -t splunk-tui:latest .

# Verify the build
docker run --rm splunk-tui:latest --version
```

### Multi-Architecture Build

```bash
# Create buildx builder (first time only)
docker buildx create --name multiarch --use
docker buildx inspect --bootstrap

# Build for multiple platforms
docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -t ghcr.io/fitchmultz/splunk-tui:latest \
  --push .
```

### Build with Cache

```bash
# Build with layer caching
docker build \
  --cache-from=type=local,src=/tmp/.buildx-cache \
  --cache-to=type=local,dest=/tmp/.buildx-cache \
  -t splunk-tui:latest .
```

## Docker Compose Usage

### Starting the Development Environment

```bash
# Start Splunk Enterprise and supporting services
docker-compose up -d splunk

# Wait for Splunk to be healthy (check logs)
docker-compose logs -f splunk

# Once healthy, run CLI commands
docker-compose --profile cli run --rm cli jobs list

# Run the TUI interactively
docker-compose --profile tui run --rm tui
```

### Docker Compose Configuration

The included `docker-compose.yml` provides:

- **splunk**: Splunk Enterprise instance with REST API on port 8089
- **cli**: CLI service for running commands (profile: `cli`)
- **tui**: TUI service for interactive use (profile: `tui`)

### Environment Variables

Create a `.env` file:

```bash
SPLUNK_BASE_URL=https://splunk:8089
SPLUNK_USERNAME=admin
SPLUNK_PASSWORD=changeme
SPLUNK_SKIP_VERIFY=true
```

### Example Commands

```bash
# Run a search
docker-compose --profile cli run --rm cli \
  search "search index=_internal | head 10" --wait

# Export search results to CSV
docker-compose --profile cli run --rm cli \
  search "search index=main | head 100" --format csv > results.csv

# Check cluster health
docker-compose --profile cli run --rm cli cluster health
```

## Kubernetes Deployment

### Helm Installation

```bash
# Add the Helm repository (if published)
helm repo add splunk-tui https://fitchmultz.github.io/splunk-tui
helm repo update

# Install with minimal configuration
helm install splunk-tui ./helm/splunk-tui \
  --set cli.splunk.baseUrl=https://splunk:8089 \
  --set cli.splunk.username=admin \
  --set cli.splunk.password=changeme

# Install with existing secret
helm install splunk-tui ./helm/splunk-tui \
  --set cli.enabled=true \
  --set cli.existingSecret=my-splunk-secret
```

### Helm Values

Key configuration options:

```yaml
# values.yaml
image:
  repository: ghcr.io/fitchmultz/splunk-tui
  tag: "latest"

cli:
  enabled: true
  splunk:
    baseUrl: "https://splunk:8089"
    username: "admin"
    password: "changeme"
    skipVerify: false
  resources:
    limits:
      cpu: 500m
      memory: 256Mi

tui:
  enabled: false  # Enable for TUI sidecar deployment
```

### Using the CLI in Kubernetes

```bash
# Get pod name
CLI_POD=$(kubectl get pods -l app.kubernetes.io/component=cli -o jsonpath='{.items[0].metadata.name}')

# Run a command
kubectl exec -it $CLI_POD -- splunk-cli jobs list

# Run a search
kubectl exec -it $CLI_POD -- splunk-cli \
  search "search index=_internal | head 10" --wait
```

### CI/CD Job Example

```yaml
apiVersion: batch/v1
kind: Job
metadata:
  name: splunk-health-check
spec:
  template:
    spec:
      containers:
        - name: cli
          image: ghcr.io/fitchmultz/splunk-tui:latest
          command:
            - /usr/local/bin/splunk-cli
            - health
          envFrom:
            - secretRef:
                name: splunk-credentials
      restartPolicy: Never
```

### CronJob for Scheduled Tasks

```yaml
apiVersion: batch/v1
kind: CronJob
metadata:
  name: splunk-daily-report
spec:
  schedule: "0 9 * * *"
  jobTemplate:
    spec:
      template:
        spec:
          containers:
            - name: cli
              image: ghcr.io/fitchmultz/splunk-tui:latest
              command:
                - /usr/local/bin/splunk-cli
                - search
                - "search index=main earliest=-24h | stats count by host"
                - --wait
                - --format
                - json
              envFrom:
                - secretRef:
                    name: splunk-credentials
          restartPolicy: OnFailure
```

### TUI as Sidecar

For debugging or troubleshooting in Kubernetes:

```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: my-app-with-splunk-tui
spec:
  template:
    spec:
      containers:
        - name: my-app
          image: my-app:latest
          # ... main application config
        - name: splunk-tui
          image: ghcr.io/fitchmultz/splunk-tui:latest
          command: ["/bin/sh", "-c", "sleep infinity"]
          envFrom:
            - secretRef:
                name: splunk-credentials
          resources:
            limits:
              cpu: 500m
              memory: 256Mi
```

Access the TUI:

```bash
kubectl exec -it deploy/my-app-with-splunk-tui -c splunk-tui -- splunk-tui
```

## CI/CD Integration

### GitHub Actions

```yaml
name: Splunk Health Check

on:
  schedule:
    - cron: '0 */6 * * *'  # Every 6 hours

jobs:
  health-check:
    runs-on: ubuntu-latest
    container:
      image: ghcr.io/fitchmultz/splunk-tui:latest
    steps:
      - name: Check Splunk Health
        run: splunk-cli health
        env:
          SPLUNK_BASE_URL: ${{ secrets.SPLUNK_BASE_URL }}
          SPLUNK_API_TOKEN: ${{ secrets.SPLUNK_API_TOKEN }}

  search-verify:
    runs-on: ubuntu-latest
    steps:
      - name: Run Search
        uses: docker://ghcr.io/fitchmultz/splunk-tui:latest
        with:
          args: search "search index=main | head 10" --wait
        env:
          SPLUNK_BASE_URL: ${{ secrets.SPLUNK_BASE_URL }}
          SPLUNK_API_TOKEN: ${{ secrets.SPLUNK_API_TOKEN }}
```

### GitLab CI

```yaml
variables:
  SPLUNK_IMAGE: ghcr.io/fitchmultz/splunk-tui:latest

.splunk_base:
  image: $SPLUNK_IMAGE
  variables:
    SPLUNK_BASE_URL: $SPLUNK_BASE_URL
    SPLUNK_API_TOKEN: $SPLUNK_API_TOKEN

health_check:
  extends: .splunk_base
  script:
    - splunk-cli health
  rules:
    - if: $CI_PIPELINE_SOURCE == "schedule"

search_validation:
  extends: .splunk_base
  script:
    - splunk-cli search "search index=main | head 100" --wait --format json
```

### Jenkins Pipeline

```groovy
pipeline {
    agent {
        docker {
            image 'ghcr.io/fitchmultz/splunk-tui:latest'
        }
    }
    environment {
        SPLUNK_BASE_URL = credentials('splunk-base-url')
        SPLUNK_API_TOKEN = credentials('splunk-api-token')
    }
    stages {
        stage('Health Check') {
            steps {
                sh 'splunk-cli health'
            }
        }
        stage('Search Test') {
            steps {
                sh 'splunk-cli search "search index=_internal | head 10" --wait'
            }
        }
    }
}
```

### Azure DevOps

```yaml
trigger:
  - main

pool:
  vmImage: 'ubuntu-latest'

container:
  image: ghcr.io/fitchmultz/splunk-tui:latest
  endpoint: ghcr-connection

steps:
  - script: splunk-cli health
    displayName: 'Check Splunk Health'
    env:
      SPLUNK_BASE_URL: $(SPLUNK_BASE_URL)
      SPLUNK_API_TOKEN: $(SPLUNK_API_TOKEN)
```

### CircleCI

```yaml
version: 2.1

jobs:
  splunk-check:
    docker:
      - image: ghcr.io/fitchmultz/splunk-tui:latest
    steps:
      - run:
          name: Check Splunk Health
          command: splunk-cli health
          environment:
            SPLUNK_BASE_URL: $SPLUNK_BASE_URL
            SPLUNK_API_TOKEN: $SPLUNK_API_TOKEN

workflows:
  version: 2
  scheduled:
    triggers:
      - schedule:
          cron: "0 */6 * * *"
          filters:
            branches:
              only:
                - main
    jobs:
      - splunk-check
```

## Security Considerations

### Distroless Base Image

The container uses Google's distroless image which:
- Contains no shell (no `sh`, `bash`)
- Contains no package manager
- Minimal attack surface
- Only runtime dependencies for C/C++ applications

### Non-Root User

Containers run as `nonroot` user (UID 65532):
- No privilege escalation possible
- File system access limited
- Follows principle of least privilege

### Secrets Management

**Never pass secrets via command line arguments** (they appear in process lists).

#### Docker Secrets (Swarm)

```yaml
# docker-compose.yml
secrets:
  splunk_password:
    external: true

services:
  cli:
    image: ghcr.io/fitchmultz/splunk-tui:latest
    secrets:
      - splunk_password
    environment:
      SPLUNK_PASSWORD_FILE: /run/secrets/splunk_password
```

#### Kubernetes Secrets

```bash
# Create secret
kubectl create secret generic splunk-credentials \
  --from-literal=SPLUNK_BASE_URL=https://splunk:8089 \
  --from-literal=SPLUNK_USERNAME=admin \
  --from-literal=SPLUNK_PASSWORD=changeme

# Use in Helm
helm install splunk-tui ./helm/splunk-tui \
  --set cli.existingSecret=splunk-credentials
```

#### External Secrets Operator

For cloud-managed secrets (AWS Secrets Manager, GCP Secret Manager, Azure Key Vault):

```yaml
apiVersion: external-secrets.io/v1beta1
kind: ExternalSecret
metadata:
  name: splunk-credentials
spec:
  secretStoreRef:
    name: aws-secrets-manager
    kind: ClusterSecretStore
  target:
    name: splunk-credentials
  data:
    - secretKey: SPLUNK_PASSWORD
      remoteRef:
        key: prod/splunk
        property: password
```

### TLS Verification

**Production**: Always use valid TLS certificates:

```bash
docker run --rm \
  -e SPLUNK_BASE_URL=https://splunk.company.com:8089 \
  -e SPLUNK_API_TOKEN=token \
  -v /path/to/ca-cert.pem:/etc/ssl/certs/splunk-ca.pem:ro \
  ghcr.io/fitchmultz/splunk-tui:latest health
```

**Development Only**: Skip TLS verification (not recommended for production):

```bash
docker run --rm \
  -e SPLUNK_BASE_URL=https://splunk:8089 \
  -e SPLUNK_API_TOKEN=token \
  -e SPLUNK_SKIP_VERIFY=true \
  ghcr.io/fitchmultz/splunk-tui:latest health
```

### Network Policies

Restrict egress in Kubernetes:

```yaml
apiVersion: networking.k8s.io/v1
kind: NetworkPolicy
metadata:
  name: splunk-tui-policy
spec:
  podSelector:
    matchLabels:
      app.kubernetes.io/name: splunk-tui
  policyTypes:
    - Egress
  egress:
    - to:
        - podSelector:
            matchLabels:
              app: splunk
      ports:
        - protocol: TCP
          port: 8089
```

## Troubleshooting

### Container Won't Start

```bash
# Check logs
docker logs <container-id>

# Verify environment variables
docker run --rm -e SPLUNK_BASE_URL=test ghcr.io/fitchmultz/splunk-tui:latest --help
```

### Connection Refused / Timeout

**Issue**: Cannot connect to Splunk

**Solutions**:
1. Verify Splunk URL is accessible from container:
   ```bash
   docker run --rm --network=host splunk-tui:latest health
   ```

2. Check firewall rules between container and Splunk

3. For Docker Compose, ensure services are on same network:
   ```yaml
   services:
     cli:
       networks:
         - splunk-network
   networks:
     splunk-network:
   ```

### TLS Certificate Errors

**Issue**: `certificate verify failed`

**Solutions**:
1. Mount custom CA certificate:
   ```bash
   docker run --rm \
     -v /path/to/ca.crt:/etc/ssl/certs/splunk-ca.crt:ro \
     -e SPLUNK_BASE_URL=https://splunk:8089 \
     ghcr.io/fitchmultz/splunk-tui:latest health
   ```

2. Skip verification (development only):
   ```bash
   docker run -e SPLUNK_SKIP_VERIFY=true ...
   ```

### Authentication Failures

**Issue**: `401 Unauthorized` or `403 Forbidden`

**Solutions**:
1. Verify credentials are correct
2. Check password doesn't contain special characters that need escaping
3. For API tokens, ensure token has required capabilities
4. Quote password in docker run:
   ```bash
   docker run -e "SPLUNK_PASSWORD=p@ssw0rd!" ...
   ```

### TUI Display Issues

**Issue**: TUI doesn't render correctly or shows garbled text

**Solutions**:
1. Ensure TTY is allocated:
   ```bash
   docker run -it ...
   ```
   (Both `-i` for interactive and `-t` for TTY are required)

2. Set terminal type:
   ```bash
   docker run -it -e TERM=xterm-256color ...
   ```

3. For Windows terminals, use Windows Terminal or WSL2

### Permission Denied

**Issue**: Cannot write files or access directories

**Cause**: Container runs as UID 65532 (nonroot)

**Solution**: Ensure mounted volumes have correct permissions:

```bash
# Create directory with correct ownership
docker run --rm -v $(pwd)/output:/output ghcr.io/fitchmultz/splunk-tui:latest \
  search "search index=main | head 10" --wait --format csv > output/results.csv
```

### Out of Memory

**Issue**: Container killed (OOMKilled)

**Solutions**:
1. Increase memory limit:
   ```bash
   docker run --memory=512m ...
   ```

2. For large result sets, use pagination:
   ```bash
   splunk-cli search "..." --max-count 1000
   ```

3. In Kubernetes, increase resources in values.yaml:
   ```yaml
   cli:
     resources:
       limits:
         memory: 512Mi
   ```

### Debugging

Since the distroless image has no shell, use these debugging techniques:

**Option 1: Use a debug sidecar**

```bash
# Run a debug container alongside
docker run --rm --network=container:<splunk-tui-container> \
  nicolaka/netshoot \
  curl -k https://splunk:8089/services/server/info
```

**Option 2: Build a debug version**

```dockerfile
# Dockerfile.debug
FROM splunk-tui:latest as base
FROM alpine:latest
COPY --from=base /usr/local/bin/splunk-cli /usr/local/bin/
COPY --from=base /usr/local/bin/splunk-tui /usr/local/bin/
RUN apk add --no-cache ca-certificates
ENTRYPOINT ["/usr/local/bin/splunk-cli"]
```

```bash
docker build -f Dockerfile.debug -t splunk-tui:debug .
docker run --rm -it splunk-tui:debug sh
```

**Option 3: Check logs and describe**

```bash
# Kubernetes
kubectl logs <pod>
kubectl describe pod <pod>

# Docker
docker logs <container>
docker inspect <container>
```

## Reference

### Environment Variables

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `SPLUNK_BASE_URL` | Yes | - | Splunk REST API URL |
| `SPLUNK_USERNAME` | No* | - | Splunk username |
| `SPLUNK_PASSWORD` | No* | - | Splunk password |
| `SPLUNK_API_TOKEN` | No* | - | Splunk API token (preferred) |
| `SPLUNK_SKIP_VERIFY` | No | `false` | Skip TLS verification |
| `SPLUNK_TIMEOUT` | No | `30` | Connection timeout (seconds) |
| `SPLUNK_MAX_RETRIES` | No | `3` | Max retry attempts |
| `RUST_LOG` | No | `info` | Log level (error, warn, info, debug, trace) |

*Either API token OR username/password required

### Image Tags

| Tag | Description |
|-----|-------------|
| `latest` | Latest stable release |
| `v0.1.0` | Specific version |
| `main` | Latest build from main branch |
| `sha-abc1234` | Specific commit |

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | Success |
| 1 | General error |
| 2 | Invalid arguments |
| 3 | Authentication failure |
| 4 | Connection error |
| 5 | Timeout |
| 130 | Interrupted (Ctrl+C) |

## Additional Resources

- [Main README](../README.md)
- [Usage Guide](usage.md)
- [Security Policy](../SECURITY.md)
- [Splunk REST API Documentation](https://docs.splunk.com/Documentation/Splunk/latest/RESTREF/RESTprolog)
