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
- `splunk-cli` - Command-line interface
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
splunk-cli search "search index=main | head 10" --wait

# Search with time range
splunk-cli search "search index=main ERROR" \
  --earliest "-24h" \
  --latest "now" \
  --wait

# Maximum results
splunk-cli search "search index=main" --count 1000 --wait
```

### Indexes

```bash
# List all indexes
splunk-cli indexes

# Detailed information
splunk-cli indexes --detailed
```

### Cluster

```bash
# Show cluster status
splunk-cli cluster

# Detailed cluster information including peers
splunk-cli cluster --detailed
```

### Jobs

```bash
# List all search jobs
splunk-cli jobs --list

# Cancel a job
splunk-cli jobs --cancel 123456789.123456789

# Delete a job
splunk-cli jobs --delete 123456789.123456789
```

## TUI Usage

Launch the interactive terminal interface:

```bash
splunk-tui
```

### CLI Options

The TUI supports several command-line options for configuration:

```bash
# Use a specific profile
splunk-tui --profile production

# Custom config file location
splunk-tui --config-path /path/to/config.json

# Disable mouse support
splunk-tui --no-mouse

# Custom log directory
splunk-tui --log-dir /var/log/splunk-tui
```

Run `splunk-tui --help` for all available options.

#### Configuration Precedence

Configuration values are loaded in the following order (highest to lowest):
1. CLI arguments (e.g., `--profile`, `--config-path`)
2. Environment variables (e.g., `SPLUNK_PROFILE`, `SPLUNK_BASE_URL`)
3. Profile configuration (from config.json)
4. Default values

### Keybindings

<!-- BEGIN TUI KEYBINDINGS -->

### Navigation

- `?`: Help
- `q`: Quit
- `Ctrl+Q`: Quit (global)
- `Tab`: Next screen
- `Shift+Tab`: Previous screen
- `Ctrl+c`: Copy to clipboard

### Screen Specific Shortcuts

#### Search Screen

The Search screen has two input modes that affect how keys are handled:

**QueryFocused mode** (default): Type your search query. Printable characters (including `q`, `?`, digits) are inserted into the query. Use `Tab` to switch to ResultsFocused mode.

**ResultsFocused mode**: Navigate and control the application. Global shortcuts like `q` (quit) and `?` (help) work in this mode. Use `Tab` or `Esc` to return to QueryFocused mode.

- `Enter`: Run search
- `Ctrl+e`: Export results
- `Ctrl+c`: Copy query (or current result)
- `Up/Down`: Navigate history (query)
- `Ctrl+j/k`: Scroll results (while typing)
- `PgDn`: Page down
- `PgUp`: Page up
- `Home`: Go to top
- `End`: Go to bottom
- `j,k,...`: Type search query

#### Jobs Screen
- `r`: Refresh jobs
- `Ctrl+e`: Export jobs
- `Ctrl+c`: Copy selected SID
- `a`: Toggle auto-refresh
- `s`: Cycle sort column
- `/`: Filter jobs
- `Space`: Toggle job selection
- `c`: Cancel selected job(s)
- `d`: Delete selected job(s)
- `j/k or Up/Down`: Navigate list
- `Enter`: Inspect job

#### Job Details (Inspect) Screen
- `Esc`: Back to jobs
- `Ctrl+c`: Copy job SID

#### Indexes Screen
- `r`: Refresh indexes
- `Enter`: View index details
- `Ctrl+e`: Export indexes
- `Ctrl+c`: Copy selected index name
- `j/k or Up/Down`: Navigate list

#### Cluster Screen
- `r`: Refresh cluster info
- `p`: Toggle peers view
- `j/k or Up/Down`: Navigate peers list
- `Ctrl+e`: Export cluster info
- `Ctrl+c`: Copy cluster ID

#### Health Screen
- `r`: Refresh health status
- `Ctrl+e`: Export health info
- `Ctrl+c`: Copy health status

#### Saved Searches Screen
- `r`: Refresh saved searches
- `Ctrl+e`: Export saved searches
- `Ctrl+c`: Copy selected saved search name
- `Enter`: Run selected search
- `j/k or Up/Down`: Navigate list

#### Internal Logs Screen
- `r`: Refresh logs
- `Ctrl+e`: Export logs
- `a`: Toggle auto-refresh
- `Ctrl+c`: Copy selected log message
- `j/k or Up/Down`: Navigate list

#### Apps Screen
- `r`: Refresh apps
- `Ctrl+e`: Export apps
- `Ctrl+c`: Copy selected app name
- `j/k or Up/Down`: Navigate list
- `e`: Enable selected app
- `d`: Disable selected app

#### Users Screen
- `r`: Refresh users
- `Ctrl+e`: Export users
- `Ctrl+c`: Copy selected username
- `j/k or Up/Down`: Navigate list

#### Settings Screen
- `t`: Cycle theme
- `a`: Toggle auto-refresh
- `s`: Cycle sort column
- `d`: Toggle sort direction
- `c`: Clear search history
- `r`: Reload settings
<!-- END TUI KEYBINDINGS -->

### Screens

**Search**: Enter SPL queries and view results
**Indexes**: Browse and inspect indexes
**Cluster**: View cluster status and peer information
**Jobs**: Monitor and manage search jobs
**Health**: Comprehensive health monitoring

## Documentation

- [User Guide](docs/user-guide.md) - Task-oriented guide for CLI and TUI
- [Usage Guide](docs/usage.md) - Detailed technical reference and configuration
- Development: configure live server access via `.env.test` (untracked; copy from `.env.test.example`) or environment variables (`SPLUNK_BASE_URL`, `SPLUNK_USERNAME`, `SPLUNK_PASSWORD`).

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
