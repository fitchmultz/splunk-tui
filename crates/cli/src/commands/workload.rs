//! Workload management command implementation.
//!
//! This module provides the CLI command for listing Splunk workload pools and rules.
//!
//! # What this module handles:
//! - Listing workload pools and rules with pagination support
//! - Multiple output formats (table, JSON, CSV, XML)
//! - Cancellation support
//!
//! # What this module does NOT handle:
//! - Direct HTTP API calls (delegated to client library)
//! - Output formatting (delegated to formatters)

use anyhow::{Context, Result};
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{
    Formatter, OutputFormat, Pagination, TableFormatter, get_formatter, write_to_file,
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

    let mut client = crate::commands::build_client_from_config(&config)?;

    let count_u64 =
        u64::try_from(count).context("Invalid --count (value too large for this platform)")?;
    let offset_u64 =
        u64::try_from(offset).context("Invalid --offset (value too large for this platform)")?;

    // Avoid sending offset=0 unless user explicitly paginates; both are functionally OK.
    let offset_param = if offset == 0 { None } else { Some(offset_u64) };

    // Fetch both pools and rules concurrently
    let (pools, rules) = tokio::select! {
        res = async {
            let pools = client.list_workload_pools(Some(count_u64), offset_param).await?;
            let rules = client.list_workload_rules(Some(count_u64), offset_param).await?;
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
    let mut output = formatter.format_workload_pools(&pools, detailed)?;
    output.push('\n');
    output.push_str(&formatter.format_workload_rules(&rules, detailed)?);

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
