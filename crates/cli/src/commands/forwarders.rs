//! Forwarders (deployment clients) command implementation.
//!
//! Responsibilities:
//! - List deployment clients (forwarders) with optional count limiting
//! - Support pagination via offset parameter
//! - Show detailed forwarder information when requested
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Forwarder configuration or management
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Count and offset parameters are validated for safe pagination
//! - Server-side total may not be available for all deployments

use anyhow::Result;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, Pagination, TableFormatter, get_formatter, output_result};

/// Run the forwarders command.
///
/// Lists deployment clients (forwarders) from the Splunk deployment server.
///
/// # Arguments
///
/// * `config` - The loaded Splunk configuration
/// * `detailed` - Whether to show detailed information
/// * `count` - Maximum number of results to return
/// * `offset` - Offset for pagination
/// * `output_format` - Output format (table, json, csv, xml)
/// * `output_file` - Optional file path to write output to
/// * `cancel` - Cancellation token for graceful shutdown
///
/// # Returns
///
/// Returns `Ok(())` on success, or an error if the operation fails.
pub async fn run(
    config: splunk_config::Config,
    detailed: bool,
    count: usize,
    offset: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing forwarders (count: {}, offset: {})", count, offset);

    let client = crate::commands::build_client_from_config(&config)?;

    // Avoid sending offset=0 unless user explicitly paginates; both are functionally OK.
    let offset_param = if offset == 0 { None } else { Some(offset) };

    let forwarders = tokio::select! {
        res = client.list_forwarders(Some(count), offset_param) => res?,
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
            total: None, // server-side total is not available in current client response shape
        };
        let output = formatter.format_forwarders_paginated(&forwarders, detailed, pagination)?;
        output_result(&output, format, output_file.as_ref())?;
        return Ok(());
    }

    let formatter = get_formatter(format);
    let output = formatter.format_forwarders(&forwarders, detailed)?;
    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}
