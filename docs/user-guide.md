# Splunk TUI & CLI User Guide

Welcome to the comprehensive user guide for Splunk TUI and CLI. This guide is designed to help you get the most out of your Splunk interaction from the terminal, whether you prefer a quick command-line tool or an immersive interactive interface.

---

## Table of Contents

1. [Introduction](#introduction)
2. [Getting Started](#getting-started)
3. [CLI Deep Dive](#cli-deep-dive)
    - [Searching Splunk](#searching-splunk)
    - [Managing Jobs](#managing-jobs)
    - [Inspecting Indexes](#inspecting-indexes)
    - [Cluster & Health Monitoring](#cluster--health-monitoring)
    - [Configuration Profiles](#configuration-profiles)
4. [TUI Master Class](#tui-master-class)
    - [Navigation Basics](#navigation-basics)
    - [The Search Screen](#the-search-screen)
    - [The Indexes Screen](#the-indexes-screen)
    - [The Jobs Screen](#the-jobs-screen)
    - [The Cluster Screen](#the-cluster-screen)
    - [The Health Screen](#the-health-screen)
    - [The Saved Searches Screen](#the-saved-searches-screen)
    - [The Internal Logs Screen](#the-internal-logs-screen)
    - [The Apps Screen](#the-apps-screen)
    - [The Users Screen](#the-users-screen)
    - [Mouse Support](#mouse-support)
5. [Search Syntax Tips](#search-syntax-tips)
6. [Troubleshooting](#troubleshooting)

---

## Introduction

Splunk TUI provides two main ways to interact with your Splunk environment:
- **`splunk-cli`**: A powerful CLI for automation, quick checks, and piping data to other tools.
- **`splunk-tui`**: A terminal-based dashboard for real-time monitoring, interactive searching, and job management.

Both tools share a common configuration and support modern Splunk authentication (Session and API Tokens).

## Getting Started

Before using the tools, you need to configure your connection to Splunk.

1. Create a `.env` file in your project root or set environment variables:
   ```bash
   export SPLUNK_BASE_URL=https://your-splunk-instance:8089
   export SPLUNK_API_TOKEN=your-secret-token
   # Or use username/password
   # export SPLUNK_USERNAME=admin
   # export SPLUNK_PASSWORD=changeme
   ```
2. If you are using self-signed certificates (common in dev environments), enable:
   ```bash
   export SPLUNK_SKIP_VERIFY=true
   ```

Refer to the [README](../README.md) for full installation instructions.

---

## CLI Deep Dive

The `splunk-cli` command is your primary tool for non-interactive tasks.

### Searching Splunk

Execute searches directly and get results in various formats.

**Basic search (returns results immediately):**
```bash
splunk-cli search "index=main | head 5"
```

**Wait for search completion:**
By default, searches are asynchronous. Use `--wait` to block until completion.
```bash
splunk-cli search "index=main error | stats count by host" --wait
```

**Specifying time ranges:**
```bash
splunk-cli search "index=_internal" --earliest "-15m" --latest "now" --wait
```

**Output formatting:**
Use `-o` or `--output` to switch between `json`, `table`, `csv`, and `xml`.
```bash
splunk-cli search "index=main | head 5" -o table
```

### Managing Jobs

View and control your search jobs.

**List recent jobs:**
```bash
splunk-cli jobs --list
```

**Cancel or Delete a job:**
```bash
splunk-cli jobs --cancel 1705852800.123
splunk-cli jobs --delete 1705852800.123
```

### Inspecting Indexes

**List all indexes:**
```bash
splunk-cli indexes list
```

**Get detailed index metrics:**
```bash
splunk-cli indexes list --detailed
```

### Cluster & Health Monitoring

**Check overall system health:**
```bash
splunk-cli health
```

**View cluster status:**
```bash
splunk-cli cluster show
```

**View detailed cluster information including peers:**
```bash
splunk-cli cluster show --detailed
```

### Configuration Profiles

If you manage multiple Splunk environments, use profiles in `~/.config/splunk-tui/config.json`:

> **Note:** Older versions stored the config at `~/.config/splunk-tui/splunk-tui/config.json`.
> It will be automatically migrated to the new location on first run.

```json
{
  "profiles": {
    "prod": { "base_url": "https://prod:8089", "api_token": "..." },
    "dev": { "base_url": "https://localhost:8089", "username": "admin", "password": "..." }
  }
}
```

Switch profiles using the `--profile` flag:
```bash
splunk-cli --profile prod search "index=security | head 1"
```

### Configuration Precedence

When multiple configuration sources define the same value, the following override precedence applies (highest to lowest):

1. **CLI arguments** (e.g., `--profile`, `--base-url`)
2. **Environment variables** (e.g., `SPLUNK_PROFILE`, `SPLUNK_BASE_URL`)
3. **Profile configuration** (from `config.json`)
4. **Default values**

**`.env` File Behavior:**
The `.env` file is loaded before CLI parsing to populate environment variable defaults. This means:
- `.env` values are treated as environment variables (layer #2 above)
- CLI arguments still override `.env` values
- Set `DOTENV_DISABLED=1` to skip `.env` loading (useful for hermetic testing)

For example, if you have `SPLUNK_BASE_URL=https://localhost:8089` in your `.env` file but run:
```bash
splunk-cli --base-url https://prod.example.com:8089 health
```
The CLI argument (`https://prod.example.com:8089`) wins.

---

## TUI Master Class

Launch the interactive interface with `splunk-tui`.

### Navigation Basics

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

- **Scrolling**:
    - `j` / `k` or Arrow Keys: Move selection.
    - `PageUp` / `PageDown`: Scroll through long lists.
    - `Home` / `End`: Jump to top or bottom.

### The Search Screen

The Search screen is optimized for rapid iteration.

- **Executing Queries**: Type your SPL and press `Enter`.
- **Real-time Validation**: As you type, SPL syntax is validated after a brief pause (500ms). The query box border shows:
  - **Green** (`✓`): Valid SPL syntax
  - **Yellow** (`⚠`): Valid with warnings
  - **Red** (`✗`): Syntax errors (shown in status bar)
- **History**: Use `Up` and `Down` arrows to navigate previous searches.
- **Result Scrolling**: Use `Ctrl+j` and `Ctrl+k` to scroll the results while keeping focus on the input box.
- **Viewing Results**: Results are rendered as pretty-printed JSON objects.

### The Indexes Screen

View and explore Splunk indexes.

- **Navigation**: Use `j`/`k` or arrow keys to move through the index list.
- **Refresh**: Press `r` to reload the indexes list.
- **Display**: Shows index name, current size, total size, and event count.

### The Jobs Screen

Manage your search lifecycle in real-time.

- **Auto-Refresh**: Press `a` to toggle 5-second polling. Look for `[AUTO]` in the title.
- **Sorting**: Press `s` to cycle through sort columns (SID, Status, Duration, Results, Events).
- **Filtering**: Press `/` to search for specific jobs by SID or status.
- **Inspecting**: Select a job and press `Enter` to see full details in "Inspect Mode". Press `Esc` to return.
- **Lifecycle**: Press `c` to cancel or `d` to delete a selected job (requires confirmation).

### The Cluster Screen

View Splunk cluster configuration and peer information.

- **Refresh**: Press `r` to reload cluster information.
- **Summary View**: Shows cluster ID, mode, label, manager URI, replication factor, and search factor.
- **Peers View**: Press `p` to toggle to the peers list view, which displays:
  - Host name (with `[C]` indicator for the captain)
  - Status (Up/Down/Pending with color coding)
  - Peer state
  - Site
  - Port
  - Replication count and status
- **Navigation**: In Peers view, use `j/k` or `Up/Down` to navigate the peers list.

### The Health Screen

Monitor Splunk system health status.

- **Health Status Indicator**: Look at the header for `[+]` (Healthy), `[!]` (Unhealthy), or `[?]` (Unknown).
- **Refresh**: Press `r` to pull the latest metrics.
- **Sections**: The Health screen covers `splunkd` health, license usage, KVStore status, and log parsing issues.

### The Saved Searches Screen

Browse and run pre-configured saved searches from your Splunk instance.

- **Navigation**: Use `j`/`k` or arrow keys to move through the list.
- **Running a Search**: Select a saved search and press `Enter` to load it into the Search screen and execute it automatically.
- **Refresh**: Press `r` to reload the saved searches list.

### The Internal Logs Screen

Monitor Splunk's internal logs in real-time.

- **Auto-Refresh**: Press `a` to toggle 5-second polling. Look for `[AUTO]` in the title.
- **Navigation**: Use `j`/`k` or arrow keys to scroll through log entries.
- **Refresh**: Press `r` to pull the latest log entries.
- **Content**: Displays log level, timestamp, source component, and message.

### The Apps Screen

View and manage installed Splunk applications.

- **Navigation**: Use `j`/`k` or arrow keys to browse the apps list.
- **Refresh**: Press `r` to reload the apps list.
- **Display**: Shows app name, label, version, and whether it is disabled.

### The Users Screen

View user accounts and their roles.

- **Navigation**: Use `j`/`k` or arrow keys to browse the users list.
- **Refresh**: Press `r` to reload the users list.
- **Display**: Shows username, real name, assigned roles, and last login time.

### Mouse Support

Splunk TUI supports mouse interaction for most common tasks:
- **Tab Switching**: Click the screen names in the footer to navigate.
- **Selection**: Click rows in the Jobs or Indexes tables to select them.
- **Inspect**: Double-click a job to enter Inspect mode.
- **Scrolling**: Use the mouse wheel to scroll through lists and search results.

---

## Search Syntax Tips

Splunk TUI uses standard SPL (Search Processing Language). Here are a few tips for terminal users:

- **Limit your results**: Always include `| head 100` or similar when testing new queries to avoid overloading the TUI.
- **Use `fields`**: Reduce visual noise by selecting only the fields you need: `index=main | fields _time, host, source, msg`.
- **Streaming vs. Non-streaming**: Remember that commands like `stats` or `sort` require the search to finish before showing full results.

---

## Troubleshooting

### Using the Doctor Command

If you're experiencing issues with splunk-cli or splunk-tui, the `doctor` command is your first diagnostic tool:

```bash
splunk-cli doctor
```

This will validate your configuration, test connectivity, and report any issues found.

### Common Issues

**"Failed to build client"**
- Check that SPLUNK_BASE_URL is set correctly (e.g., `https://localhost:8089`)
- Verify the URL includes the scheme (http:// or https://)

**"Failed to connect"**
- Verify the Splunk server is running and accessible
- Check that SPLUNK_SKIP_VERIFY is set if using self-signed certificates
- Test network connectivity: `curl $SPLUNK_BASE_URL`

**Authentication failures**
- For API tokens: Ensure the token has not expired
- For username/password: Verify credentials work in the Splunk web UI

### Generating Support Bundles

When reporting issues, include a support bundle:

```bash
splunk-cli doctor --bundle ./support-bundle.zip
```

The bundle contains redacted diagnostic information safe to share:
- Configuration summary (secrets removed)
- Environment variable names (values redacted)
- Diagnostic check results
- Health endpoint responses

### Common Errors

- **`AuthFailed`**: Verify your username/password or API token. If using session auth, ensure your password hasn't expired.
- **`HttpError / TlsError`**: Usually caused by connectivity issues or untrusted SSL certificates. Try setting `SPLUNK_SKIP_VERIFY=true`.
- **`ApiError (404)`**: The endpoint might not exist on your version of Splunk. Ensure you are running v9.0+.
- **`SessionExpired`**: The TUI handles auto-renewal, but if you leave it idle for a very long time, you might need to restart.
- **`RateLimited (429)`**: Splunk is throttling requests. The client automatically retries with exponential backoff, respecting the `Retry-After` header if present. If retries are exhausted, you'll see a `MaxRetriesExceeded` error. Reduce search frequency or increase `SPLUNK_MAX_RETRIES` if this occurs frequently.
- **`MaxRetriesExceeded`**: All retry attempts were exhausted. This can occur due to sustained rate limiting (429), transient server errors (502/503/504), or network issues. The error includes the underlying cause and the number of attempts made. Consider increasing `SPLUNK_MAX_RETRIES` or implementing application-level circuit breaking.

### Connectivity Check

If you can't connect, try a simple `curl` to verify the API is reachable:
```bash
curl -k -u admin:password https://your-splunk-host:8089/services/server/info
```

If `curl` works but the TUI doesn't, check your `.env` settings and profile configuration.

### Retry Behavior and Limitations

The client implements automatic retry with exponential backoff for transient failures. Understanding when retries occur (and when they don't) can help diagnose issues:

**When Retries Occur:**
- HTTP 429 (rate limiting), 502, 503, 504 (transient server errors)
- Transport errors: connection refused, reset, timeout, DNS failures
- Default: 3 retries with 1s, 2s, 4s delays

**When Retries Do NOT Occur:**
- Streaming requests (file uploads, large data streams) cannot be retried because the body is consumed on the first attempt
- Client errors (400, 401, 403, 404) fail immediately
- Server error 500 (Internal Server Error) is not retried as it typically indicates a bug, not a transient issue

**Tuning Retry Behavior:**
- Increase `SPLUNK_MAX_RETRIES` for unreliable networks or heavily loaded servers
- The client respects `Retry-After` headers for rate-limited responses
- For streaming uploads that must be reliable, buffer data in memory or implement application-level retry
