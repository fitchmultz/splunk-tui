//! Data models command implementation.
//!
//! Responsibilities:
//! - List data models with optional count limiting and pagination
//! - Show detailed information about specific data models
//! - Support detailed view with descriptions
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Data model creation or editing (use Splunk web UI or API directly)
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Count and offset parameters are validated for safe pagination
//! - Data model names are passed through without modification

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, Pagination, TableFormatter, get_formatter, write_to_file};
use splunk_config::constants::*;

#[derive(Subcommand)]
pub enum DatamodelsCommand {
    /// List all data models (default)
    List {
        /// Show detailed information including description
        #[arg(short, long)]
        detailed: bool,
        /// Maximum number of data models to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,
        /// Offset into the data model list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
    },
    /// View a specific data model by name
    View {
        /// Data model name (required)
        name: String,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: DatamodelsCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        DatamodelsCommand::List {
            detailed,
            count,
            offset,
        } => {
            run_list(
                config,
                detailed,
                count,
                offset,
                output_format,
                output_file,
                cancel,
            )
            .await
        }
        DatamodelsCommand::View { name } => {
            run_view(config, &name, output_format, output_file, cancel).await
        }
    }
}

async fn run_list(
    config: splunk_config::Config,
    detailed: bool,
    count: usize,
    offset: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing data models (count: {}, offset: {})", count, offset);

    let client = crate::commands::build_client_from_config(&config)?;

    let offset_param = if offset == 0 { None } else { Some(offset) };

    let datamodels = tokio::select! {
        res = client.list_datamodels(Some(count), offset_param) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;

    if format == OutputFormat::Table {
        let formatter = TableFormatter;
        let pagination = Pagination {
            offset,
            page_size: count,
            total: None,
        };
        let output = formatter.format_datamodels_paginated(&datamodels, detailed, pagination)?;
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
        return Ok(());
    }

    let formatter = get_formatter(format);
    let output = formatter.format_datamodels(&datamodels, detailed)?;
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

async fn run_view(
    config: splunk_config::Config,
    name: &str,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Viewing data model: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    let datamodel = tokio::select! {
        res = client.get_datamodel(name) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_datamodel(&datamodel)?;

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
