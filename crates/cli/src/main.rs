//! Splunk CLI - Command-line interface for Splunk Enterprise.
//!
//! Responsibilities:
//! - Parse command-line arguments and environment variables.
//! - Execute Splunk REST API commands via the shared client library.
//! - Format and display results in various output formats (table, JSON, etc.).
//!
//! Does NOT handle:
//! - Core business logic or REST API implementation (see `crates/client`).
//! - Long-term persistence of search results.
//!
//! Invariants / Assumptions:
//! - `load_dotenv()` is called BEFORE CLI parsing to allow `.env` to provide clap defaults.
//! - Global options (like `--base-url`) are applied consistently across all subcommands.

mod cancellation;
mod commands;
mod formatters;
mod progress;

use anyhow::Result;
use clap::{Parser, Subcommand};
use splunk_config::SearchDefaultConfig;
use std::path::{Path, PathBuf};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use splunk_config::{ConfigLoader, env_var_or_none};

use crate::cancellation::{
    CancellationToken, SIGINT_EXIT_CODE, is_cancelled_error, print_cancelled_message,
};

/// Context for command execution, distinguishing between real and placeholder configs.
///
/// This enum provides compile-time guarantees that placeholder configs (used for
/// config management commands and multi-profile operations) cannot be accidentally
/// used for actual Splunk API connections.
enum ConfigCommandContext {
    /// A real, validated config loaded from profiles/environment/CLI args.
    /// Used for actual Splunk API operations.
    /// Includes search defaults for applying env var overrides to search parameters.
    Real(splunk_config::Config, SearchDefaultConfig),
    /// A placeholder config for commands that don't need real connection details.
    /// Only valid for Config commands and multi-profile ListAll operations.
    Placeholder,
}

impl ConfigCommandContext {
    /// Extract the real config, failing if this is a placeholder.
    ///
    /// Use this for commands that require actual connection details.
    fn into_real_config(self) -> anyhow::Result<splunk_config::Config> {
        match self {
            ConfigCommandContext::Real(config, _) => Ok(config),
            ConfigCommandContext::Placeholder => {
                anyhow::bail!(
                    "Internal error: attempted to use placeholder config for an operation requiring real connection details"
                )
            }
        }
    }

    /// Extract both the real config and search defaults, failing if this is a placeholder.
    ///
    /// Use this for commands that require actual connection details and search defaults.
    fn into_real_config_with_search_defaults(
        self,
    ) -> anyhow::Result<(splunk_config::Config, SearchDefaultConfig)> {
        match self {
            ConfigCommandContext::Real(config, search_defaults) => Ok((config, search_defaults)),
            ConfigCommandContext::Placeholder => {
                anyhow::bail!(
                    "Internal error: attempted to use placeholder config for an operation requiring real connection details"
                )
            }
        }
    }
}

#[derive(Parser)]
#[command(name = "splunk-cli")]
#[command(about = "Splunk CLI - Manage Splunk Enterprise from the command line", long_about = None)]
#[command(version)]
struct Cli {
    /// Base URL of the Splunk server (e.g., https://localhost:8089)
    #[arg(short, long, global = true, env = "SPLUNK_BASE_URL")]
    base_url: Option<String>,

    /// Username for session token authentication
    #[arg(short, long, global = true, env = "SPLUNK_USERNAME")]
    username: Option<String>,

    /// Password for session token authentication
    #[arg(short, long, global = true, env = "SPLUNK_PASSWORD")]
    password: Option<String>,

    /// API token for authentication (preferred over username/password)
    #[arg(short, long, global = true, env = "SPLUNK_API_TOKEN")]
    api_token: Option<String>,

    /// Connection timeout in seconds
    #[arg(long, global = true, env = "SPLUNK_TIMEOUT")]
    timeout: Option<u64>,

    /// Maximum number of retries for failed requests
    #[arg(long, global = true, env = "SPLUNK_MAX_RETRIES")]
    max_retries: Option<usize>,

    /// Skip TLS certificate verification (for self-signed certificates)
    #[arg(long, global = true, env = "SPLUNK_SKIP_VERIFY")]
    skip_verify: bool,

    /// Profile name to load from config file
    #[arg(long, global = true, env = "SPLUNK_PROFILE")]
    profile: Option<String>,

    /// Output format (json, table, csv, xml)
    #[arg(short, long, global = true, default_value = "table")]
    output: String,

    /// Output file path (saves results to file instead of stdout)
    #[arg(long, global = true, value_name = "FILE")]
    output_file: Option<PathBuf>,

    /// Path to a custom configuration file (overrides default location).
    ///
    /// Can also be set via SPLUNK_CONFIG_PATH environment variable.
    #[arg(long, global = true, env = "SPLUNK_CONFIG_PATH", value_name = "FILE")]
    config_path: Option<PathBuf>,

    /// Suppress all progress output (spinners / progress bars).
    ///
    /// Note: Progress indicators always write to STDERR; this flag disables them entirely.
    #[arg(long, global = true)]
    quiet: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
    },

    /// List and manage indexes
    Indexes {
        /// Show detailed information about each index
        #[arg(short, long)]
        detailed: bool,

        /// Maximum number of indexes to list
        #[arg(short, long, default_value = "30")]
        count: usize,

        /// Offset into the index list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
    },

    /// Show cluster status and configuration
    Cluster {
        /// Show detailed cluster information
        #[arg(short, long)]
        detailed: bool,

        /// Offset into the cluster peer list (zero-based). Only applies with --detailed.
        #[arg(long, default_value = "0")]
        offset: usize,

        /// Number of peers per page. Only applies with --detailed.
        #[arg(long = "page-size", default_value = "50")]
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

    /// Show KVStore status
    Kvstore,

    /// Show license information
    License(commands::license::LicenseArgs),

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

    /// Show internal Splunk logs
    InternalLogs {
        /// Maximum number of log entries to show
        #[arg(short, long, default_value = "50")]
        count: usize,

        /// Earliest time for logs (e.g., '-24h', '2024-01-01T00:00:00')
        #[arg(short, long, default_value = "-15m", allow_hyphen_values = true)]
        earliest: String,
    },

    /// List and manage users
    Users {
        /// Maximum number of users to list
        #[arg(short, long, default_value = "30")]
        count: usize,
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
}

/// Returns true if the path is empty or contains only whitespace.
fn path_is_blank(path: &Path) -> bool {
    path.to_string_lossy().trim().is_empty()
}

/// Normalizes the config path, ignoring empty or whitespace-only values.
/// This prevents empty environment variables or blank CLI flags from clobbering other sources.
/// If the resulting path is blank, it falls back to the environment variable (and normalizes that too).
fn resolve_config_path(path: Option<PathBuf>) -> Option<PathBuf> {
    let path = path.filter(|p| !path_is_blank(p));
    if path.is_none() {
        env_var_or_none("SPLUNK_CONFIG_PATH")
            .map(PathBuf::from)
            .filter(|p| !path_is_blank(p))
    } else {
        path
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file BEFORE CLI parsing so clap env defaults can read .env values
    ConfigLoader::new().load_dotenv()?;

    let mut cli = Cli::parse();

    // Resolve config path immediately after parsing to ensure blank values are ignored.
    // This handles both blank env vars and blank CLI flags consistently.
    cli.config_path = resolve_config_path(cli.config_path.take());

    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer())
        .init();

    // Determine if we need a real config or can use a placeholder
    // Config commands and multi-profile list-all don't need connection details
    let is_multi_profile_list_all = matches!(
        cli.command,
        Commands::ListAll {
            all_profiles: true,
            ..
        } | Commands::ListAll {
            profiles: Some(_),
            ..
        }
    );
    let needs_real_config =
        !matches!(cli.command, Commands::Config { .. }) && !is_multi_profile_list_all;

    // Build configuration only if needed
    let config = if needs_real_config {
        let mut loader = ConfigLoader::new();

        // Apply custom config path if provided (highest priority for loader setup)
        if let Some(ref path) = cli.config_path {
            loader = loader.with_config_path(path.clone());
        }

        // Load from profile if specified (lowest priority)
        if let Some(ref profile_name) = cli.profile {
            loader = loader
                .with_profile_name(profile_name.clone())
                .from_profile()?;
        }

        // Apply environment variable overrides (medium priority)
        loader = loader.from_env()?;

        // Apply CLI overrides (highest priority)
        if let Some(ref url) = cli.base_url {
            loader = loader.with_base_url(url.clone());
        }
        if let Some(ref username) = cli.username {
            loader = loader.with_username(username.clone());
        }
        if let Some(ref password) = cli.password {
            loader = loader.with_password(password.clone());
        }
        if let Some(ref token) = cli.api_token {
            loader = loader.with_api_token(token.clone());
        }
        if let Some(timeout_secs) = cli.timeout {
            loader = loader.with_timeout(std::time::Duration::from_secs(timeout_secs));
        }
        if let Some(retries) = cli.max_retries {
            loader = loader.with_max_retries(retries);
        }
        if cli.skip_verify {
            loader = loader.with_skip_verify(true);
        }

        // Build search defaults with env var overrides (matching TUI behavior)
        // Must be done before loader.build() since build() consumes the loader
        let search_defaults = loader.build_search_defaults(None);

        let config = loader.build()?;

        // Warn if using default credentials (security check)
        if config.is_using_default_credentials() {
            tracing::warn!(
                "Using default Splunk credentials (admin/changeme). \
                 These are for local development only - change before production use."
            );
        }

        Some((config, search_defaults))
    } else {
        None
    };

    // Create cancellation token and set up signal handling
    let cancel = CancellationToken::new();
    let cancel_clone = cancel.clone();

    tokio::spawn(async move {
        if let Err(e) = tokio::signal::ctrl_c().await {
            eprintln!("Failed to listen for Ctrl+C: {}", e);
            return;
        }
        cancel_clone.cancel();
    });

    // Wrap config in appropriate context based on command type
    let config_context = if let Some((config, search_defaults)) = config {
        ConfigCommandContext::Real(config, search_defaults)
    } else {
        ConfigCommandContext::Placeholder
    };

    // Execute command
    match run_command(cli, config_context, &cancel).await {
        Ok(()) => Ok(()),
        Err(e) if is_cancelled_error(&e) => {
            print_cancelled_message();
            std::process::exit(SIGINT_EXIT_CODE as i32);
        }
        Err(e) => Err(e),
    }
}

async fn run_command(
    cli: Cli,
    config: ConfigCommandContext,
    cancel_token: &CancellationToken,
) -> Result<()> {
    match cli.command {
        Commands::Config { command } => {
            // Config commands don't use the config parameter - they use ConfigManager directly
            // The config context is ignored here (can be Real or Placeholder)
            commands::config::run(
                command,
                &cli.output,
                cli.output_file.clone(),
                cli.config_path.clone(),
            )?;
        }
        Commands::Search {
            query,
            wait,
            earliest,
            latest,
            count,
        } => {
            let (config, search_defaults) = config.into_real_config_with_search_defaults()?;
            commands::search::run(
                config,
                query,
                wait,
                earliest.as_deref(),
                latest.as_deref(),
                count,
                &search_defaults,
                &cli.output,
                cli.quiet,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Indexes {
            detailed,
            count,
            offset,
        } => {
            let config = config.into_real_config()?;
            commands::indexes::run(
                config,
                detailed,
                count,
                offset,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Cluster {
            detailed,
            offset,
            page_size,
        } => {
            let config = config.into_real_config()?;
            commands::cluster::run(
                config,
                detailed,
                offset,
                page_size,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Jobs {
            list,
            inspect,
            cancel,
            delete,
            count,
        } => {
            let config = config.into_real_config()?;
            commands::jobs::run(
                config,
                list,
                inspect,
                cancel,
                delete,
                count,
                &cli.output,
                cli.quiet,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Health => {
            let config = config.into_real_config()?;
            commands::health::run(config, &cli.output, cli.output_file.clone(), cancel_token)
                .await?;
        }
        Commands::Kvstore => {
            let config = config.into_real_config()?;
            commands::kvstore::run(config, &cli.output, cli.output_file.clone(), cancel_token)
                .await?;
        }
        Commands::License(_args) => {
            let config = config.into_real_config()?;
            commands::license::run(config, &cli.output, cli.output_file.clone(), cancel_token)
                .await?;
        }
        Commands::Logs {
            count,
            earliest,
            tail,
        } => {
            let config = config.into_real_config()?;
            commands::logs::run(
                config,
                count,
                earliest,
                tail,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::InternalLogs { count, earliest } => {
            let config = config.into_real_config()?;
            commands::internal_logs::run(
                config,
                count,
                earliest,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Users { count } => {
            let config = config.into_real_config()?;
            commands::users::run(
                config,
                count,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::Apps { apps_command } => {
            let config = config.into_real_config()?;
            commands::apps::run(
                config,
                apps_command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::ListAll {
            resources,
            profiles,
            all_profiles,
        } => {
            // Load ConfigManager if multi-profile mode
            let config_manager = if all_profiles || profiles.is_some() {
                // Use custom config path if provided via CLI arg or env var (already resolved)
                if let Some(config_path) = &cli.config_path {
                    Some(splunk_config::ConfigManager::new_with_path(
                        config_path.clone(),
                    )?)
                } else {
                    Some(splunk_config::ConfigManager::new()?)
                }
            } else {
                None
            };

            // Determine which profiles to query
            let is_multi_profile = all_profiles || profiles.is_some();

            // Only extract real config for single-profile mode
            let config = if is_multi_profile {
                // Multi-profile mode doesn't use the config parameter
                // (it loads configs from ConfigManager)
                None
            } else {
                Some(config.into_real_config()?)
            };

            // Unwrap config for single-profile mode, use placeholder for multi-profile
            let config_for_list_all = config.unwrap_or(splunk_config::Config {
                connection: splunk_config::ConnectionConfig {
                    base_url: String::new(),
                    skip_verify: false,
                    timeout: std::time::Duration::from_secs(30),
                    max_retries: 3,
                    session_expiry_buffer_seconds: 60,
                    session_ttl_seconds: 3600,
                    health_check_interval_seconds: 60,
                },
                auth: splunk_config::AuthConfig {
                    strategy: splunk_config::types::AuthStrategy::SessionToken {
                        username: String::new(),
                        password: secrecy::SecretString::new(String::new().into()),
                    },
                },
            });

            commands::list_all::run(
                config_for_list_all,
                resources,
                profiles,
                all_profiles,
                config_manager,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
        Commands::SavedSearches { command } => {
            let config = config.into_real_config()?;
            commands::saved_searches::run(
                config,
                command,
                &cli.output,
                cli.output_file.clone(),
                cancel_token,
            )
            .await?;
        }
    }

    Ok(())
}
