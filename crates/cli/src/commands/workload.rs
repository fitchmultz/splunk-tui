//! Workload management command implementation.
//!
//! Responsibilities:
//! - List workload pools with optional count limiting and pagination
//! - List workload rules with optional count limiting
//! - Show detailed information when requested
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Workload pool or rule configuration
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Count and offset parameters are validated for safe pagination
//! - Pools and rules are fetched concurrently for efficiency
//! - Server-side total may not be available for all listings

use anyhow::Result;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{
    Formatter, OutputFormat, Pagination, TableFormatter, get_formatter, output_result,
};

/// Run the workload command.
///
/// Lists workload pools and rules from the Splunk server.
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
    info!(
        "Listing workload pools and rules (count: {}, offset: {})",
        count, offset
    );

    let client = crate::commands::build_client_from_config(&config)?;

    // Avoid sending offset=0 unless user explicitly paginates; both are functionally OK.
    let offset_param = if offset == 0 { None } else { Some(offset) };

    // Fetch both pools and rules concurrently
    let (pools, rules) = tokio::select! {
        res = async {
            let pools = client.list_workload_pools(Some(count), offset_param).await?;
            let rules = client.list_workload_rules(Some(count), offset_param).await?;
            Ok::<_, anyhow::Error>((pools, rules))
        } => res?,
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
        let output = formatter.format_workload_pools_paginated(&pools, detailed, pagination)?;
        let output = format!(
            "{}\n\n{}",
            output,
            formatter.format_workload_rules(&rules, detailed)?
        );
        output_result(&output, format, output_file.as_ref())?;
        return Ok(());
    }

    let formatter = get_formatter(format);
    let mut output = formatter.format_workload_pools(&pools, detailed)?;
    output.push('\n');
    output.push_str(&formatter.format_workload_rules(&rules, detailed)?);

    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}
