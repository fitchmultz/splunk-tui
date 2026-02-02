//! Saved searches command implementation.

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
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
    /// Edit a saved search
    Edit {
        /// Name of the saved search to edit
        #[arg(value_name = "NAME")]
        name: String,
        /// New search query (SPL)
        #[arg(short, long)]
        search: Option<String>,
        /// New description
        #[arg(short, long)]
        description: Option<String>,
        /// Enable/disable the saved search
        #[arg(long)]
        disabled: Option<bool>,
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
        SavedSearchesCommand::Edit {
            name,
            search,
            description,
            disabled,
        } => {
            run_edit(
                config,
                &name,
                search.as_deref(),
                description.as_deref(),
                disabled,
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

    let mut client = crate::commands::build_client_from_config(&config)?;

    let searches = tokio::select! {
        res = client.list_saved_searches(Some(count as u64), None) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

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

    let mut client = crate::commands::build_client_from_config(&config)?;

    let search = tokio::select! {
        res = client.get_saved_search(name) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_saved_search_info(&search)?;
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

    let mut client = crate::commands::build_client_from_config(&config)?;

    let search = tokio::select! {
        res = client.get_saved_search(name) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

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

async fn run_edit(
    config: splunk_config::Config,
    name: &str,
    search: Option<&str>,
    description: Option<&str>,
    disabled: Option<bool>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Editing saved search: {}", name);

    // Validate at least one field is provided
    if search.is_none() && description.is_none() && disabled.is_none() {
        return Err(anyhow::anyhow!(
            "At least one field must be provided to update (--search, --description, or --disabled)"
        ));
    }

    let mut client = crate::commands::build_client_from_config(&config)?;

    tokio::select! {
        res = client.update_saved_search(name, search, description, disabled) => {
            res?;
            eprintln!("Saved search '{}' updated successfully", name);
        }
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    }

    Ok(())
}
