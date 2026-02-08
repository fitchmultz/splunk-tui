//! Configuration files command implementation.
//!
//! Responsibilities:
//! - List available configuration files (e.g., props, transforms, indexes)
//! - List configuration stanzas for a specific config file
//! - View detailed configuration for specific stanzas
//! - Support pagination for large config file listings
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Configuration modification (read-only operations)
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Config file names and stanza names are passed through without modification
//! - Count and offset parameters are validated for safe pagination

use anyhow::Result;
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{
    Formatter, OutputFormat, Pagination, TableFormatter, get_formatter, output_result,
};
use splunk_config::constants::*;

/// Configs subcommands.
#[derive(Subcommand)]
pub enum ConfigsCommand {
    /// List configuration files or stanzas
    List {
        /// Specific config file to list stanzas from (e.g., "props", "transforms")
        #[arg(short, long)]
        config_file: Option<String>,

        /// Maximum number of items to return
        #[arg(long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,

        /// Offset for pagination
        #[arg(long, default_value = "0")]
        offset: usize,
    },

    /// View a specific configuration stanza
    View {
        /// Configuration file name (e.g., "props", "transforms")
        config_file: String,

        /// Stanza name to view
        stanza_name: String,
    },
}

/// Run the configs command.
///
/// Handles configuration file operations.
///
/// # Arguments
///
/// * `config` - The loaded Splunk configuration
/// * `command` - The configs subcommand to run
/// * `output_format` - Output format (table, json, csv, xml)
/// * `output_file` - Optional file path to write output to
/// * `cancel` - Cancellation token for graceful shutdown
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the operation fails.
pub async fn run(
    config: splunk_config::Config,
    command: ConfigsCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        ConfigsCommand::List {
            config_file,
            count,
            offset,
        } => {
            run_list(
                config,
                config_file,
                count,
                offset,
                output_format,
                output_file,
                cancel,
            )
            .await
        }
        ConfigsCommand::View {
            config_file,
            stanza_name,
        } => {
            run_view(
                config,
                config_file,
                stanza_name,
                output_format,
                output_file,
                cancel,
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_list(
    config: splunk_config::Config,
    config_file: Option<String>,
    count: usize,
    offset: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    let client = crate::commands::build_client_from_config(&config)?;

    // Avoid sending offset=0 unless user explicitly paginates
    let offset_param = if offset == 0 { None } else { Some(offset) };

    if let Some(config_file) = config_file {
        info!(
            "Listing config stanzas for '{}' (count: {}, offset: {})",
            config_file, count, offset
        );

        let stanzas = tokio::select! {
            res = client.list_config_stanzas(&config_file, Some(count), offset_param) => res?,
            _ = cancel.cancelled() => return Err(Cancelled.into()),
        };

        // Parse output format
        let format = OutputFormat::from_str(output_format)?;

        // Table output gets pagination footer; machine-readable formats must not.
        if format == OutputFormat::Table {
            let formatter = TableFormatter;
            let pagination = Pagination {
                offset,
                page_size: count,
                total: None,
            };
            let output = formatter.format_config_stanzas_paginated(&stanzas, pagination)?;
            output_result(&output, format, output_file.as_ref())?;
            return Ok(());
        }

        let formatter = get_formatter(format);
        let output = formatter.format_config_stanzas(&stanzas)?;
        output_result(&output, format, output_file.as_ref())?;
    } else {
        info!("Listing available config files");

        let config_files = tokio::select! {
            res = client.list_config_files() => res?,
            _ = cancel.cancelled() => return Err(Cancelled.into()),
        };

        // Parse output format
        let format = OutputFormat::from_str(output_format)?;

        if format == OutputFormat::Table {
            let formatter = TableFormatter;
            let output = formatter.format_config_files(&config_files)?;
            output_result(&output, format, output_file.as_ref())?;
            return Ok(());
        }

        let formatter = get_formatter(format);
        let output = formatter.format_config_files(&config_files)?;
        output_result(&output, format, output_file.as_ref())?;
    }

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_view(
    config: splunk_config::Config,
    config_file: String,
    stanza_name: String,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!(
        "Viewing config stanza '{}' from '{}'",
        stanza_name, config_file
    );

    let client = crate::commands::build_client_from_config(&config)?;

    let stanza = tokio::select! {
        res = client.get_config_stanza(&config_file, &stanza_name) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;

    let formatter = get_formatter(format);
    let output = formatter.format_config_stanza(&stanza)?;
    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}
