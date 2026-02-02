//! CLI argument definitions and parsing.
//!
//! Responsibilities:
//! - Define the CLI structure using clap derive macros.
//! - Parse command-line arguments and environment variables.
//! - Provide config path resolution helpers.
//!
//! Non-responsibilities:
//! - Does not execute commands (see `dispatch` module).
//! - Does not handle config loading (see `config_context` module).

use clap::{Parser, Subcommand};
use splunk_config::env_var_or_none;
use std::path::{Path, PathBuf};

use crate::commands;

#[derive(Parser)]
#[command(name = "splunk-cli")]
#[command(about = "Splunk CLI - Manage Splunk Enterprise from the command line", long_about = None)]
#[command(version)]
#[command(
    after_help = "Examples:\n  splunk-cli search 'index=main | head 10' --wait\n  splunk-cli indexes --detailed\n  splunk-cli forwarders --detailed\n  splunk-cli health\n  splunk-cli list-all --all-profiles\n  splunk-cli --profile production jobs --list\n  splunk-cli -a $SPLUNK_API_TOKEN search 'index=_internal' --wait\n"
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
    #[arg(short, long, global = true, env = "SPLUNK_API_TOKEN")]
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

    /// Output format (json, table, csv, xml)
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

    /// Suppress all progress output (spinners / progress bars).
    ///
    /// Note: Progress indicators always write to STDERR; this flag disables them entirely.
    #[arg(long, global = true)]
    pub quiet: bool,

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

    /// Execute a search query
    Search {
        /// The search query to execute (e.g., 'search index=main | head 10')
        query: String,

        /// Wait for the search to complete before returning results
        #[arg(long)]
        wait: bool,

        /// Earliest time for the search (e.g., '-24h', '2024-01-01T00:00:00')
        #[arg(short, long, allow_hyphen_values = true)]
        earliest: Option<String>,

        /// Latest time for the search (e.g., 'now', '2024-01-02T00:00:00')
        #[arg(short, long, allow_hyphen_values = true)]
        latest: Option<String>,

        /// Maximum number of results to return
        #[arg(short, long)]
        count: Option<usize>,

        /// Run search in real-time mode
        #[arg(long)]
        realtime: bool,

        /// Real-time window in seconds (e.g., 60 for a 60-second window)
        #[arg(long, requires = "realtime")]
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
        #[arg(short, long, default_value = "30")]
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
        #[arg(short, long, default_value = "30")]
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
        #[arg(long = "page-size", hide = true, default_value = "50")]
        page_size: usize,
    },

    /// Manage search jobs
    Jobs {
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

        /// Maximum number of jobs to list
        #[arg(short, long, default_value = "50")]
        count: usize,
    },

    /// Perform a comprehensive system health check
    Health,

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
        #[arg(short, long, default_value = "50")]
        count: usize,

        /// Earliest time for logs (e.g., '-24h', '2024-01-01T00:00:00')
        #[arg(short, long, default_value = "-15m", allow_hyphen_values = true)]
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

    /// List lookup tables (CSV-based lookups)
    Lookups {
        /// Maximum number of lookup tables to list
        #[arg(short, long, default_value = "30")]
        count: usize,

        /// Offset into the lookup table list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
    },

    /// List workload pools and rules
    Workload {
        /// Show detailed information
        #[arg(short, long)]
        detailed: bool,

        /// Maximum number of items to list
        #[arg(short, long, default_value = "30")]
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
}

/// Returns true if the path is empty or contains only whitespace.
pub(crate) fn path_is_blank(path: &Path) -> bool {
    path.to_string_lossy().trim().is_empty()
}

/// Normalizes the config path, ignoring empty or whitespace-only values.
/// This prevents empty environment variables or blank CLI flags from clobbering other sources.
/// If the resulting path is blank, it falls back to the environment variable (and normalizes that too).
pub fn resolve_config_path(path: Option<PathBuf>) -> Option<PathBuf> {
    let path = path.filter(|p| !path_is_blank(p));
    if path.is_none() {
        env_var_or_none("SPLUNK_CONFIG_PATH")
            .map(PathBuf::from)
            .filter(|p| !path_is_blank(p))
    } else {
        path
    }
}
