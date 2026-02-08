//! Saved searches command implementation.
//!
//! Responsibilities:
//! - List saved searches with optional count limiting
//! - Show detailed information about specific saved searches
//! - Execute saved searches with optional time bounds
//! - Edit saved search properties (search query, description, disabled status)
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Saved search scheduling or alerting configuration
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Saved search names are validated as non-empty
//! - At least one field must be provided for edit operations
//! - Execution uses saved search's stored SPL query
//! - Time bounds default to -24h/now if not specified

use anyhow::{Context, Result};
use clap::Subcommand;
use splunk_client::SearchRequest;
use splunk_config::constants::*;
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter, output_result};

#[derive(Subcommand)]
pub enum SavedSearchesCommand {
    /// List saved searches
    List {
        /// Maximum number of saved searches to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
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
        #[arg(short, long, default_value_t = DEFAULT_MAX_RESULTS)]
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
    /// Create a new saved search
    Create {
        /// Name of the saved search to create
        #[arg(value_name = "NAME")]
        name: String,
        /// Search query (SPL) - required
        #[arg(short, long, required = true)]
        search: String,
        /// Description for the saved search
        #[arg(short, long)]
        description: Option<String>,
        /// Create the saved search in disabled state
        #[arg(long)]
        disabled: bool,
    },
    /// Delete a saved search by name
    Delete {
        /// Name of the saved search to delete
        #[arg(value_name = "NAME")]
        name: String,
        /// Force deletion without confirmation
        #[arg(short, long)]
        force: bool,
    },
    /// Enable a saved search
    Enable {
        /// Name of the saved search to enable
        #[arg(value_name = "NAME")]
        name: String,
    },
    /// Disable a saved search
    Disable {
        /// Name of the saved search to disable
        #[arg(value_name = "NAME")]
        name: String,
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
        SavedSearchesCommand::Create {
            name,
            search,
            description,
            disabled,
        } => {
            run_create(
                config,
                &name,
                &search,
                description.as_deref(),
                disabled,
                cancel,
            )
            .await
        }
        SavedSearchesCommand::Delete { name, force } => {
            run_delete(config, &name, force, cancel).await
        }
        SavedSearchesCommand::Enable { name } => {
            run_enable_disable(config, &name, false, cancel).await
        }
        SavedSearchesCommand::Disable { name } => {
            run_enable_disable(config, &name, true, cancel).await
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

    let client = crate::commands::build_client_from_config(&config)?;

    let searches = cancellable!(client.list_saved_searches(Some(count), None), cancel)?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_saved_searches(&searches)?;
    output_result(&output, format, output_file.as_ref())?;

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

    let client = crate::commands::build_client_from_config(&config)?;

    let search = cancellable!(client.get_saved_search(name), cancel)?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_saved_search_info(&search)?;
    output_result(&output, format, output_file.as_ref())?;

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

    let client = crate::commands::build_client_from_config(&config)?;

    let search = cancellable!(client.get_saved_search(name), cancel)?;

    info!("Executing search query: {}", search.search);
    let request = SearchRequest::new(&search.search, wait)
        .time_bounds(earliest.unwrap_or("-24h"), latest.unwrap_or("now"))
        .max_results(max_results);
    let results = cancellable!(client.search(request), cancel)?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_search_results(&results)?;
    output_result(&output, format, output_file.as_ref())?;

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

    let client = crate::commands::build_client_from_config(&config)?;

    cancellable_with!(
        client.update_saved_search(name, search, description, disabled),
        cancel,
        |_res| {
            eprintln!("Saved search '{}' updated successfully", name);
            Ok(())
        }
    )?;

    Ok(())
}

async fn run_create(
    config: splunk_config::Config,
    name: &str,
    search: &str,
    description: Option<&str>,
    disabled: bool,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Creating saved search: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    // First create the saved search
    cancellable!(client.create_saved_search(name, search), cancel)
        .with_context(|| format!("Failed to create saved search '{}'", name))?;

    // If description or disabled flag provided, update the saved search
    if description.is_some() || disabled {
        cancellable!(
            client.update_saved_search(name, None, description, Some(disabled)),
            cancel
        )
        .with_context(|| {
            format!(
                "Failed to update saved search '{}' with description/disabled",
                name
            )
        })?;
    }

    println!("Saved search '{}' created successfully.", name);
    Ok(())
}

async fn run_delete(
    config: splunk_config::Config,
    name: &str,
    force: bool,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Deleting saved search: {}", name);

    if !force && !crate::interactive::confirm_delete(name, "saved search")? {
        return Ok(());
    }

    let client = crate::commands::build_client_from_config(&config)?;

    cancellable!(client.delete_saved_search(name), cancel)
        .with_context(|| format!("Failed to delete saved search '{}'", name))?;

    println!("Saved search '{}' deleted successfully.", name);

    Ok(())
}

async fn run_enable_disable(
    config: splunk_config::Config,
    name: &str,
    disabled: bool,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    let action = if disabled { "Disabling" } else { "Enabling" };
    info!("{} saved search: {}", action, name);

    let client = crate::commands::build_client_from_config(&config)?;

    let status = if disabled { "disabled" } else { "enabled" };

    cancellable!(
        client.update_saved_search(name, None, None, Some(disabled)),
        cancel
    )
    .with_context(|| format!("Failed to {} saved search '{}'", status, name))?;

    println!("Saved search '{}' {} successfully.", name, status);

    Ok(())
}
