//! Data inputs command implementation.
//!
//! Responsibilities:
//! - List data inputs with optional type filtering (tcp/raw, tcp/cooked, udp, monitor, script)
//! - Support pagination via offset parameter
//! - Show detailed input information when requested
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Input configuration or creation
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Count and offset parameters are validated for safe pagination
//! - Input type filters are passed through without modification

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, Pagination, TableFormatter, get_formatter, write_to_file};
use splunk_config::constants::*;

/// Inputs subcommands.
#[derive(Subcommand)]
pub enum InputsCommand {
    /// List data inputs
    List {
        /// Show detailed information about each input
        #[arg(short, long)]
        detailed: bool,

        /// Filter by input type (tcp/raw, tcp/cooked, udp, monitor, script)
        #[arg(short, long)]
        input_type: Option<String>,

        /// Maximum number of inputs to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,

        /// Offset into the input list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
    },
}

/// Run the inputs command.
///
/// Lists data inputs from the Splunk server.
///
/// # Arguments
///
/// * `config` - The loaded Splunk configuration
/// * `command` - The inputs subcommand to run
/// * `output_format` - Output format (table, json, csv, xml)
/// * `output_file` - Optional file path to write output to
/// * `cancel` - Cancellation token for graceful shutdown
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the operation fails.
pub async fn run(
    config: splunk_config::Config,
    command: InputsCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        InputsCommand::List {
            detailed,
            input_type,
            count,
            offset,
        } => {
            run_list(
                config,
                detailed,
                input_type,
                count,
                offset,
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
    detailed: bool,
    input_type: Option<String>,
    count: usize,
    offset: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing inputs (count: {}, offset: {})", count, offset);

    let client = crate::commands::build_client_from_config(&config)?;

    // Avoid sending offset=0 unless user explicitly paginates; both are functionally OK.
    let offset_param = if offset == 0 { None } else { Some(offset) };

    let inputs = if let Some(input_type) = input_type {
        tokio::select! {
            res = client.list_inputs_by_type(&input_type, Some(count), offset_param) => res?,
            _ = cancel.cancelled() => return Err(Cancelled.into()),
        }
    } else {
        tokio::select! {
            res = client.list_inputs(Some(count), offset_param) => res?,
            _ = cancel.cancelled() => return Err(Cancelled.into()),
        }
    };

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;

    // Table output gets pagination footer; machine-readable formats must not.
    if format == OutputFormat::Table {
        let formatter = TableFormatter;
        let pagination = Pagination {
            offset,
            page_size: count,
            total: None, // server-side total is not available in current client response shape
        };
        let output = formatter.format_inputs_paginated(&inputs, detailed, pagination)?;
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
    let output = formatter.format_inputs(&inputs, detailed)?;
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
