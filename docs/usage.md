# Usage Guide

Splunk TUI provides a terminal-based interface (TUI) and a command-line interface (CLI) for interacting with Splunk Enterprise.

## Configuration

`splunk-tui` can be configured using environment variables, command-line arguments, or a configuration file.

### Configuration File

The configuration file is stored in a platform-specific directory:
- Linux/macOS: `~/.config/splunk-tui/config.json`
- Windows: `%AppData%\splunk-tui\config.json`

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

**Note**: Empty environment variable values (e.g., `SPLUNK_API_TOKEN=""`) are treated as unset and will not override values from the configuration file or other sources. This allows you to leave placeholder variables empty in `.env` files.

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

### Cancellation (Ctrl+C / SIGINT)
Long-running commands can be interrupted with `Ctrl+C`:
- The CLI prints `^C` and `Operation cancelled by user` to **stderr**
- The CLI exits with standard Unix SIGINT exit code **130**
- Common cancellable operations: `search --wait`, `logs --tail`, `list-all`

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
```

- `-f, --format <FORMAT>`: Output format (`json`, `table`, `csv`, `xml`) [default: `table`]

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
splunk-cli config list --output json
splunk-cli config set my-profile --base-url https://localhost:8089 --username admin
splunk-cli config show my-profile --output table
splunk-cli config edit my-profile --use-keyring
splunk-cli config delete my-profile
```

- `list`: List all configured profiles
  - `-o, --output <FORMAT>`: Output format (json, table). Default: json
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
  - `-o, --output <FORMAT>`: Output format (json, table, csv, xml). Default: table
- `edit <profile-name>`: Edit a profile interactively
  - `--use-keyring`: Store credentials in system keyring
  - Prompts for each field with current values as defaults
  - Press Enter when prompted for password/token to keep existing values
- `delete <profile-name>`: Delete a profile

---

## Terminal User Interface (TUI)

Launch the TUI by running `splunk-tui`.

### Navigation

- `1`-`9`: Switch between screens:
  1. **Search**: Execute and view search results
  2. **Indexes**: View index status and metrics
  3. **Cluster**: View cluster information
  4. **Jobs**: Manage active and historical search jobs
  5. **Health**: View system health status
  6. **Saved Searches**: View and run saved searches
  7. **Internal Logs**: View internal Splunk logs
  8. **Apps**: View installed Splunk apps
  9. **Users**: View users and their roles
  0. **Settings**: View and modify configuration options
- `j` / `Down Arrow`: Move selection down (use `Ctrl+j` in Search screen)
- `k` / `Up Arrow`: Move selection up (use `Ctrl+k` in Search screen)
- `?`: Show help popup
- `q`: Quit the application

### Screen Specific Shortcuts

#### Search Screen
- `Enter`: Execute the search query typed in the input box (adds to history)
- `e`: Export current search results to a file (JSON or CSV)
- `Ctrl+c`: Copy current search query to clipboard (if results are loaded, copies the “current” result JSON instead)
- `Up` / `Down`: Navigate through search history (query)
- `Ctrl+j` / `Ctrl+k`: Scroll search results (while typing)
- `j`, `k`, ...: Type search query characters directly
- `Backspace`: Delete character in the search input
- `PageUp` / `PageDown`: Scroll through search results
- `Home` / `End`: Jump to top or bottom of results

#### Jobs Screen
- `Enter`: View details for the selected job (Inspect mode)
- `e`: Export list of jobs to a file (JSON or CSV)
- `Ctrl+c`: Copy SID of the selected job to clipboard
- `Space`: Toggle selection for the job under cursor (multi-selection mode)
- `r`: Refresh list of jobs manually
- `a`: Toggle auto-refresh (polls every 5 seconds)
- `s`: Cycle through sort columns (SID, Status, Duration, Results, Events)
- `/`: Enter filter mode to search for specific jobs by SID or status
- `c`: Cancel selected job(s). If multiple jobs are selected, cancels all at once (requires confirmation). If none selected, cancels the job under cursor.
- `d`: Delete selected job(s). If multiple jobs are selected, deletes all at once (requires confirmation). If none selected, deletes the job under cursor.

#### Indexes / Cluster / Health Screens
- `r`: Refresh the data for the current screen
- `e`: Export screen data to a file (JSON or CSV)
- `Ctrl+c`: Copy screen-specific ID or status (Index name, Cluster ID, or Health status)

#### Apps Screen
- `r`: Refresh the list of installed apps
- `e`: Export list of apps to a file (JSON or CSV)
- `Ctrl+c`: Copy selected app name to clipboard
- Displays: App name, label, version, and disabled status

#### Users Screen
- `r`: Refresh to list of users
- `e`: Export list of users to a file (JSON or CSV)
- `Ctrl+c`: Copy selected username to clipboard
- Displays: Username, real name, roles, and last login time

#### Settings Screen
- `t`: Cycle theme (Default, Light, Dark, HighContrast). Theme is persisted across sessions.
- `a`: Toggle auto-refresh for jobs screen (polls every 5 seconds)
- `s`: Cycle through sort columns (SID, Status, Duration, Results, Events)
- `d`: Toggle sort direction (ascending/descending)
- `c`: Clear search history (removes all saved queries)
- `r`: Reload settings from config file (reverts any unsaved changes)
- Displays current configuration values:
  - Auto-refresh status
  - Sort column and direction
  - Search history count
  - Current profile name (if set via SPLUNK_PROFILE environment variable)

### Error Handling

When a search fails, you will see an error toast in the bottom-right corner:
- Errors show a brief summary of the issue
- Press `e` when an error toast is visible to see full details
- Error details popup shows:
  - HTTP status code
  - Request URL
  - Splunk request ID (for support)
  - Structured error messages from Splunk
  - Raw error response body

Navigate error details popup:
- `j` / `↓` - Scroll down
- `k` / `↑` - Scroll up
- `PageDown` - Page down
- `PageUp` - Page up
- `Esc` / `q` - Close popup

#### Job Details (Inspect) Screen
- `Esc`: Return to the main Jobs list
- `Ctrl+c`: Copy SID of the selected job to clipboard
- `?`: Show help popup
- `0`: Go to settings screen

#### Saved Searches Screen
- `r`: Refresh saved searches
- `e`: Export list of saved searches to a file (JSON or CSV)
- `Ctrl+c`: Copy selected saved search name to clipboard
- `Enter`: Run the selected saved search (switches to Search screen)
- `j` / `k`: Navigate the list of saved searches

#### Internal Logs Screen
- `r`: Refresh the internal logs list
- `e`: Export internal logs to a file (JSON or CSV)
- `a`: Toggle auto-refresh (polls every 5 seconds)
- `Ctrl+c`: Copy selected log message to clipboard
- `j` / `k`: Navigate the logs list
