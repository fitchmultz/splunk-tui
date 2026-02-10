//! CLI argument definitions and parsing.
//!
//! Responsibilities:
//! - Define the CLI structure using clap derive macros.
//! - Parse command-line arguments and environment variables.
//! - Provide config path resolution helpers.
//!
//! Does NOT handle:
//! - Command execution (see `dispatch` module).
//! - Configuration loading (see `config_context` module).
//!
//! Invariants:
//! - All arguments have sensible defaults where applicable
//! - Environment variable fallbacks are documented in --help

use clap::{Parser, Subcommand};
use splunk_config::constants::*;
use std::path::PathBuf;

use crate::commands;
use crate::dynamic_complete::CompletionType;

#[derive(Parser)]
#[command(name = "splunk-cli")]
#[command(about = "Splunk CLI - Manage Splunk Enterprise from the command line", long_about = None)]
#[command(version)]
#[command(
    after_help = "Examples:\n  splunk-cli search 'index=main | head 10' --wait\n  splunk-cli search validate 'index=main | stats count'\n  splunk-cli indexes --detailed\n  splunk-cli forwarders --detailed\n  splunk-cli health\n  splunk-cli doctor\n  splunk-cli doctor --bundle ./support-bundle.zip\n  splunk-cli list-all --all-profiles\n  splunk-cli --profile production jobs --list\n  splunk-cli --api-token $SPLUNK_API_TOKEN search 'index=_internal' --wait\n  splunk-cli jobs --results 1705852800.123 --result-count 100\n  splunk-cli jobs --results 1705852800.123 --result-offset 100 --result-count 50 -o json\n  splunk-cli jobs --results 1705852800.123 --output-file results.json\n\nShell Completions:\n  splunk-cli completions bash > /etc/bash_completion.d/splunk-cli\n  splunk-cli completions zsh > /usr/local/share/zsh/site-functions/_splunk-cli\n  splunk-cli completions fish > ~/.config/fish/completions/splunk-cli.fish\n\nManpage:\n  splunk-cli man > /usr/local/share/man/man1/splunk-cli.1\n"
)]
pub struct Cli {
    /// Base URL of the Splunk server (e.g., https://localhost:8089)
    #[arg(short, long, global = true, env = "SPLUNK_BASE_URL")]
    pub base_url: Option<String>,

    /// Username for session token authentication
    #[arg(short, long, global = true, env = "SPLUNK_USERNAME")]
    pub username: Option<String>,

    /// Password for session token authentication
    #[arg(short, long, global = true, env = "SPLUNK_PASSWORD")]
    pub password: Option<String>,

    /// API token for authentication (preferred over username/password)
    #[arg(long, global = true, env = "SPLUNK_API_TOKEN")]
    pub api_token: Option<String>,

    /// Connection timeout in seconds
    #[arg(long, global = true, env = "SPLUNK_TIMEOUT")]
    pub timeout: Option<u64>,

    /// Maximum number of retries for failed requests
    #[arg(long, global = true, env = "SPLUNK_MAX_RETRIES")]
    pub max_retries: Option<usize>,

    /// Skip TLS certificate verification (for self-signed certificates)
    #[arg(long, global = true, env = "SPLUNK_SKIP_VERIFY")]
    pub skip_verify: bool,

    /// Profile name to load from config file
    #[arg(long, global = true, env = "SPLUNK_PROFILE")]
    pub profile: Option<String>,

    /// Output format (json, table, csv, xml, ndjson, yaml, markdown)
    #[arg(short, long, global = true, default_value = "table")]
    pub output: String,

    /// Output file path (saves results to file instead of stdout)
    #[arg(long, global = true, value_name = "FILE")]
    pub output_file: Option<PathBuf>,

    /// Path to a custom configuration file (overrides default location).
    ///
    /// Can also be set via SPLUNK_CONFIG_PATH environment variable.
    #[arg(long, global = true, env = "SPLUNK_CONFIG_PATH", value_name = "FILE")]
    pub config_path: Option<PathBuf>,

    /// Password for configuration file encryption
    #[arg(long, global = true, env = "SPLUNK_CONFIG_PASSWORD")]
    pub config_password: Option<String>,

    /// Environment variable containing the configuration encryption key (hex)
    #[arg(long, global = true, env = "SPLUNK_CONFIG_KEY_VAR")]
    pub config_key_var: Option<String>,

    /// Suppress all progress output (spinners / progress bars).
    ///
    /// Note: Progress indicators always write to STDERR; this flag disables them entirely.
    #[arg(long, global = true)]
    pub quiet: bool,

    /// Disable client-side response caching.
    ///
    /// By default, GET responses are cached to improve performance.
    /// Use this flag to always fetch fresh data from the server.
    #[arg(long, global = true)]
    pub no_cache: bool,

    /// Disable the circuit breaker for API calls.
    ///
    /// When enabled (default), the client will fail fast when an endpoint
    /// exceeds failure thresholds to prevent cascading failures.
    #[arg(long, global = true, env = "SPLUNK_CIRCUIT_BREAKER_DISABLED")]
    pub no_circuit_breaker: bool,

    /// Number of failures within window to open circuit.
    #[arg(long, global = true, env = "SPLUNK_CIRCUIT_FAILURE_THRESHOLD")]
    pub circuit_failure_threshold: Option<u32>,

    /// Time window for failure counting in seconds.
    #[arg(long, global = true, env = "SPLUNK_CIRCUIT_FAILURE_WINDOW")]
    pub circuit_failure_window: Option<u64>,

    /// Time to wait before attempting half-open in seconds.
    #[arg(long, global = true, env = "SPLUNK_CIRCUIT_RESET_TIMEOUT")]
    pub circuit_reset_timeout: Option<u64>,

    /// Number of requests allowed in half-open state.
    #[arg(long, global = true, env = "SPLUNK_CIRCUIT_HALF_OPEN_REQUESTS")]
    pub circuit_half_open_requests: Option<u32>,

    /// Enable Prometheus metrics endpoint and bind address (e.g., "localhost:9090")
    ///
    /// When enabled, exposes /metrics endpoint for Prometheus scraping.
    #[arg(long, global = true, env = "SPLUNK_METRICS_BIND")]
    pub metrics_bind: Option<String>,

    /// Enable OpenTelemetry tracing and specify OTLP endpoint (e.g., "http://localhost:4317")
    ///
    /// When enabled, traces are exported to the specified OTLP endpoint.
    /// Can also be set via SPLUNK_OTLP_ENDPOINT environment variable.
    #[arg(long, global = true, env = "SPLUNK_OTLP_ENDPOINT")]
    pub otlp_endpoint: Option<String>,

    /// Service name for OpenTelemetry traces
    ///
    /// Defaults to "splunk-cli". Can be customized when running multiple instances.
    #[arg(long, global = true, env = "SPLUNK_OTEL_SERVICE_NAME")]
    pub otel_service_name: Option<String>,

    /// Execute command as part of a transaction.
    ///
    /// When enabled, the operation is staged in a local transaction log instead
    /// of being executed immediately. Use 'splunk-cli transaction commit' to
    /// execute all staged operations atomically.
    #[arg(long, global = true)]
    pub transaction: bool,

    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand)]
pub enum Commands {
    /// Manage configuration profiles
    Config {
        #[command(subcommand)]
        command: commands::config::ConfigCommand,
    },

    /// Execute a search query or validate SPL syntax
    Search {
        #[command(subcommand)]
        command: Option<commands::search::SearchCommand>,

        /// The search query to execute (e.g., 'search index=main | head 10')
        /// (Deprecated: use 'search execute <QUERY>')
        #[arg(hide = true)]
        query: Option<String>,

        /// Wait for the search to complete before returning results
        /// (Deprecated: use 'search execute --wait')
        #[arg(long, hide = true)]
        wait: bool,

        /// Earliest time for the search (e.g., '-24h', '2024-01-01T00:00:00')
        /// (Deprecated: use 'search execute --earliest')
        #[arg(short, long, allow_hyphen_values = true, hide = true)]
        earliest: Option<String>,

        /// Latest time for the search (e.g., 'now', '2024-01-02T00:00:00')
        /// (Deprecated: use 'search execute --latest')
        #[arg(short, long, allow_hyphen_values = true, hide = true)]
        latest: Option<String>,

        /// Maximum number of results to return
        /// (Deprecated: use 'search execute --count')
        #[arg(short, long, hide = true)]
        count: Option<usize>,

        /// Run search in real-time mode
        /// (Deprecated: use 'search execute --realtime')
        #[arg(long, hide = true)]
        realtime: bool,

        /// Real-time window in seconds
        /// (Deprecated: use 'search execute --realtime-window')
        #[arg(long, requires = "realtime", hide = true)]
        realtime_window: Option<u64>,
    },

    /// List and manage indexes
    Indexes {
        #[command(subcommand)]
        command: commands::indexes::IndexesCommand,
    },

    /// List deployment clients (forwarders)
    Forwarders {
        /// Show detailed information about each forwarder
        #[arg(short, long)]
        detailed: bool,

        /// Maximum number of forwarders to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,

        /// Offset into the forwarder list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
    },

    /// List distributed search peers
    SearchPeers {
        /// Show detailed information about each search peer
        #[arg(short, long)]
        detailed: bool,

        /// Maximum number of search peers to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,

        /// Offset into the search peer list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
    },

    /// Show cluster status and manage cluster configuration
    Cluster {
        #[command(subcommand)]
        command: Option<commands::cluster::ClusterCommand>,

        /// Show detailed cluster information (deprecated: use 'cluster show --detailed')
        #[arg(short, long, hide = true)]
        detailed: bool,

        /// Offset into the cluster peer list (zero-based) (deprecated: use 'cluster show')
        #[arg(long, hide = true, default_value = "0")]
        offset: usize,

        /// Number of peers per page (deprecated: use 'cluster show')
        #[arg(long, hide = true, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,
    },

    /// Manage search jobs
    Jobs {
        #[command(subcommand)]
        command: Option<commands::jobs::JobsCommand>,

        /// List all search jobs (default action)
        #[arg(long, default_value = "true")]
        list: bool,

        /// Inspect a specific job by SID (show detailed information)
        #[arg(long, value_name = "SID", group = "action")]
        inspect: Option<String>,

        /// Cancel a specific job by SID
        #[arg(long, value_name = "SID", group = "action")]
        cancel: Option<String>,

        /// Delete a specific job by SID
        #[arg(long, value_name = "SID", group = "action")]
        delete: Option<String>,

        /// Retrieve results for a specific job by SID
        #[arg(long, value_name = "SID", group = "action")]
        results: Option<String>,

        /// Maximum number of results to retrieve (for --results)
        #[arg(long, value_name = "N")]
        result_count: Option<usize>,

        /// Offset into results for pagination (for --results)
        #[arg(long, value_name = "N", default_value = "0")]
        result_offset: usize,

        /// Maximum number of jobs to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE, visible_alias = "job-count")]
        count: usize,
    },

    /// Perform a comprehensive system health check
    Health,

    /// Run comprehensive diagnostics and validate configuration
    Doctor {
        /// Write a redacted support bundle to the specified path
        #[arg(long, value_name = "PATH")]
        bundle: Option<PathBuf>,

        /// Include recent splunk-tui logs in the bundle
        #[arg(long, requires = "bundle")]
        include_logs: bool,
    },

    /// Show KVStore status and manage collections
    Kvstore {
        #[command(subcommand)]
        command: commands::kvstore::KvstoreCommand,
    },

    /// Show and manage license information
    License {
        #[command(subcommand)]
        command: Option<commands::license::LicenseCommand>,
    },

    /// Show internal logs (index=_internal)
    Logs {
        /// Maximum number of log entries to show
        #[arg(short, long, default_value_t = DEFAULT_INTERNAL_LOGS_COUNT)]
        count: usize,

        /// Earliest time for logs (e.g., '-24h', '2024-01-01T00:00:00')
        #[arg(short, long, default_value = DEFAULT_INTERNAL_LOGS_EARLIEST_TIME, allow_hyphen_values = true)]
        earliest: String,

        /// Follow logs in real-time
        #[arg(short, long)]
        tail: bool,
    },

    /// List and manage users
    Users {
        #[command(subcommand)]
        command: commands::users::UsersCommand,
    },

    /// List and manage roles
    Roles {
        #[command(subcommand)]
        command: commands::roles::RolesCommand,
    },

    /// List and manage installed Splunk apps
    Apps {
        #[command(subcommand)]
        apps_command: commands::apps::AppsCommand,
    },

    /// List all Splunk resources in unified overview
    ListAll {
        /// Optional comma-separated list of resource types (e.g., 'indexes,jobs,apps')
        #[arg(short, long, value_delimiter = ',')]
        resources: Option<Vec<String>>,

        /// Comma-separated list of profile names to query (e.g., 'dev,prod')
        /// If not specified, uses the default profile or SPLUNK_PROFILE env var
        #[arg(long, value_delimiter = ',')]
        profiles: Option<Vec<String>>,

        /// Query all configured profiles
        #[arg(long, conflicts_with = "profiles")]
        all_profiles: bool,
    },

    /// List and manage saved searches
    SavedSearches {
        #[command(subcommand)]
        command: commands::saved_searches::SavedSearchesCommand,
    },

    /// List and manage search macros
    Macros {
        #[command(subcommand)]
        command: commands::macros::MacrosCommand,
    },

    /// List and manage data inputs (TCP, UDP, Monitor, Script)
    Inputs {
        #[command(subcommand)]
        command: commands::inputs::InputsCommand,
    },

    /// View and manage configuration files (props.conf, transforms.conf, etc.)
    Configs {
        #[command(subcommand)]
        command: commands::configs::ConfigsCommand,
    },

    /// List and manage fired alerts
    Alerts {
        #[command(subcommand)]
        command: commands::alerts::AlertsCommand,
    },

    /// View audit events
    Audit {
        #[command(subcommand)]
        command: commands::audit::AuditCommand,
    },

    /// List and manage dashboards
    Dashboards {
        #[command(subcommand)]
        command: commands::dashboards::DashboardsCommand,
    },

    /// List and manage data models
    Datamodels {
        #[command(subcommand)]
        command: commands::datamodels::DatamodelsCommand,
    },

    /// List and manage lookup tables (CSV-based lookups)
    Lookups {
        #[command(subcommand)]
        command: Option<commands::lookups::LookupsCommand>,

        /// Maximum number of lookup tables to list (deprecated: use 'lookups list')
        #[arg(short, long, hide = true, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,

        /// Offset into the lookup table list (zero-based) (deprecated: use 'lookups list')
        #[arg(long, hide = true, default_value = "0")]
        offset: usize,
    },

    /// List workload pools and rules
    Workload {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,

        /// Maximum number of items to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,

        /// Offset into the list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
    },

    /// Send events to Splunk via HTTP Event Collector (HEC)
    Hec {
        #[command(subcommand)]
        command: commands::hec::HecCommand,
    },

    /// Show search head cluster status and manage SHC configuration
    Shc {
        #[command(subcommand)]
        command: Option<commands::shc::ShcCommand>,

        /// Show detailed SHC information (deprecated: use 'shc show --detailed')
        #[arg(short, long, hide = true)]
        detailed: bool,

        /// Offset into the member list (zero-based) (deprecated: use 'shc show')
        #[arg(long, hide = true, default_value = "0")]
        offset: usize,

        /// Number of members per page (deprecated: use 'shc show')
        #[arg(long, hide = true, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,
    },

    /// Generate shell completion scripts
    Completions {
        /// Shell to generate completions for
        #[arg(value_enum)]
        shell: clap_complete::Shell,

        /// Enable dynamic completion support (includes helper functions)
        #[arg(long)]
        dynamic: bool,

        /// Cache TTL in seconds for dynamic completions
        #[arg(long, default_value = "60", requires = "dynamic")]
        completion_cache_ttl: u64,
    },

    /// Generate completion values for shell integration (internal use)
    #[command(hide = true)]
    Complete {
        /// Type of completion to generate
        #[arg(value_enum)]
        completion_type: CompletionType,

        /// Cache TTL in seconds
        #[arg(long, default_value = "60")]
        cache_ttl: u64,
    },

    /// Generate manpage
    Man,

    /// Manage multi-step configuration transactions
    Transaction {
        #[command(subcommand)]
        command: TransactionCommand,
    },
}

#[derive(Debug, Subcommand)]
pub enum TransactionCommand {
    /// Start a new transaction session
    Begin,
    /// Validate and execute all staged operations
    Commit {
        /// Preview planned changes without executing
        #[arg(long)]
        dry_run: bool,
    },
    /// Clear all staged operations
    Rollback,
    /// Show currently staged operations
    Status,
    /// Mark a savepoint for partial rollback
    Savepoint {
        /// Name of the savepoint
        #[arg(value_name = "NAME")]
        name: String,
    },
}
