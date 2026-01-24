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

    /// Skip TLS certificate verification (for self-signed certificates)
    #[arg(long, global = true, env = "SPLUNK_SKIP_VERIFY")]
    skip_verify: bool,

    /// Profile name to load from config file
    #[arg(long, global = true, env = "SPLUNK_PROFILE")]
    profile: Option<String>,

    /// Output format (json, table, csv, xml)
    #[arg(short, long, global = true, default_value = "json")]
    output: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
    },

    /// Show cluster status and configuration
    Cluster {
        /// Show detailed cluster information
        #[arg(short, long)]
        detailed: bool,
    },

    /// Manage search jobs
    Jobs {
        /// List all search jobs (default action)
        #[arg(long, default_value = "true")]
        list: bool,

        /// Cancel a specific job by SID
        #[arg(long)]
        cancel: Option<String>,

        /// Delete a specific job by SID
        #[arg(long)]
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

        /// Follow the logs in real-time
        #[arg(short, long)]
        tail: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    // Initialize logging
    tracing_subscriber::registry()
        .with(EnvFilter::from_default_env())
        .with(fmt::layer())
        .init();

    // Build configuration
    let mut loader = ConfigLoader::new().load_dotenv()?;

    // Load from profile if specified
    if let Some(ref profile_name) = cli.profile {
        loader = loader
            .with_profile_name(profile_name.clone())
            .from_profile()?;
    }

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
    if cli.skip_verify {
        loader = loader.with_skip_verify(true);
    }

    let config = loader.build()?;

    // Execute command
    run_command(cli, config).await
}

async fn run_command(cli: Cli, config: splunk_config::Config) -> Result<()> {
    match cli.command {
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
        Commands::Indexes { detailed, count } => {
            commands::indexes::run(config, detailed, count, &cli.output).await?;
        }
        Commands::Cluster { detailed } => {
            commands::cluster::run(config, detailed, &cli.output).await?;
        }
        Commands::Jobs {
            list,
            cancel,
            delete,
            count,
        } => {
            commands::jobs::run(config, list, cancel, delete, count, &cli.output).await?;
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
    }

    Ok(())
}
