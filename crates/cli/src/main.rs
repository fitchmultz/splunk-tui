//! Splunk CLI - Command-line interface for Splunk Enterprise.
//!
//! This CLI provides tools for managing Splunk deployments, running searches,
//! and managing indexes and clusters.

mod commands;
mod formatters;

use anyhow::Result;
use clap::{Parser, Subcommand};
use tracing_subscriber::{EnvFilter, fmt, prelude::*};

use splunk_config::ConfigLoader;

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
        #[arg(short, long)]
        earliest: Option<String>,

        /// Latest time for the search (e.g., 'now', '2024-01-02T00:00:00')
        #[arg(short, long)]
        latest: Option<String>,

        /// Maximum number of results to return
        #[arg(short, long, default_value = "100")]
        count: usize,
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
        #[arg(short, long, default_value = "-15m")]
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
        #[arg(short, long, default_value = "-15m")]
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
    },

    /// List and manage saved searches
    SavedSearches {
        #[command(subcommand)]
        command: commands::saved_searches::SavedSearchesCommand,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    // Load .env file BEFORE CLI parsing so clap env defaults can read .env values
    ConfigLoader::new().load_dotenv()?;

    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer())
        .init();

    // Build configuration
    let mut loader = ConfigLoader::new();

    // Only build config if not running config command (it manages its own profiles)
    let config = if matches!(cli.command, Commands::Config { .. }) {
        // Config command doesn't need full config, return minimal placeholder
        splunk_config::Config {
            connection: splunk_config::ConnectionConfig {
                base_url: std::env::var("SPLUNK_BASE_URL").unwrap_or_default(),
                skip_verify: false,
                timeout: std::time::Duration::from_secs(30),
                max_retries: 3,
            },
            auth: splunk_config::AuthConfig {
                strategy: splunk_config::types::AuthStrategy::SessionToken {
                    username: std::env::var("SPLUNK_USERNAME").unwrap_or_default(),
                    password: secrecy::SecretString::new(
                        std::env::var("SPLUNK_PASSWORD").unwrap_or_default().into(),
                    ),
                },
            },
        }
    } else {
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

        loader.build()?
    };

    // Execute command
    run_command(cli, config).await
}

async fn run_command(cli: Cli, config: splunk_config::Config) -> Result<()> {
    match cli.command {
        Commands::Config { command } => {
            commands::config::run(command)?;
        }
        Commands::Search {
            query,
            wait,
            earliest,
            latest,
            count,
        } => {
            commands::search::run(
                config,
                query,
                wait,
                earliest.as_deref(),
                latest.as_deref(),
                count,
                &cli.output,
            )
            .await?;
        }
        Commands::Indexes {
            detailed,
            count,
            offset,
        } => {
            commands::indexes::run(config, detailed, count, offset, &cli.output).await?;
        }
        Commands::Cluster {
            detailed,
            offset,
            page_size,
        } => {
            commands::cluster::run(config, detailed, offset, page_size, &cli.output).await?;
        }
        Commands::Jobs {
            list,
            inspect,
            cancel,
            delete,
            count,
        } => {
            commands::jobs::run(config, list, inspect, cancel, delete, count, &cli.output).await?;
        }
        Commands::Health => {
            commands::health::run(config, &cli.output).await?;
        }
        Commands::Kvstore => {
            commands::kvstore::run(config, &cli.output).await?;
        }
        Commands::License(args) => {
            commands::license::run(config, &args).await?;
        }
        Commands::Logs {
            count,
            earliest,
            tail,
        } => {
            commands::logs::run(config, count, earliest, tail, &cli.output).await?;
        }
        Commands::InternalLogs { count, earliest } => {
            commands::internal_logs::run(config, count, earliest, &cli.output).await?;
        }
        Commands::Users { count } => {
            commands::users::run(config, count, &cli.output).await?;
        }
        Commands::Apps { apps_command } => {
            commands::apps::run(config, apps_command, &cli.output).await?;
        }
        Commands::ListAll { resources } => {
            commands::list_all::run(config, resources, &cli.output).await?;
        }
        Commands::SavedSearches { command } => {
            commands::saved_searches::run(config, command, &cli.output).await?;
        }
    }

    Ok(())
}
