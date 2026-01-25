//! Saved searches command implementation.

use anyhow::{Context, Result};
use clap::Subcommand;
use splunk_client::SplunkClient;
use tracing::info;

use crate::commands::convert_auth_strategy;
use crate::formatters::{OutputFormat, get_formatter};

#[derive(Subcommand)]
pub enum SavedSearchesCommand {
    /// List saved searches
    List {
        /// Maximum number of saved searches to list
        #[arg(short, long, default_value = "30")]
        count: usize,
    },
    /// Show detailed information about a saved search
    Info {
        /// Name of the saved search
        #[arg(value_name = "NAME")]
        name: String,
    },
    /// Run a saved search by name
    Run {
        /// Name of the saved search to execute
        #[arg(value_name = "NAME")]
        name: String,
        /// Wait for the search to complete before returning results
        #[arg(long)]
        wait: bool,
        /// Earliest time for the search (e.g., '-24h', '2024-01-01T00:00:00')
        #[arg(short, long)]
        earliest: Option<String>,
        /// Latest time for the search (e.g., 'now', '2024-01-02T00:00:00')
        #[arg(short, long)]
        latest: Option<String>,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: SavedSearchesCommand,
    output_format: &str,
) -> Result<()> {
    match command {
        SavedSearchesCommand::List { count } => run_list(config, count, output_format).await,
        SavedSearchesCommand::Info { name } => run_info(config, &name, output_format).await,
        SavedSearchesCommand::Run {
            name,
            wait,
            earliest,
            latest,
        } => {
            run_run(
                config,
                &name,
                wait,
                earliest.as_deref(),
                latest.as_deref(),
                output_format,
            )
            .await
        }
    }
}

async fn run_list(config: splunk_config::Config, count: usize, output_format: &str) -> Result<()> {
    info!("Listing saved searches (count: {})", count);

    let auth_strategy = convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    let mut searches = client.list_saved_searches().await?;

    // Truncate to requested count if needed
    if searches.len() > count {
        searches.truncate(count);
    }

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_saved_searches(&searches)?;
    print!("{}", output);

    Ok(())
}

async fn run_info(config: splunk_config::Config, name: &str, output_format: &str) -> Result<()> {
    info!("Getting saved search info for: {}", name);

    let auth_strategy = convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    let searches = client.list_saved_searches().await?;

    let search = searches
        .iter()
        .find(|s| s.name == name)
        .with_context(|| format!("Saved search '{}' not found", name))?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_saved_search_info(search)?;
    print!("{}", output);

    Ok(())
}

async fn run_run(
    config: splunk_config::Config,
    name: &str,
    wait: bool,
    earliest: Option<&str>,
    latest: Option<&str>,
    output_format: &str,
) -> Result<()> {
    info!("Running saved search: {}", name);

    let auth_strategy = convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    let searches = client.list_saved_searches().await?;

    let search = searches
        .iter()
        .find(|s| s.name == name)
        .with_context(|| format!("Saved search '{}' not found", name))?;

    info!("Executing search query: {}", search.search);
    let results = client
        .search(&search.search, wait, earliest, latest, Some(100))
        .await?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_search_results(&results)?;
    print!("{}", output);

    Ok(())
}
