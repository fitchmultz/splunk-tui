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

> **Security Warning:** The example above shows Splunk's default credentials (`admin`/`changeme`).
> These credentials are **only appropriate for local development** against a local Splunk instance.
> **Always change default credentials before connecting to production Splunk servers.**
> Consider using API tokens or the system keyring for secure credential storage (see below).

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

**Configuration Loading Order:**

Configuration is resolved in the following override precedence (highest to lowest):

1. **CLI arguments** (highest priority, e.g., `--profile`, `--base-url`)
2. **Environment variables** (e.g., `SPLUNK_PROFILE`, `SPLUNK_BASE_URL`)
3. **Profile configuration** (from `config.json`)
4. **Default values** (lowest priority)

**`.env` File Behavior:**
The `.env` file is loaded before CLI parsing to populate environment variable defaults. This means:
- `.env` values are treated as environment variables (layer #2 above)
- CLI arguments still override `.env` values
- Set `DOTENV_DISABLED=1` to skip `.env` loading (useful for hermetic testing)

**Note**: 
- Empty environment variable values (e.g., `SPLUNK_API_TOKEN=""`) or whitespace-only values (e.g., `SPLUNK_TIMEOUT="  "`) are treated as unset and will not override values from the configuration file or other sources. This allows you to leave placeholder variables empty in `.env` files.
- Non-empty values are automatically trimmed (leading/trailing whitespace removed). For example, `SPLUNK_USERNAME=" admin "` will be treated as `admin`.

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
| `SPLUNK_SESSION_TTL` | Session token lifetime in seconds (default: 3600) |
| `SPLUNK_SESSION_EXPIRY_BUFFER` | Buffer before expiry to refresh tokens (default: 60) |
| `SPLUNK_EARLIEST_TIME` | Default earliest time for searches (e.g., `-24h`, `2024-01-01T00:00:00`) [default: `-24h`] |
| `SPLUNK_LATEST_TIME` | Default latest time for searches (e.g., `now`) [default: `now`] |
| `SPLUNK_MAX_RESULTS` | Default maximum number of results per search [default: `1000`] |
| `SPLUNK_INTERNAL_LOGS_COUNT` | Default number of internal log entries to fetch in TUI [default: `100`] |
| `SPLUNK_INTERNAL_LOGS_EARLIEST` | Default earliest time for internal logs in TUI [default: `-15m`] |

#### Retry Behavior

The client automatically retries transient failures using exponential backoff. This improves reliability when Splunk is temporarily overloaded or experiencing transient issues.

**Default Configuration:**
- Maximum retries: 3 (configurable via `SPLUNK_MAX_RETRIES` or the `--max-retries` CLI flag)
- Backoff delays: 1s, 2s, 4s (exponential: 2^attempt seconds)

**Retryable Conditions:**

The client retries requests that fail with:

- **HTTP 429** (Too Many Requests): Rate limiting from Splunk
- **HTTP 502** (Bad Gateway): Transient upstream server error
- **HTTP 503** (Service Unavailable): Transient server unavailability
- **HTTP 504** (Gateway Timeout): Transient timeout from upstream
- **Transport errors**: Connection refused, connection reset, broken pipe, timeout, DNS failures

**Retry-After Header Support:**

For 429 responses, the client respects the `Retry-After` header when present:
- Supports delay-seconds format (e.g., `"120"` for 120 seconds)
- Supports HTTP-date format (e.g., `"Wed, 21 Oct 2015 07:28:00 GMT"`)
- Uses the maximum of the server's suggested delay and the exponential backoff

**Streaming Request Limitation:**

Requests with streaming bodies (e.g., file uploads, large data streams) **cannot be retried** because the body can only be consumed once. If such a request fails with a retryable error:
- The request fails immediately without retry attempts
- A warning is logged indicating the request cannot be cloned for retry

Applications requiring retry guarantees for large uploads should buffer the data in memory or use non-streaming request bodies.

**Error Handling Example:**

When retries are exhausted, the client returns a `MaxRetriesExceeded` error containing the underlying failure:

```rust
use splunk_client::ClientError;

match client.search("index=main | head 10").await {
    Err(ClientError::MaxRetriesExceeded(attempts, cause)) => {
        eprintln!("Request failed after {} attempts: {}", attempts, cause);
        // Consider implementing circuit breaker or backing off longer
    }
    Err(e) => eprintln!("Request failed: {}", e),
    Ok(results) => println!("Results: {:?}", results),
}
```

#### Session Configuration

The session TTL and expiry buffer work together to control session token lifecycle:

- **`SPLUNK_SESSION_TTL`**: Total session lifetime in seconds (default: 3600 = 1 hour)
  - Increase this if your Splunk server has longer session timeouts
  - Decrease if you want more frequent re-authentication

- **`SPLUNK_SESSION_EXPIRY_BUFFER`**: Proactive refresh buffer in seconds (default: 60)
  - The client will refresh the session this many seconds before the TTL expires
  - Prevents race conditions where a token expires during an API call
  - Should be significantly smaller than the TTL

Example: With TTL=3600 and buffer=60, the client will proactively refresh the session after 3540 seconds (59 minutes), leaving a 60-second safety margin.

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

## Diagnostics

The `doctor` command provides comprehensive diagnostics for troubleshooting:

```bash
# Run all diagnostics
splunk-cli doctor

# Generate a redacted support bundle
splunk-cli doctor --bundle ./support-bundle.zip

# Include splunk-tui logs in the bundle
splunk-cli doctor --bundle ./support-bundle.zip --include-logs

# Output diagnostics as JSON
splunk-cli doctor --output json
```

### Redaction Guarantees

Support bundles generated by the doctor command contain only redacted information:
- Authentication credentials are never included
- Environment variable values are replaced with `***REDACTED***`
- Only environment variable names are captured
- Server base URLs are included (needed for support context)
- Configuration sources are described but secrets are omitted

### Exit Codes

| Code | Meaning |
|------|---------|
| 0 | All required checks passed |
| 1 | One or more required checks failed (config load, connectivity) |

### Diagnostic Checks

The doctor command runs the following checks:

1. **config_load** - Validates that configuration was loaded successfully
2. **auth_strategy** - Detects and reports the authentication method being used
3. **client_build** - Verifies that a Splunk client can be constructed
4. **server_connectivity** - Tests connection to the Splunk server and fetches server info
5. **license_status** - Checks if license information can be retrieved
6. **kvstore_status** - Reports KVStore health status

---

## Command Line Interface (CLI)

The CLI tool is named `splunk-cli`.

### Global Options

- `-b, --base-url <URL>`: Splunk base URL
- `-u, --username <NAME>`: Username for session auth
- `-p, --password <PASS>`: Password for session auth
- `--api-token <TOKEN>`: API token for bearer auth
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
- `--quiet`: Suppress all progress output (spinners / progress bars)
  - Progress indicators always write to STDERR; this flag disables them entirely
  - Command results are still printed to stdout; only progress UI is suppressed

#### Cancellation (Ctrl+C / SIGINT)
Long-running commands can be interrupted with `Ctrl+C`:
- The CLI prints `^C` and `Operation cancelled by user` to **stderr**
- The CLI exits with standard Unix SIGINT exit code **130**
- Common cancellable operations: `search --wait`, `logs --tail`, `list-all`

### Commands

#### `lookups`
List and manage lookup tables (CSV-based lookups).

**Subcommands:**

- `list`: List all lookup tables (default)
  - `-c, --count <NUMBER>`: Maximum number of lookup tables to list [default: 30]
  - `--offset <NUMBER>`: Offset into the lookup table list [default: 0]

- `download <NAME>`: Download a lookup table as CSV
  - `-o, --output <FILE>`: Output file path (defaults to `<NAME>.csv`)
  - `--app <APP>`: App namespace [default: search]
  - `--owner <OWNER>`: Owner namespace [default: -]

- `upload <FILE>`: Upload or replace a lookup table file
  - `-n, --name <NAME>`: Lookup name (defaults to filename without extension)
  - `--app <APP>`: App namespace [default: search]
  - `--owner <OWNER>`: Owner namespace [default: -]
  - `--sharing <LEVEL>`: Sharing level (user, app, global)

- `delete <NAME>`: Delete a lookup table file
  - `--app <APP>`: App namespace [default: search]
  - `--owner <OWNER>`: Owner namespace [default: -]
  - `-f, --force`: Skip confirmation prompt

```bash
# List all lookup tables
splunk-cli lookups list

# List with count limit
splunk-cli lookups list --count 50

# List with offset (pagination)
splunk-cli lookups list --count 30 --offset 30

# List in different formats
splunk-cli lookups list --output json
splunk-cli lookups list --output csv
splunk-cli lookups list --output xml

# Download a lookup to current directory
splunk-cli lookups download my_lookup

# Download to specific path
splunk-cli lookups download my_lookup -o /path/to/output.csv

# Upload a new lookup
splunk-cli lookups upload /path/to/data.csv --name my_lookup

# Delete a lookup (with confirmation)
splunk-cli lookups delete my_lookup

# Delete without confirmation
splunk-cli lookups delete my_lookup --force
```

**Output Fields:**
- **Name**: The lookup table name
- **Filename**: The actual filename of the lookup
- **Owner**: Owner of the lookup
- **App**: App context where the lookup is defined
- **Sharing**: Sharing level (user, app, global)
- **Size**: File size in human-readable format (table output) or bytes (CSV/JSON/XML)

**Notes:**
- Only CSV-based lookup files are listed (KV store lookups use a different endpoint)
- File sizes are shown in human-readable format (B, KB, MB, GB) in table output
- For backward compatibility, `splunk-cli lookups` without subcommand defaults to `list`

#### `hec`

Send events to Splunk via HTTP Event Collector (HEC).

**Environment Variables:**
- `SPLUNK_HEC_URL`: HEC endpoint URL (e.g., `https://localhost:8088`)
- `SPLUNK_HEC_TOKEN`: HEC authentication token

**Subcommands:**

- `send <EVENT>`: Send a single event
  - `<EVENT>`: The event data as JSON string or `@file.json` to read from file
  - `-u, --hec-url <URL>`: HEC endpoint URL (or use `SPLUNK_HEC_URL` env var)
  - `-t, --hec-token <TOKEN>`: HEC token (or use `SPLUNK_HEC_TOKEN` env var)
  - `-i, --index <INDEX>`: Destination index (optional)
  - `--source <SOURCE>`: Source field (optional)
  - `--sourcetype <SOURCETYPE>`: Sourcetype field (optional)
  - `--host <HOST>`: Host field (optional)
  - `--time <TIME>`: Event timestamp as Unix epoch (optional)

- `send-batch <FILE>`: Send batch of events from JSON file
  - `<FILE>`: Path to JSON file containing array of events
  - `-u, --hec-url <URL>`: HEC endpoint URL
  - `-t, --hec-token <TOKEN>`: HEC token
  - `--ndjson`: Use newline-delimited JSON format instead of JSON array

- `health`: Check HEC health endpoint
  - `-u, --hec-url <URL>`: HEC endpoint URL
  - `-t, --hec-token <TOKEN>`: HEC token

- `check-ack`: Check acknowledgment status for previously sent events
  - `-u, --hec-url <URL>`: HEC endpoint URL
  - `-t, --hec-token <TOKEN>`: HEC token
  - `-a, --ack-ids <IDS>`: Comma-separated list of acknowledgment IDs

**Examples:**

```bash
# Send a simple event
splunk-cli hec send '{"message": "Hello Splunk"}' --hec-url https://localhost:8088 --hec-token abc123

# Send with metadata
splunk-cli hec send '{"severity": "error", "message": "Disk full"}' \
  --hec-url https://localhost:8088 \
  --hec-token abc123 \
  --index main \
  --source myapp \
  --sourcetype json \
  --host server01

# Send event from file
splunk-cli hec send @event.json --hec-url https://localhost:8088 --hec-token abc123

# Send batch from file
splunk-cli hec send-batch events.json --hec-url https://localhost:8088 --hec-token abc123

# Send batch using NDJSON format
splunk-cli hec send-batch events.json --hec-url https://localhost:8088 --hec-token abc123 --ndjson

# Check health
splunk-cli hec health --hec-url https://localhost:8088 --hec-token abc123

# Check acknowledgment status
splunk-cli hec check-ack --hec-url https://localhost:8088 --hec-token abc123 --ack-ids 1,2,3
```

**Notes:**
- HEC uses a separate endpoint (typically port 8088) from the REST API (port 8089)
- HEC uses a separate authentication token with "Splunk" prefix (not "Bearer")
- The `send-batch` command expects a JSON file containing an array of event objects
- Use `--ndjson` for newline-delimited JSON format (one event per line)
- Acknowledgment IDs are returned when HEC acknowledgments are enabled on the server

#### `alerts`
View fired alerts and alert history.

```bash
# List all fired alerts
splunk-cli alerts list

# List with count limit
splunk-cli alerts list --count 50

# List in different formats
splunk-cli alerts list --output json
splunk-cli alerts list --output csv
splunk-cli alerts list --output xml

# Show details for a specific fired alert
splunk-cli alerts info "scheduler__admin__search__MyAlert_at_1351181001_5.31_1351181987"
```

**Subcommands:**
- `list` [options]: List fired alerts
  - `-c, --count <NUMBER>`: Maximum number of fired alerts to list [default: 30]
  - `-o, --output <FORMAT>`: Output format (table, json, csv, xml) [default: table]

- `info <NAME>`: Show detailed information about a fired alert
  - `-o, --output <FORMAT>`: Output format (table, json, csv, xml) [default: table]

**Output Fields:**
- **Name**: The alert identifier (scheduler format)
- **Saved Search**: Name of the saved search that triggered the alert
- **Severity**: Alert severity level (Info, Low, Medium, High, Critical)
- **Actions**: Actions triggered (email, webhook, etc.)
- **Trigger Time**: When the alert fired
- **Alert Type**: Type of alert (scheduled, realtime, etc.)

**Notes:**
- Fired alerts are read-only historical records
- Alert configuration is managed via `saved-searches` command
- Use `--output json` for full alert details including SID and trigger metadata

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
- `--realtime`: Run search in real-time mode
- `--realtime-window <SECONDS>`: Real-time window in seconds (e.g., 60 for a 60-second window). Only valid with `--realtime`.

**Real-time Searches:**

Real-time searches continuously stream results as events arrive. Unlike normal searches that query historical data, real-time searches monitor incoming data within a sliding time window.

```bash
# Real-time search with default window
splunk-cli search "index=main" --realtime --wait

# Real-time search with 60-second window
splunk-cli search "index=main" --realtime --realtime-window 60 --wait

# Real-time search that returns immediately with SID (no waiting)
splunk-cli search "index=main" --realtime
```

**Note:** Real-time searches run continuously until cancelled. Use `Ctrl+C` to cancel a running real-time search, or omit `--wait` to start the search without waiting for results.

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

#### `forwarders`
List deployment clients (forwarders) that have checked in with the deployment server.

```bash
splunk-cli forwarders --detailed
splunk-cli forwarders --count 30 --offset 30
splunk-cli forwarders --output json
```

- `-d, --detailed`: Show detailed information about each forwarder (includes client name, utsname, repository location, and server classes)
- `-c, --count <NUMBER>`: Maximum number of forwarders to list [default: 30]
- `--offset <NUMBER>`: Offset into the forwarder list (zero-based) [default: 0]

**Note (table output):** table output includes a pagination footer (e.g., `Showing 1-30 (page 1)`).

**Output fields:**
- **Name**: The forwarder identifier (typically hostname or client name)
- **Hostname**: The reported hostname of the forwarder
- **IP Address**: The IP address of the forwarder
- **Version**: The Splunk version running on the forwarder
- **Last Phone**: The last time the forwarder checked in with the deployment server

In detailed mode, additional fields are shown:
- **Client Name**: The configured client name on the forwarder
- **Utsname**: System information from the forwarder (OS, kernel, architecture)
- **Repository Location**: The configuration repository location for this forwarder
- **Server Classes**: The server classes this forwarder belongs to

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

**Retrieve job results by SID:**

```bash
# Get first 100 results from a job
splunk-cli jobs --results "1705852800.123"

# Get results with custom count
splunk-cli jobs --results "1705852800.123" --result-count 500

# Paginate through large result sets
splunk-cli jobs --results "1705852800.123" --result-offset 0 --result-count 100
splunk-cli jobs --results "1705852800.123" --result-offset 100 --result-count 100

# Export results to file
splunk-cli jobs --results "1705852800.123" --result-count 1000 --output-file results.json

# Get results in different formats
splunk-cli jobs --results "1705852800.123" -o csv > results.csv
splunk-cli jobs --results "1705852800.123" -o json > results.json
splunk-cli jobs --results "1705852800.123" -o xml > results.xml
```

- `--results <SID>`: Retrieve results for a specific job by SID
- `--result-count <N>`: Maximum number of results to retrieve [default: 100]
- `--result-offset <N>`: Offset into results for pagination [default: 0]

**Note:** The default count of 100 is a safety measure. Use `--result-count` to retrieve more results or `--output-file` for large exports.

#### `health`
Perform a comprehensive system health check.

```bash
splunk-cli health
```

#### `kvstore`
Show detailed KVStore status and manage collections.

```bash
# Show KVStore status
splunk-cli kvstore status

# List all collections
splunk-cli kvstore list
splunk-cli kvstore list --app search --count 50

# Create a new collection
splunk-cli kvstore create mycollection --app search

# Delete a collection
splunk-cli kvstore delete mycollection --app search

# Query collection data
splunk-cli kvstore data mycollection --query '{"field": "value"}'
```

#### `license`
Show license information, including usage, pools, and stacks.

```bash
splunk-cli license
splunk-cli -o json license
```

#### `users`
List and manage Splunk users.

```bash
# List all users
splunk-cli users list
splunk-cli users list --count 10 --output table

# Create a new user
splunk-cli users create johndoe --roles user,power --email john@example.com --realname "John Doe"
splunk-cli users create janedoe --roles admin --password "secure123"

# Modify an existing user
splunk-cli users modify johndoe --roles power --email newemail@example.com
splunk-cli users modify janedoe --password "newpassword456"

# Delete a user
splunk-cli users delete johndoe
splunk-cli users delete janedoe --force
```

**List options:**
- `-c, --count <NUMBER>`: Maximum number of users to list [default: 30]

**Create options:**
- `name`: Username (required)
- `-p, --password <PASSWORD>`: Initial password (will prompt if not provided)
- `-r, --roles <ROLES>`: Roles to assign (comma-separated, at least one required)
- `--realname <NAME>`: Real name of the user
- `--email <EMAIL>`: Email address of the user
- `--default-app <APP>`: Default app for the user

**Modify options:**
- `name`: Username (required)
- `-p, --password <PASSWORD>`: New password
- `-r, --roles <ROLES>`: Roles to assign (comma-separated, replaces existing)
- `--realname <NAME>`: Real name of the user
- `--email <EMAIL>`: Email address of the user
- `--default-app <APP>`: Default app for the user

**Delete options:**
- `name`: Username (required)
- `-f, --force`: Skip confirmation prompt

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

# Install an app from a .spl package
splunk-cli apps install /path/to/my_app.spl

# Remove an app (with confirmation prompt)
splunk-cli apps remove my_app

# Remove an app without confirmation
splunk-cli apps remove my_app --force
```

**Subcommands:**
- `list` [options]: List installed apps
  - `-c, --count <NUMBER>`: Maximum number of apps to list [default: 30]
  - `-o, --output <FORMAT>`: Output format (table, json, csv, xml) [default: table]

- `info <APP_NAME>`: Show detailed information about an app
  - `-o, --output <FORMAT>`: Output format (table, json, csv, xml) [default: table]

- `enable <APP_NAME>`: Enable an app by name

- `disable <APP_NAME>`: Disable an app by name

- `install <FILE_PATH>`: Install an app from a .spl package file

- `remove <APP_NAME>`: Remove (uninstall) an app by name
  - `-f, --force`: Skip confirmation prompt

**Output Formats:**
- **Table**: Human-readable formatted output (list: table view, info: detailed key-value pairs)
- **JSON**: Full app object(s) with all fields
- **CSV**: Comma-separated values with header row
- **XML**: Hierarchical XML structure with app elements

**Notes:**
- System apps may require admin privileges to enable/disable
- Some apps cannot be disabled (e.g., core Splunk apps)
- Use `apps list` first to find the exact app name

#### `logs`
View internal logs with real-time streaming support.

```bash
splunk-cli logs --count 50 --earliest "-15m" --tail
```

- `-c, --count <NUMBER>`: Maximum number of log entries to show [default: 50]
- `-e, --earliest <TIME>`: Earliest time for logs [default: "-15m"]
- `-t, --tail`: Follow logs in real-time (streaming mode)

**Tail Mode Output Formats:**

When using `--tail`, output is streamed continuously with format-specific behavior:

- **table/csv**: Header is printed once at the start, then log entries as they arrive
- **json**: NDJSON format (one JSON object per line) suitable for streaming parsers
- **xml**: XML declaration and root element printed once; note that the closing `</logs>` tag may not appear if the stream is interrupted

Examples:

```bash
# Tail logs in NDJSON format for processing with jq
splunk-cli logs --tail --output json | jq -r '.message'

# Tail logs to a CSV file (header written once)
splunk-cli logs --tail --output csv > logs.csv
```

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

# Edit a saved search query
splunk-cli saved-searches edit "Errors Last 24 Hours" --search "index=main | stats count"

# Disable a saved search
splunk-cli saved-searches edit "Errors Last 24 Hours" --disabled true

# Update description
splunk-cli saved-searches edit "Errors Last 24 Hours" --description "Updated description"
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
  - `-c, --count <NUMBER>`: Maximum number of results to return [default: 100]
  - `-o, --output <FORMAT>`: Output format for search results (table, json, csv, xml) [default: table]

- `edit <NAME>`: Edit an existing saved search
  - `-s, --search <QUERY>`: New search query (SPL)
  - `-d, --description <DESC>`: New description
  - `--disabled <BOOL>`: Enable/disable the saved search (true/false)

**Output Formats:**
- **Table**: Human-readable formatted output (list: table view, info: detailed view)
- **JSON**: Full saved search object(s) with all fields
- **CSV**: Comma-separated values with header row
- **XML**: Hierarchical XML structure with saved-search elements

**Notes:**
- All saved searches (including disabled ones) are shown in list output
- The `run` subcommand extracts the search query from the saved search and executes it
- The `edit` subcommand only updates provided fields; omitted fields retain their current values
- At least one field (`--search`, `--description`, or `--disabled`) must be provided when editing
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

**Multi-Profile Aggregation:**

Query multiple Splunk environments simultaneously:

```bash
# Query specific profiles
splunk-cli list-all --profiles dev,prod --resources indexes,health

# Query all configured profiles
splunk-cli list-all --all-profiles

# Multi-profile output with JSON
splunk-cli list-all --all-profiles --output json
```

- `--profiles <NAMES>`: Comma-separated list of profile names to query (e.g., `dev,prod`)
  - Profiles must be configured via `splunk-cli config set`
  - If a profile doesn't exist, the command fails fast with available profiles listed
- `--all-profiles`: Query all configured profiles
  - Cannot be used with `--profiles`

**Multi-Profile Output Schema (JSON):**

```json
{
  "timestamp": "2026-01-27T19:30:00+00:00",
  "profiles": [
    {
      "profile_name": "dev",
      "base_url": "https://dev.splunk.local:8089",
      "resources": [...],
      "error": null
    },
    {
      "profile_name": "prod",
      "base_url": "https://prod.splunk.com:8089",
      "resources": [...],
      "error": null
    }
  ]
}
```

**Error Handling:**
- Individual resource fetch failures do not stop the command
- Failed resources show status "error" with error message in Error column
- Non-clustered instances show cluster status "not clustered" (not error)
- License information unavailable shows status "unavailable"
- In multi-profile mode, profile-level errors don't stop other profiles from being queried

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
splunk-cli config edit my-profile
splunk-cli config delete my-profile
```

- `list`: List all configured profiles
- `set <profile-name>`: Create or update a profile
  - `-b, --base-url <URL>`: Base URL of Splunk server
  - `-u, --username <NAME>`: Username for session authentication
  - `-p, --password <PASS>`: Password for session authentication (⚠️ **discouraged**: visible in shell history)
  - `--password-stdin`: Read password from stdin (single line, trailing newline trimmed)
  - `--password-file <PATH>`: Read password from file (content trimmed)
  - `--api-token <TOKEN>`: API token for bearer authentication (⚠️ **discouraged**: visible in shell history)
  - `--api-token-stdin`: Read API token from stdin (single line, trailing newline trimmed)
  - `--api-token-file <PATH>`: Read API token from file (content trimmed)
  - `-s, --skip-verify`: Skip TLS certificate verification
  - `-t, --timeout <SECONDS>`: Connection timeout (updates the profile's stored timeout, not a one-off runtime override)
  - `-m, --max-retries <NUMBER>`: Maximum number of retries
  - `--plaintext`: Store credentials as plaintext instead of using system keyring
  - `--no-prompt`: Fail fast instead of prompting for missing values (useful for CI/automation)
- `show <profile-name>`: Display a profile's configuration
- `edit <profile-name>`: Edit a profile interactively
  - `--plaintext`: Store credentials as plaintext instead of using system keyring
  - Prompts for each field with current values as defaults
  - Press Enter when prompted for password/token to keep existing values
- `delete <profile-name>`: Delete a profile

**Secure Credential Input (Recommended for CI/CD):**

For automation and CI/CD pipelines, avoid passing secrets via command-line arguments. Use one of these secure alternatives:

```bash
# Option 1: Read from file (e.g., Docker secrets, Kubernetes secrets)
echo "$SPLUNK_PASSWORD" > /run/secrets/splunk-password
splunk-cli config set my-profile \
  --base-url https://splunk.example.com:8089 \
  --username admin \
  --password-file /run/secrets/splunk-password \
  --no-prompt

# Option 2: Pipe from environment variable or command
printf '%s' "$SPLUNK_PASSWORD" | splunk-cli config set my-profile \
  --base-url https://splunk.example.com:8089 \
  --username admin \
  --password-stdin \
  --no-prompt

# Option 3: Use API token from file
splunk-cli config set my-profile \
  --base-url https://splunk.example.com:8089 \
  --api-token-file /run/secrets/splunk-token \
  --no-prompt
```

**Security Notes:**
- `--password` and `--api-token` flags are convenient for interactive use but expose secrets in shell history and process listings
- Use `--no-prompt` in automation to ensure the command fails fast if credentials are missing (prevents hanging on interactive prompts)
- The `--password-*` and `--api-token-*` options are mutually exclusive - use only one method per secret
- File contents are trimmed (leading/trailing whitespace removed) when reading secrets
- Stdin input reads a single line and trims the trailing newline

##### Credential Storage Security

By default, `splunk-cli` and `splunk-tui` store credentials in your system's secure keyring (macOS Keychain, Windows Credential Locker, or Linux Secret Service). This provides the best security by keeping secrets out of the configuration file.

If the keyring is unavailable or you prefer to store credentials in the configuration file:
- CLI: Use the `--plaintext` flag with `config set` or `config edit`
- TUI: Uncheck "Use Keyring" in the profile creation/edit dialog

**Security Warning**: Storing credentials as plaintext in `config.json` is less secure and should be avoided in production environments.

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

Configuration values are resolved in the following override precedence (highest to lowest):

1. **CLI arguments** (e.g., `--profile`, `--config-path`)
2. **Environment variables** (e.g., `SPLUNK_PROFILE`, `SPLUNK_BASE_URL`)
3. **Profile configuration** (from `config.json`)
4. **Default values**

**`.env` File Behavior:**
The `.env` file is loaded before CLI parsing to populate environment variable defaults. This means:
- `.env` values are treated as environment variables (layer #2 above)
- CLI arguments still override `.env` values
- Set `DOTENV_DISABLED=1` to skip `.env` loading (useful for hermetic testing)

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

### Profile Switching

The TUI supports switching between configured profiles at runtime without restarting. This allows you to quickly switch between different Splunk environments (e.g., dev, staging, production).

**To switch profiles:**
1. Navigate to the **Settings** screen (`Tab`/`Shift+Tab`)
2. Press `p` to open the **Profile Selector** popup
3. Use `↑`/`↓` or `j`/`k` to navigate the profile list
4. Press `Enter` to select a profile
5. Press `Esc` to cancel without switching

**What happens during a profile switch:**
- The TUI authenticates with the new profile's credentials
- All cached data is cleared (jobs, indexes, searches, etc.)
- The connection context header is updated with the new profile info
- The current screen data is reloaded from the new server

**Error handling:**
- If authentication fails, an error toast is shown and you remain on the current profile
- If no profiles are configured, an error message guides you to use `splunk-cli` to create profiles

**Security notes:**
- Credentials from the previous profile are cleared from memory when switching
- The profile switch requires re-authentication; session tokens are not reused across profiles

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

### Internal Logs Defaults

The TUI applies default parameters when fetching internal logs from `index=_internal`:

- **Count**: `100` (number of log entries to fetch)
- **Earliest time**: `-15m` (last 15 minutes)

You can customize these defaults using environment variables:
- `SPLUNK_INTERNAL_LOGS_COUNT`: Override the default number of entries
- `SPLUNK_INTERNAL_LOGS_EARLIEST`: Override the default earliest time (e.g., `-1h` for last hour)

The Settings screen displays the currently active internal logs defaults. These defaults are persisted to the configuration file and will be restored on the next run.

### Keybinding Customization

You can customize a subset of global keybindings by adding a `keybind_overrides` section to your persisted state in `~/.config/splunk-tui/config.json`:

```json
{
  "state": {
    "keybind_overrides": {
      "quit": "Ctrl+x",
      "help": "F1",
      "next_screen": "Ctrl+n",
      "previous_screen": "Ctrl+p"
    }
  }
}
```

#### Supported Actions

| Action | Default | Description |
|--------|---------|-------------|
| `quit` | `q` | Quit the application |
| `help` | `?` | Open the help popup |
| `next_screen` | `Tab` | Navigate to the next screen |
| `previous_screen` | `Shift+Tab` | Navigate to the previous screen |

#### Key Syntax

Key combinations use the following format:

- Simple keys: `"q"`, `"x"`, `"1"`
- With modifiers: `"Ctrl+x"`, `"Shift+Tab"`, `"Alt+F4"`
- Special keys: `"F1"` through `"F20"`, `"Esc"`, `"Enter"`, `"Space"`, `"Home"`, `"End"`, `"PageUp"`, `"PageDown"`, `"Up"`, `"Down"`, `"Left"`, `"Right"`

#### Limitations

- Only global navigation keys can be customized. Screen-specific keybindings (like `r` for refresh) always use their default keys.
- Some key combinations are reserved and cannot be overridden (e.g., `Ctrl+c` for copy to clipboard).
- If you assign the same key to multiple actions, the application will log a warning and use the default bindings.

#### Validation

The application validates keybinding overrides at startup:

- Invalid syntax will be logged as a warning and ignored
- Conflicting keybindings (same key for multiple actions) will be detected and rejected
- Reserved keys will be rejected with a warning

If validation fails, the application will start with default keybindings.
