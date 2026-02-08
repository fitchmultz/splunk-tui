//! Dashboards command implementation.
//!
//! Responsibilities:
//! - List dashboards with optional count limiting and pagination
//! - Show detailed information about specific dashboards
//! - Support detailed view with descriptions
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Dashboard creation or editing (use Splunk web UI or API directly)
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Count and offset parameters are validated for safe pagination
//! - Dashboard names are passed through without modification

use anyhow::Result;
use clap::Subcommand;
use tracing::info;

use crate::formatters::{OutputFormat, Pagination, TableFormatter, get_formatter, output_result};
use splunk_config::constants::*;

#[derive(Subcommand)]
pub enum DashboardsCommand {
    /// List all dashboards (default)
    List {
        /// Show detailed information including description
        #[arg(short, long)]
        detailed: bool,
        /// Maximum number of dashboards to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,
        /// Offset into the dashboard list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
    },
    /// View a specific dashboard by name
    View {
        /// Dashboard name (required)
        name: String,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: DashboardsCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        DashboardsCommand::List {
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
        DashboardsCommand::View { name } => {
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
    info!("Listing dashboards (count: {}, offset: {})", count, offset);

    let client = crate::commands::build_client_from_config(&config)?;

    let offset_param = if offset == 0 { None } else { Some(offset) };

    let dashboards = cancellable!(client.list_dashboards(Some(count), offset_param), cancel)?;

    let format = OutputFormat::from_str(output_format)?;

    if format == OutputFormat::Table {
        let formatter = TableFormatter;
        let pagination = Pagination {
            offset,
            page_size: count,
            total: None,
        };
        let output = formatter.format_dashboards_paginated(&dashboards, detailed, pagination)?;
        output_result(&output, format, output_file.as_ref())?;
        return Ok(());
    }

    let formatter = get_formatter(format);
    let output = formatter.format_dashboards(&dashboards, detailed)?;
    output_result(&output, format, output_file.as_ref())?;
    Ok(())
}

async fn run_view(
    config: splunk_config::Config,
    name: &str,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Viewing dashboard: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    let dashboard = cancellable!(client.get_dashboard(name), cancel)?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_dashboard(&dashboard)?;

    output_result(&output, format, output_file.as_ref())?;
    Ok(())
}
