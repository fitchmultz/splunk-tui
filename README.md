# Splunk CLI and TUI

A robust Rust-based CLI and TUI tool for managing Splunk Enterprise v9+ deployments. Supports both standalone and clustered deployments with dual authentication methods (session token and API token).

## Features

- **Dual Authentication**: Session token (username/password) and API token support
- **Search Management**: Create, execute, and monitor search jobs
- **SPL Validation**: Validate search syntax without executing (`splunk-cli search validate`)
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

## Quick Verification

After installation, verify your configuration and connectivity:

```bash
# Run diagnostics to validate config and connectivity
splunk-cli doctor

# Generate a support bundle for troubleshooting
splunk-cli doctor --bundle ./support-bundle.zip
```

The doctor command checks:
- Configuration loading and authentication setup
- Connectivity to your Splunk server
- Server info, health status, and license information
- KVStore status

If any required checks fail, the command exits with a non-zero status code.

For troubleshooting common issues, see the [Troubleshooting Guide](docs/user-guide.md#troubleshooting).

## Configuration

Configuration is loaded from environment variables or a `.env` file:

```bash
# Splunk Connection
export SPLUNK_BASE_URL=https://localhost:8089
export SPLUNK_USERNAME=admin
export SPLUNK_PASSWORD=changeme
export SPLUNK_API_TOKEN=your-api-token

# Optional Connection Settings
export SPLUNK_SKIP_VERIFY=false
export SPLUNK_TIMEOUT=30
export SPLUNK_MAX_RETRIES=3

# Session Management
export SPLUNK_SESSION_TTL=3600
export SPLUNK_SESSION_EXPIRY_BUFFER=60

# Search Defaults
export SPLUNK_EARLIEST_TIME=-24h
export SPLUNK_LATEST_TIME=now
export SPLUNK_MAX_RESULTS=1000

# TUI Settings
export SPLUNK_HEALTH_CHECK_INTERVAL=60
export SPLUNK_INTERNAL_LOGS_COUNT=100
export SPLUNK_INTERNAL_LOGS_EARLIEST=-15m

# Profile Selection
export SPLUNK_PROFILE=default
export SPLUNK_CONFIG_PATH=/path/to/config.json
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
splunk-cli indexes list

# Detailed information
splunk-cli indexes list --detailed

# Pagination
splunk-cli indexes list --count 30 --offset 30

# Create a new index
splunk-cli indexes create myindex --max-data-size-mb 1000

# Modify an index
splunk-cli indexes modify myindex --max-hot-buckets 10

# Delete an index
splunk-cli indexes delete myindex
```

### Cluster

```bash
# Show cluster status
splunk-cli cluster show

# Detailed cluster information including peers
splunk-cli cluster show --detailed

# List cluster peers
splunk-cli cluster peers

# Maintenance mode
splunk-cli cluster maintenance enable
splunk-cli cluster maintenance disable
splunk-cli cluster maintenance status

# Rebalance cluster
splunk-cli cluster rebalance

# Manage peers
splunk-cli cluster peers-manage decommission <peer_name>
splunk-cli cluster peers-manage remove <peer_guid>
```

### Jobs

```bash
# List all search jobs (default)
splunk-cli jobs
splunk-cli jobs --list
splunk-cli jobs --list --count 50

# Inspect a specific job
splunk-cli jobs --inspect 1705852800.123

# Cancel a job
splunk-cli jobs --cancel 1705852800.123

# Delete a job
splunk-cli jobs --delete 1705852800.123

# Retrieve job results
splunk-cli jobs --results 1705852800.123
splunk-cli jobs --results 1705852800.123 --result-count 500
```

### Saved Searches

```bash
# List all saved searches
splunk-cli saved-searches list

# Show detailed information
splunk-cli saved-searches info "My Saved Search"

# Run a saved search
splunk-cli saved-searches run "My Saved Search" --wait

# Create a new saved search
splunk-cli saved-searches create "New Search" --search "index=main | stats count"

# Edit a saved search
splunk-cli saved-searches edit "My Saved Search" --description "Updated description"

# Enable/disable a saved search
splunk-cli saved-searches enable "My Saved Search"
splunk-cli saved-searches disable "My Saved Search"

# Delete a saved search
splunk-cli saved-searches delete "My Saved Search"
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

# Custom config file location (or set SPLUNK_CONFIG_PATH env var)
splunk-tui --config-path /path/to/config.json

# Disable mouse support
splunk-tui --no-mouse

# Custom log directory
splunk-tui --log-dir /var/log/splunk-tui
```

Run `splunk-tui --help` for all available options.

#### Configuration Precedence

Configuration values are resolved in the following override precedence (highest to lowest):

1. **CLI arguments** (e.g., `--profile`, `--config-path`)
2. **Environment variables** (e.g., `SPLUNK_PROFILE`, `SPLUNK_BASE_URL`)
3. **Profile configuration** (from `config.json`)
4. **Default values**

**`.env` File Loading:**
The `.env` file is loaded early (before CLI parsing) to populate environment variables. This means:
- Values in `.env` become environment variable defaults
- CLI arguments still override `.env` values
- Set `DOTENV_DISABLED=1` to skip `.env` loading (useful for hermetic testing)

### Keybindings

<!-- BEGIN TUI KEYBINDINGS -->

### Navigation

- `?`: Help
- `Ctrl+P`: Command palette
- `q`: Quit
- `Ctrl+Q`: Quit (global)
- `Tab`: Next screen
- `Shift+Tab`: Previous screen
- `Ctrl+Tab`: Next focus
- `Ctrl+Shift+Tab`: Previous focus
- `Ctrl+c`: Copy to clipboard
- `e`: Show error details (when an error is present)
- `Ctrl+Z`: Undo last operation
- `Ctrl+Shift+Z`: Redo last undone operation

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
- `Ctrl+r`: Toggle real-time mode

#### Jobs Screen
- `r`: Refresh jobs
- `L`: Load more jobs
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
- `L`: Load more indexes
- `Enter`: View index details
- `Ctrl+e`: Export indexes
- `Ctrl+c`: Copy selected index name
- `j/k or Up/Down`: Navigate list
- `c`: Create new index
- `m`: Modify selected index
- `d`: Delete selected index

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

#### License Screen
- `r`: Refresh license info
- `Ctrl+e`: Export license info
- `Ctrl+c`: Copy license summary

#### KVStore Screen
- `r`: Refresh KVStore status
- `Ctrl+e`: Export KVStore status
- `Ctrl+c`: Copy KVStore status

#### Saved Searches Screen
- `r`: Refresh saved searches
- `Ctrl+e`: Export saved searches
- `Ctrl+c`: Copy selected saved search name
- `Enter`: Run selected search
- `j/k or Up/Down`: Navigate list
- `e`: Edit selected saved search
- `n`: Create new saved search
- `d`: Delete selected saved search
- `t`: Toggle saved search enabled/disabled state

#### Macros Screen
- `r`: Refresh macros
- `Ctrl+e`: Export macros
- `Ctrl+c`: Copy definition
- `y`: Copy definition (vim-style)
- `e`: Edit macro
- `n`: New macro
- `d`: Delete macro
- `j/k or Up/Down`: Navigate list
- `PgDn`: Page down
- `PgUp`: Page up
- `Home`: Go to top
- `End`: Go to bottom

#### Internal Logs Screen
- `r`: Refresh logs
- `L`: Load more logs
- `Ctrl+e`: Export logs
- `a`: Toggle auto-refresh
- `Ctrl+c`: Copy selected log message
- `j/k or Up/Down`: Navigate list

#### Apps Screen
- `r`: Refresh apps
- `L`: Load more apps
- `Ctrl+e`: Export apps
- `Ctrl+c`: Copy selected app name
- `j/k or Up/Down`: Navigate list
- `e`: Enable selected app
- `d`: Disable selected app
- `i`: Install app from .spl file
- `x`: Remove selected app

#### Users Screen
- `r`: Refresh users
- `L`: Load more users
- `Ctrl+e`: Export users
- `Ctrl+c`: Copy selected username
- `j/k or Up/Down`: Navigate list

#### Roles Screen
- `r`: Refresh roles
- `L`: Load more roles
- `c`: Create new role
- `m`: Modify selected role
- `d`: Delete selected role
- `Ctrl+e`: Export roles
- `Ctrl+c`: Copy selected role name
- `j/k or Up/Down`: Navigate list

#### Search Peers Screen
- `r`: Refresh search peers
- `Ctrl+e`: Export search peers
- `Ctrl+c`: Copy selected peer name
- `j/k or Up/Down`: Navigate list

#### Data Inputs Screen
- `r`: Refresh inputs
- `L`: Load more inputs
- `e`: Enable input
- `d`: Disable input
- `Ctrl+c`: Copy selected input name
- `j/k or Up/Down`: Navigate list

#### Configuration Files Screen
- `r`: Refresh config files
- `/`: Search stanzas
- `Enter`: View stanza details
- `h`: Go back
- `j/k or Up/Down`: Navigate list

#### Fired Alerts Screen
- `r`: Refresh fired alerts
- `Ctrl+e`: Export fired alerts
- `Ctrl+c`: Copy selected alert name
- `j/k or Up/Down`: Navigate list

#### Forwarders Screen
- `r`: Refresh forwarders
- `Ctrl+e`: Export forwarders
- `Ctrl+c`: Copy selected forwarder name
- `j/k or Up/Down`: Navigate list

#### Lookups Screen
- `r`: Refresh lookup tables
- `Ctrl+e`: Export lookup tables
- `Ctrl+c`: Copy selected lookup name
- `j/k or Up/Down`: Navigate list
- `d or Ctrl+d`: Download selected lookup as CSV
- `x or Ctrl+x`: Delete selected lookup (with confirmation)

#### Audit Events Screen
- `r`: Refresh audit events
- `Ctrl+e`: Export audit events
- `Ctrl+c`: Copy selected event
- `j/k or Up/Down`: Navigate list

#### Dashboards Screen
- `r`: Refresh dashboards
- `L`: Load more dashboards
- `j/k or Up/Down`: Navigate list

#### Data Models Screen
- `r`: Refresh data models
- `L`: Load more data models
- `j/k or Up/Down`: Navigate list

#### Workload Management Screen
- `r`: Refresh workload
- `w`: Toggle pools/rules
- `j/k or Up/Down`: Navigate list
- `Ctrl+e`: Export workload

#### SHC Screen
- `r`: Refresh SHC info
- `m`: Toggle members view
- `j/k or Up/Down`: Navigate members list
- `Ctrl+e`: Export SHC info
- `Ctrl+c`: Copy captain URI

#### Settings Screen
- `t`: Cycle theme
- `a`: Toggle auto-refresh
- `s`: Cycle sort column
- `d`: Toggle sort direction
- `c`: Clear search history
- `r`: Reload settings
- `p`: Switch profile
- `n`: Create new profile
- `e`: Edit selected profile
- `x`: Delete selected profile

#### Overview Screen
- `r`: Refresh overview
- `Ctrl+e`: Export overview
- `Ctrl+c`: Copy overview summary

#### Multi-Instance Dashboard Screen
- `r`: Refresh multi-instance dashboard
- `Ctrl+e`: Export multi-instance data
- `Ctrl+c`: Copy instance summary
- `j/k or Up/Down`: Navigate instances
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
