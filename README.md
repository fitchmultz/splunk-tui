# Splunk CLI and TUI

A robust Rust-based CLI and TUI tool for managing Splunk Enterprise v9+ deployments. Supports both standalone and clustered deployments with dual authentication methods (session token and API token).

## Features

- **Dual Authentication**: Session token (username/password) and API token support
- **Search Management**: Create, execute, and monitor search jobs
- **Index Operations**: List and inspect indexes with detailed information
- **Cluster Support**: View cluster status and peer information
- **Health Monitoring**: Comprehensive health checks including server info, splunkd health, license usage, KVStore status, and log parsing health
- **Interactive TUI**: Terminal user interface with tab navigation
- **Secure**: Credentials handled with the `secrecy` crate

## Installation

### From Source

```bash
# Fetch dependencies
make install

# Build and install optimized release binaries to ~/.local/bin
make release
```

### Binaries

Two binaries are installed:
- `splunk` - Command-line interface
- `splunk-tui` - Interactive terminal interface

## Configuration

Configuration is loaded from environment variables or a `.env` file:

```bash
# Splunk Connection
export SPLUNK_BASE_URL=https://localhost:8089
export SPLUNK_USERNAME=admin
export SPLUNK_PASSWORD=changeme
export SPLUNK_API_TOKEN=your-api-token

# Optional
export SPLUNK_SKIP_VERIFY=false
export SPLUNK_TIMEOUT=30
export SPLUNK_MAX_RETRIES=3
```

Copy `.env.example` to `.env` and configure as needed.

## CLI Usage

### Search

```bash
# Execute a search and wait for results
splunk search "search index=main | head 10" --wait

# Search with time range
splunk search "search index=main ERROR" \
  --earliest "-24h" \
  --latest "now" \
  --wait

# Maximum results
splunk search "search index=main" --count 1000 --wait
```

### Indexes

```bash
# List all indexes
splunk indexes

# Detailed information
splunk indexes --detailed
```

### Cluster

```bash
# Show cluster status
splunk cluster

# Detailed cluster information including peers
splunk cluster --detailed
```

### Jobs

```bash
# List all search jobs
splunk jobs --list

# Cancel a job
splunk jobs --cancel 123456789.123456789

# Delete a job
splunk jobs --delete 123456789.123456789
```

## TUI Usage

Launch the interactive terminal interface:

```bash
splunk-tui
```

### Keybindings

- `1` - Switch to Search screen
- `2` - Switch to Indexes screen
- `3` - Switch to Cluster screen
- `4` - Switch to Jobs screen
- `5` - Switch to Health screen
- `q` - Quit
- `r` - Refresh current screen data
- `?` - Show help popup

### Screens

**Search**: Enter SPL queries and view results
**Indexes**: Browse and inspect indexes
**Cluster**: View cluster status and peer information
**Jobs**: Monitor and manage search jobs (cancel, delete, inspect)
**Health**: Comprehensive health monitoring including server info, splunkd health, license usage, KVStore status, and log parsing health

## Documentation

- [User Guide](docs/user-guide.md) - Task-oriented guide for CLI and TUI
- [Usage Guide](docs/usage.md) - Detailed technical reference and configuration
- [Development Guide](docs/splunk-test-environment.md) - Setting up a local Splunk environment

## Development

### Prerequisites

- Rust 1.84 or later
- Make

### Build

```bash
# Format code
make format

# Run linter
make lint

# Type check
make type-check

# Run tests
make test

# Full CI pipeline
make ci
```

### Project Structure

```
splunk-tui/
├── crates/
│   ├── cli/          # CLI binary
│   ├── tui/          # TUI binary
│   ├── client/       # Splunk REST API client
│   └── config/       # Configuration management
├── Makefile
└── Cargo.toml
```

## Authentication

### Session Token (Username/Password)

```bash
export SPLUNK_USERNAME=admin
export SPLUNK_PASSWORD=changeme
```

The client will automatically login and manage session token renewal.

### API Token (Recommended)

```bash
export SPLUNK_API_TOKEN=your-api-token
```

API token authentication is preferred for automation as it doesn't require session management.

## Makefile Targets

| Target | Description |
|--------|-------------|
| `make install` | Fetch all dependencies |
| `make update` | Update dependencies to latest stable versions |
| `make lint` | Run clippy (warnings as errors) |
| `make type-check` | Run cargo check |
| `make format` | Format code with rustfmt |
| `make clean` | Remove build artifacts and lock files |
| `make test` | Run all tests (unit + integration) |
| `make release` | Optimized release build and install to ~/.local/bin |
| `make build` | Alias for release |
| `make ci` | Full CI pipeline |

## Contributing

1. Format code: `make format`
2. Run linter: `make lint`
3. Run tests: `make test`
4. Run CI: `make ci`

## License

MIT
