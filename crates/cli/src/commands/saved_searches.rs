//! Saved searches command implementation.

use anyhow::{Context, Result};
use clap::Subcommand;
use splunk_client::SplunkClient;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::commands::convert_auth_strategy;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

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
        #[arg(short, long, allow_hyphen_values = true)]
        earliest: Option<String>,
        /// Latest time for the search (e.g., 'now', '2024-01-02T00:00:00')
        #[arg(short, long, allow_hyphen_values = true)]
        latest: Option<String>,
        /// Maximum number of results to return
        #[arg(short, long, default_value = "100")]
        count: usize,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: SavedSearchesCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        SavedSearchesCommand::List { count } => {
            run_list(config, count, output_format, output_file.clone(), cancel).await
        }
        SavedSearchesCommand::Info { name } => {
            run_info(config, &name, output_format, output_file.clone(), cancel).await
        }
        SavedSearchesCommand::Run {
            name,
            wait,
            earliest,
            latest,
            count,
        } => {
            run_run(
                config,
                &name,
                wait,
                earliest.as_deref(),
                latest.as_deref(),
                count,
                output_format,
                output_file.clone(),
                cancel,
            )
            .await
        }
    }
}

async fn run_list(
    config: splunk_config::Config,
    count: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing saved searches (count: {})", count);

    let auth_strategy = convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .session_ttl_seconds(config.connection.session_ttl_seconds)
        .session_expiry_buffer_seconds(config.connection.session_expiry_buffer_seconds)
        .build()?;

    let mut searches = tokio::select! {
        res = client.list_saved_searches() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    // Truncate to requested count if needed
    if searches.len() > count {
        searches.truncate(count);
    }

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_saved_searches(&searches)?;
    if let Some(ref path) = output_file {
        write_to_file(&output, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        print!("{}", output);
    }

    Ok(())
}

async fn run_info(
    config: splunk_config::Config,
    name: &str,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Getting saved search info for: {}", name);

    let auth_strategy = convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .session_ttl_seconds(config.connection.session_ttl_seconds)
        .session_expiry_buffer_seconds(config.connection.session_expiry_buffer_seconds)
        .build()?;

    let searches = tokio::select! {
        res = client.list_saved_searches() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let search = searches
        .iter()
        .find(|s| s.name == name)
        .with_context(|| format!("Saved search '{}' not found", name))?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_saved_search_info(search)?;
    if let Some(ref path) = output_file {
        write_to_file(&output, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        print!("{}", output);
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_run(
    config: splunk_config::Config,
    name: &str,
    wait: bool,
    earliest: Option<&str>,
    latest: Option<&str>,
    max_results: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Running saved search: {}", name);

    let auth_strategy = convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .session_ttl_seconds(config.connection.session_ttl_seconds)
        .session_expiry_buffer_seconds(config.connection.session_expiry_buffer_seconds)
        .build()?;

    let searches = tokio::select! {
        res = client.list_saved_searches() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let search = searches
        .iter()
        .find(|s| s.name == name)
        .with_context(|| format!("Saved search '{}' not found", name))?;

    info!("Executing search query: {}", search.search);
    let results = tokio::select! {
        res = client.search(&search.search, wait, earliest, latest, Some(max_results as u64)) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_search_results(&results)?;
    if let Some(ref path) = output_file {
        write_to_file(&output, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        print!("{}", output);
    }

    Ok(())
}
