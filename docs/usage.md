# Usage Guide

Splunk TUI provides a terminal-based interface (TUI) and a command-line interface (CLI) for interacting with Splunk Enterprise.

## Configuration

`splunk-tui` can be configured using environment variables, command-line arguments, or a configuration file.

### Configuration File

The configuration file is stored in a platform-specific directory:
- Linux/macOS: `~/.config/splunk-tui/config.json`
- Windows: `%AppData%\splunk-tui\config.json`

> **Note:** Older versions stored the config at `~/.config/splunk-tui/splunk-tui/config.json`.
> On first run, `splunk-tui` / `splunk-cli` will automatically migrate it to the new location.

The file uses JSON format and can contain multiple named profiles:

```json
{
  "profiles": {
    "default": {
      "base_url": "https://localhost:8089",
      "username": "admin",
      "password": "changeme",
      "skip_verify": true
    },
    "production": {
      "base_url": "https://splunk.example.com:8089",
      "api_token": "your-secret-api-token",
      "timeout_seconds": 60,
      "max_retries": 5
    }
  }
}
```

### Secure Credential Storage

`splunk-tui` supports storing sensitive credentials (passwords and API tokens) in your system's secure keyring (e.g., macOS Keychain, Windows Credential Locker, or Linux Secret Service).

In the `config.json` file, you can specify that a value should be fetched from the keyring instead of being stored in plain text:

```json
{
  "profiles": {
    "default": {
      "base_url": "https://localhost:8089",
      "username": "admin",
      "password": { "keyring_account": "splunk-default-admin" }
    }
  }
}
```

When configured this way, `splunk-tui` will look up the password for the account `splunk-default-admin` under the service `splunk-tui`.

### Environment Variables

Environment variables take precedence over the configuration file.

**Note**: Empty environment variable values (e.g., `SPLUNK_API_TOKEN=""`) or whitespace-only values (e.g., `SPLUNK_TIMEOUT="  "`) are treated as unset and will not override values from the configuration file or other sources. This allows you to leave placeholder variables empty in `.env` files.

| Variable | Description |
|----------|-------------|
| `SPLUNK_CONFIG_PATH` | Path to a custom configuration file (overrides default location) - works for both CLI and TUI |
| `SPLUNK_BASE_URL` | Base URL of the Splunk server (e.g., `https://localhost:8089`) |
| `SPLUNK_USERNAME` | Username for session authentication |
| `SPLUNK_PASSWORD` | Password for session authentication |
| `SPLUNK_API_TOKEN` | API token for bearer authentication (preferred over username/password) |
| `SPLUNK_SKIP_VERIFY` | Skip TLS verification (`true` or `false`) |
| `SPLUNK_TIMEOUT` | Connection timeout in seconds |
| `SPLUNK_MAX_RETRIES` | Maximum number of retries for failed requests |
| `SPLUNK_PROFILE` | Name of profile to load from the configuration file |
| `SPLUNK_EARLIEST_TIME` | Default earliest time for searches (e.g., `-24h`, `2024-01-01T00:00:00`) [default: `-24h`] |
| `SPLUNK_LATEST_TIME` | Default latest time for searches (e.g., `now`) [default: `now`] |
| `SPLUNK_MAX_RESULTS` | Default maximum number of results per search [default: `1000`] |

### Cancellation (Ctrl+C / SIGINT)
Long-running commands can be interrupted with `Ctrl+C`:
- The CLI prints `^C` and `Operation cancelled by user` to **stderr**
- The CLI exits with standard Unix SIGINT exit code **130**
- Common cancellable operations: `search --wait`, `logs --tail`, `list-all`

## Security & Secret Management

Splunk TUI includes a **secret-commit guard** to prevent accidental leaks of credentials or private environment details.

### Secret-Commit Guard

The guard ensures that sensitive files are **not tracked** in your git repository. This is critical because `.gitignore` only prevents *new* files from being tracked; it does not protect files that have already been committed.

#### Forbidden Tracked Paths
- `.env`
- `.env.test`
- `docs/splunk-test-environment.md`
- `rust_out`

#### Running the Guard
You can run the guard manually:
```bash
make lint-secrets
```

The guard is also integrated into `make ci`, ensuring that local CI passes only when no secrets are tracked.

#### Remediation
If the guard fails, follow these steps to safely untrack the files while keeping your local copies:

1. **Untrack the files**:
   ```bash
   git rm --cached -- .env .env.test docs/splunk-test-environment.md rust_out
   ```

2. **Commit the removals**:
   ```bash
   git commit -m "chore(security): stop tracking local secret files"
   ```

3. **Verify**:
   ```bash
   make lint-secrets
   ```

#### Pre-commit Hook (Optional)
To catch leaks even earlier, you can install a local git pre-commit hook:
```bash
make install-hooks
```
This will run the secret guard every time you attempt to `git commit`.

### Hermetic Testing

To ensure tests are stable and not influenced by a developer's local environment, `splunk-tui` enforces **hermetic testing**:

- **Dotenv Isolation**: Loading of `.env` files is disabled by default during test runs (`make test` or `make ci`). This is controlled by the `DOTENV_DISABLED=1` environment variable.
- **Integration Tests**: CLI integration tests use a shared utility that explicitly sets `DOTENV_DISABLED=1` for the spawned process, ensuring they stay isolated even if run via `cargo test` directly.
- **Live Tests**: Tests that require a real Splunk server (run via `make test-live`) may explicitly enable dotenv loading or rely on environment variables. They should be configured via `.env.test` which is also protected by the secret-commit guard.
- **Validation**: Regression tests prove that local `.env` values are ignored during standard test runs.

---

## Command Line Interface (CLI)

The CLI tool is named `splunk-cli`.

### Global Options

- `-b, --base-url <URL>`: Splunk base URL
- `-u, --username <NAME>`: Username for session auth
- `-p, --password <PASS>`: Password for session auth
- `-a, --api-token <TOKEN>`: API token for bearer auth
- `--timeout <SECONDS>`: Connection timeout in seconds
- `--max-retries <NUMBER>`: Maximum number of retries for failed requests
- `--skip-verify`: Skip TLS certificate verification
- `--config-path <FILE>`: Path to a custom configuration file (overrides default location)
- `--profile <NAME>`: Config profile name to load
- `-o, --output <FORMAT>`: Output format (`json`, `table`, `csv`, `xml`) [default: `table`]
  - **Note**: For CSV and XML formats, nested JSON structures are automatically handled:
    - **CSV**: Nested objects are flattened using dot-notation (e.g., `user.address.city`). Arrays use indexed notation (e.g., `tags.0`, `tags.1`).
    - **XML**: Nested structures are preserved as hierarchical elements. Arrays become container elements with `<item>` children.
- `--output-file <FILE>`: Save command results to a file instead of printing to stdout
  - Creates parent directories if they don't exist
  - Overwrites existing files
  - Success message is printed to stderr: "Results written to <path> (<format> format)"
  - Cannot be used with `--tail` mode (logs command)
  - Example: `splunk-cli search "index=main" --wait --output-file results.json`

#### Cancellation (Ctrl+C / SIGINT)
Long-running commands can be interrupted with `Ctrl+C`:
- The CLI prints `^C` and `Operation cancelled by user` to **stderr**
- The CLI exits with standard Unix SIGINT exit code **130**
- Common cancellable operations: `search --wait`, `logs --tail`, `list-all`

### Commands

#### `search`
Execute a search query and return results.

```bash
splunk-cli search "index=main | head 10" --wait --earliest "-24h"
```

- `<query>`: The SPL search query
- `--wait`: Wait for the search to complete before returning results
- `-e, --earliest <TIME>`: Earliest time (e.g., `-24h`, `2024-01-01T00:00:00`)
- `-l, --latest <TIME>`: Latest time (e.g., `now`)
- `-c, --count <NUMBER>`: Maximum number of results to return [default: 100]

#### `indexes`
List and manage Splunk indexes.

```bash
splunk-cli indexes --detailed
splunk-cli indexes --count 30 --offset 30
```

- `-d, --detailed`: Show detailed information about each index
- `-c, --count <NUMBER>`: Maximum number of indexes to list [default: 30]
- `--offset <NUMBER>`: Offset into the index list (zero-based) [default: 0]

**Note (table output):** table output includes a pagination footer (e.g., `Showing 31-60 (page 2)`).

#### `cluster`
Show cluster status and configuration.

```bash
splunk-cli cluster --detailed
splunk-cli cluster --detailed --offset 50 --page-size 50
```

- `-d, --detailed`: Show detailed cluster information
- `--offset <NUMBER>`: Offset into the cluster peer list (zero-based) [default: 0] (only applies with `--detailed`)
- `--page-size <NUMBER>`: Number of peers per page [default: 50] (only applies with `--detailed`)

**Note (table output):** table output includes a pagination footer (e.g., `Showing 1-50 of 120 (page 1 of 3)`).

#### `jobs`
Manage search jobs.

```bash
# List all jobs
splunk-cli jobs --list

# Inspect a specific job for detailed information
splunk-cli jobs --inspect "1705852800.123"

# Cancel a specific job
splunk-cli jobs --cancel "1705852800.123"

# Delete a specific job
splunk-cli jobs --delete "1705852800.123"
```

- `--list`: List all search jobs (default)
- `--inspect <SID>`: Show detailed information for a specific job by SID (includes status, duration, event counts, disk usage, priority, label, etc.)
- `--cancel <SID>`: Cancel a specific job by SID
- `--delete <SID>`: Delete a specific job by SID
- `-c, --count <NUMBER>`: Maximum number of jobs to list [default: 50]

**Output formats for `--inspect`**: Supports `--output table` (default), `--output json`, `--output csv`, `--output xml`

#### `health`
Perform a comprehensive system health check.

```bash
splunk-cli health
```

#### `kvstore`
Show detailed KVStore status.

```bash
splunk-cli kvstore
```

#### `license`
Show license information, including usage, pools, and stacks.

```bash
splunk-cli license
splunk-cli -o json license
```

#### `users`
List all Splunk users.

```bash
splunk-cli users
splunk-cli users --count 10 --output table
```

- `-c, --count <NUMBER>`: Maximum number of users to list [default: 30]

#### `apps`
List and manage installed Splunk apps.

```bash
# List all apps
splunk-cli apps list

# List with count limit
splunk-cli apps list --count 10

# List with different output formats
splunk-cli apps list --output json
splunk-cli apps list --output csv
splunk-cli apps list --output xml

# Show detailed app information
splunk-cli apps info search
splunk-cli apps info launcher

# Enable an app
splunk-cli apps enable my_custom_app

# Disable an app
splunk-cli apps disable unused_app
```

**Subcommands:**
- `list` [options]: List installed apps
  - `-c, --count <NUMBER>`: Maximum number of apps to list [default: 30]
  - `-o, --output <FORMAT>`: Output format (table, json, csv, xml) [default: table]

- `info <APP_NAME>`: Show detailed information about an app
  - `-o, --output <FORMAT>`: Output format (table, json, csv, xml) [default: table]

- `enable <APP_NAME>`: Enable an app by name

- `disable <APP_NAME>`: Disable an app by name

**Output Formats:**
- **Table**: Human-readable formatted output (list: table view, info: detailed key-value pairs)
- **JSON**: Full app object(s) with all fields
- **CSV**: Comma-separated values with header row
- **XML**: Hierarchical XML structure with app elements

**Notes:**
- System apps may require admin privileges to enable/disable
- Some apps cannot be disabled (e.g., core Splunk apps)
- Use `apps list` first to find the exact app name

#### `internal-logs`
Show internal Splunk logs (from `index=_internal`).

```bash
splunk-cli internal-logs --count 50
```

- `-c, --count <NUMBER>`: Maximum number of log entries to return [default: 50]
- `-e, --earliest <TIME>`: Earliest time for logs [default: "-15m"]

**Note**: This command provides access to internal Splunk logs using the dedicated endpoint. For real-time streaming support, see the `logs` command.

#### `logs`
View internal logs with real-time streaming support.

```bash
splunk-cli logs --count 50 --earliest "-15m" --tail
```

- `-c, --count <NUMBER>`: Maximum number of log entries to show [default: 50]
- `-e, --earliest <TIME>`: Earliest time for logs [default: "-15m"]
- `-t, --tail`: Follow logs in real-time (streaming mode)

#### `saved-searches`
Manage Splunk saved searches.

```bash
# List all saved searches
splunk-cli saved-searches list

# List with count limit
splunk-cli saved-searches list --count 50

# List in different formats
splunk-cli saved-searches list --output json
splunk-cli saved-searches list --output csv
splunk-cli saved-searches list --output xml

# Show details for a specific saved search
splunk-cli saved-searches info "Errors Last 24 Hours"

# Run a saved search
splunk-cli saved-searches run "Errors Last 24 Hours"

# Run and wait for completion
splunk-cli saved-searches run "Errors Last 24 Hours" --wait

# Run with custom time range
splunk-cli saved-searches run "Errors Last 24 Hours" --earliest "-7d" --latest "now"

# Run and get results in JSON
splunk-cli saved-searches run "Errors Last 24 Hours" --wait --output json
```

**Subcommands:**
- `list` [options]: List saved searches
  - `-c, --count <NUMBER>`: Maximum number of saved searches to list [default: 30]
  - `-o, --output <FORMAT>`: Output format (table, json, csv, xml) [default: table]

- `info <NAME>`: Show detailed information about a saved search
  - `-o, --output <FORMAT>`: Output format (table, json, csv, xml) [default: table]

- `run <NAME>`: Execute a saved search by name
  - `-w, --wait`: Wait for the search to complete before returning results
  - `-e, --earliest <TIME>`: Earliest time for the search (e.g., `-24h`, `2024-01-01T00:00:00`)
  - `-l, --latest <TIME>`: Latest time for the search (e.g., `now`, `2024-01-02T00:00:00`)
  - `-o, --output <FORMAT>`: Output format for search results (table, json, csv, xml) [default: table]

**Output Formats:**
- **Table**: Human-readable formatted output (list: table view, info: detailed view)
- **JSON**: Full saved search object(s) with all fields
- **CSV**: Comma-separated values with header row
- **XML**: Hierarchical XML structure with saved-search elements

**Notes:**
- All saved searches (including disabled ones) are shown in list output
- The `run` subcommand extracts the search query from the saved search and executes it
- Use `saved-searches list` first to find the exact saved search name
- Time modifiers (`--earliest`, `--latest`) work the same as in the `search` command

#### `list-all`
List all Splunk resources in a unified overview.

```bash
splunk-cli list-all
splunk-cli list-all --resources indexes,jobs,users
splunk-cli list-all --output table
```

- `-r, --resources <TYPES>`: Optional comma-separated list of resource types to include (e.g., `indexes,jobs,users`)
  - Valid types: `indexes`, `jobs`, `apps`, `users`, `cluster`, `health`, `kvstore`, `license`, `saved-searches`
  - If not specified, all resource types are fetched
  - Example: `--resources indexes,jobs,health` fetches only those three resource types

**Error Handling:**
- Individual resource fetch failures do not stop the command
- Failed resources show status "error" with error message in Error column
- Non-clustered instances show cluster status "not clustered" (not error)
- License information unavailable shows status "unavailable"

**Timeout Behavior:**
- Each resource fetch has a 30-second timeout
- Timed-out resources show status "timeout"
- Other resources continue fetching if one times out

**Status Values:**
- `indexes`: "ok" or "error"
- `jobs`: "active" or "error"
- `apps`: "installed" or "error"
- `users`: "active" or "error"
- `cluster`: "standalone", "peer", "search-head", "not clustered", or "error"
- `health`: "healthy", "degraded", or "error"
- `kvstore`: "running", "stopped", or "error"
- `license`: "ok", "warning" (>90% usage), "unavailable", or "error"
- `saved-searches`: "available" or "error"

#### `config`
Manage configuration profiles.

```bash
splunk-cli -o json config list
splunk-cli config set my-profile --base-url https://localhost:8089 --username admin
splunk-cli config show my-profile
splunk-cli config edit my-profile --use-keyring
splunk-cli config delete my-profile
```

- `list`: List all configured profiles
- `set <profile-name>`: Create or update a profile
  - `-b, --base-url <URL>`: Base URL of Splunk server
  - `-u, --username <NAME>`: Username for session authentication
  - `-p, --password <PASS>`: Password for session authentication (will prompt if not provided)
  - `-a, --api-token <TOKEN>`: API token for bearer authentication (will prompt if not provided)
  - `-s, --skip-verify`: Skip TLS certificate verification
  - `-t, --timeout-seconds <SECONDS>`: Connection timeout
  - `-m, --max-retries <NUMBER>`: Maximum number of retries
  - `--use-keyring`: Store credentials in system keyring
- `show <profile-name>`: Display a profile's configuration
- `edit <profile-name>`: Edit a profile interactively
  - `--use-keyring`: Store credentials in system keyring
  - Prompts for each field with current values as defaults
  - Press Enter when prompted for password/token to keep existing values
- `delete <profile-name>`: Delete a profile

---

## Terminal User Interface (TUI)

Launch the TUI by running `splunk-tui`.

### TUI Options

The TUI (`splunk-tui`) supports the following command-line options:

| Option | Description |
|--------|-------------|
| `-p, --profile <NAME>` | Config profile name to load |
| `--config-path <FILE>` | Path to a custom configuration file |
| `--log-dir <DIR>` | Directory for log files [default: logs] |
| `--no-mouse` | Disable mouse support |
| `-h, --help` | Print help information |
| `-V, --version` | Print version information |

#### Configuration Precedence

Configuration values are loaded in the following precedence (highest to lowest):

1. **CLI arguments** (e.g., `--profile`, `--config-path`)
2. **Environment variables** (e.g., `SPLUNK_PROFILE`, `SPLUNK_BASE_URL`)
3. **Profile configuration** (from config.json)
4. **Default values**

Examples:

```bash
# Use a specific profile
splunk-tui --profile production

# Use a custom config file
splunk-tui --config-path /etc/splunk-tui/config.json

# Custom log directory and disable mouse
splunk-tui --log-dir /var/log/splunk-tui --no-mouse

# Combine options
splunk-tui --profile dev --log-dir ./logs --no-mouse
```

### Connection Context Header

The TUI header displays your current Splunk connection context to help you identify which environment you're working with:

- **Profile**: The active profile name (from `--profile` or `SPLUNK_PROFILE` env var)
- **Base URL**: The Splunk server URL (truncated if too long for the terminal width)
- **Auth Mode**: Shows `token` for API token auth, or `session (username)` for session auth
- **Server Version**: Splunk version number (fetched from server on startup)

Example header display:
```
Splunk TUI - Jobs | [+] Healthy
prod@splunk.company.com:8089 | token | v9.2.1
```

Before the server info is loaded, the header shows "Connecting..." as a placeholder.

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

### Index Details Popup

When viewing the Indexes screen, press `Enter` on a selected index to open the Index Details popup. This shows comprehensive metadata about the index:

- **Name**: Index name
- **Total Event Count**: Number of events in the index
- **Current DB Size**: Current storage usage in MB
- **Max Total Data Size**: Maximum allowed data size in MB (if configured)
- **Max Warm DB Count**: Maximum warm database count (if configured)
- **Max Hot Buckets**: Maximum hot buckets (if configured)
- **Frozen Time Period**: Retention period in seconds (converted to days for readability)
- **Home Path**: Path to hot/warm buckets
- **Cold DB Path**: Path to cold buckets (if configured)
- **Thawed Path**: Path for thawed data (if configured)
- **Cold to Frozen Dir**: Directory for frozen data (if configured)
- **Primary Index**: Whether this is a primary index

Navigate the Index Details popup:
- `j` / `↓` - Scroll down
- `k` / `↑` - Scroll up
- `PageDown` - Page down
- `PageUp` - Page up
- `Ctrl+c` - Copy full index JSON to clipboard
- `Esc` / `q` - Close popup

### Error Handling

When an operation fails, you will see an error toast in the bottom-right corner:
- Errors show a brief summary of the issue
- Press `e` when an error toast is visible to see full details
- Error details popup shows:
  - HTTP status code (when available)
  - Request URL (when available)
  - Error messages
  - Raw error details

Navigate error details popup:
- `j` / `↓` - Scroll down
- `k` / `↑` - Scroll up
- `PageDown` - Page down
- `PageUp` - Page up
- `Esc` / `q` / `e` - Close popup

Note: The `e` key is globally bound to show error details when an error toast is visible. This takes precedence over screen-specific bindings (like "enable app" on the Apps screen) because viewing error details is more urgent.

See the keybindings section above for screen-specific shortcuts.
- `a`: Toggle auto-refresh (polls every 5 seconds)
- `Ctrl+c`: Copy selected log message to clipboard
- `j` / `k`: Navigate the logs list

### Search Defaults

The TUI applies default search parameters to prevent unbounded searches that can overload Splunk servers. These defaults are:

- **Earliest time**: `-24h` (last 24 hours)
- **Latest time**: `now`
- **Max results**: `1000`

You can customize these defaults using environment variables (see [Environment Variables](#environment-variables) section):
- `SPLUNK_EARLIEST_TIME`: Override the default earliest time
- `SPLUNK_LATEST_TIME`: Override the default latest time
- `SPLUNK_MAX_RESULTS`: Override the default maximum results

The Settings screen displays the currently active search defaults. Values set via environment variables take precedence over persisted settings.

Search defaults are persisted to the configuration file and will be restored on the next run. Environment variables always override persisted values.
