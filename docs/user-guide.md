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
splunk-cli indexes
```

**Get detailed index metrics:**
```bash
splunk-cli indexes --detailed
```

### Cluster & Health Monitoring

**Check overall system health:**
```bash
splunk-cli health
```

**View cluster peer status:**
```bash
splunk-cli cluster --detailed
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

---

## TUI Master Class

Launch the interactive interface with `splunk-tui`.

### Navigation Basics

- **Switch Screens**: Use keys `1` through `9`.
- **Global Shortcuts**: 
    - `q`: Quit.
    - `?`: Open help popup.
    - `r`: Refresh current screen data.
- **Scrolling**:
    - `j` / `k` or Arrow Keys: Move selection.
    - `PageUp` / `PageDown`: Scroll through long lists.
    - `Home` / `End`: Jump to top or bottom.

### The Search Screen (Key `1`)

The Search screen is optimized for rapid iteration.

- **Executing Queries**: Type your SPL and press `Enter`.
- **History**: Use `Up` and `Down` arrows to navigate previous searches.
- **Result Scrolling**: Use `Ctrl+j` and `Ctrl+k` to scroll the results while keeping focus on the input box.
- **Viewing Results**: Results are rendered as pretty-printed JSON objects.

### The Indexes Screen (Key `2`)

View and explore Splunk indexes.

- **Navigation**: Use `j`/`k` or arrow keys to move through the index list.
- **Refresh**: Press `r` to reload the indexes list.
- **Display**: Shows index name, current size, total size, and event count.

### The Jobs Screen (Key `4`)

Manage your search lifecycle in real-time.

- **Auto-Refresh**: Press `a` to toggle 5-second polling. Look for `[AUTO]` in the title.
- **Sorting**: Press `s` to cycle through sort columns (SID, Status, Duration, Results, Events).
- **Filtering**: Press `/` to search for specific jobs by SID or status.
- **Inspecting**: Select a job and press `Enter` to see full details in "Inspect Mode". Press `Esc` to return.
- **Lifecycle**: Press `c` to cancel or `d` to delete a selected job (requires confirmation).

### The Cluster Screen (Key `3`)

View Splunk cluster configuration and peer information.

- **Refresh**: Press `r` to reload cluster information.
- **Display**: Shows cluster master URI, replication status, and peer details.

### The Health Screen (Key `5`)

Monitor Splunk system health status.

- **Health Status Indicator**: Look at the header for `[+]` (Healthy), `[!]` (Unhealthy), or `[?]` (Unknown).
- **Refresh**: Press `r` to pull the latest metrics.
- **Sections**: The Health screen covers `splunkd` health, license usage, KVStore status, and log parsing issues.

### The Saved Searches Screen (Key `6`)

Browse and run pre-configured saved searches from your Splunk instance.

- **Navigation**: Use `j`/`k` or arrow keys to move through the list.
- **Running a Search**: Select a saved search and press `Enter` to load it into the Search screen and execute it automatically.
- **Refresh**: Press `r` to reload the saved searches list.

### The Internal Logs Screen (Key `7`)

Monitor Splunk's internal logs in real-time.

- **Auto-Refresh**: Press `a` to toggle 5-second polling. Look for `[AUTO]` in the title.
- **Navigation**: Use `j`/`k` or arrow keys to scroll through log entries.
- **Refresh**: Press `r` to pull the latest log entries.
- **Content**: Displays log level, timestamp, source component, and message.

### The Apps Screen (Key `8`)

View and manage installed Splunk applications.

- **Navigation**: Use `j`/`k` or arrow keys to browse the apps list.
- **Refresh**: Press `r` to reload the apps list.
- **Display**: Shows app name, label, version, and whether it is disabled.

### The Users Screen (Key `9`)

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

### Common Errors

- **`AuthFailed`**: Verify your username/password or API token. If using session auth, ensure your password hasn't expired.
- **`HttpError / TlsError`**: Usually caused by connectivity issues or untrusted SSL certificates. Try setting `SPLUNK_SKIP_VERIFY=true`.
- **`ApiError (404)`**: The endpoint might not exist on your version of Splunk. Ensure you are running v9.0+.
- **`SessionExpired`**: The TUI handles auto-renewal, but if you leave it idle for a very long time, you might need to restart.
- **`RateLimited (429)`**: Splunk is throttling requests. The tool will automatically retry with backoff, but you may need to reduce search frequency.

### Connectivity Check

If you can't connect, try a simple `curl` to verify the API is reachable:
```bash
curl -k -u admin:password https://your-splunk-host:8089/services/server/info
```

If `curl` works but the TUI doesn't, check your `.env` settings and profile configuration.
