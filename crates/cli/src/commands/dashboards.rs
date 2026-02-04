//! Dashboards command implementation.

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, Pagination, TableFormatter, get_formatter, write_to_file};

#[derive(Subcommand)]
pub enum DashboardsCommand {
    /// List all dashboards (default)
    List {
        /// Show detailed information including description
        #[arg(short, long)]
        detailed: bool,
        /// Maximum number of dashboards to list
        #[arg(short, long, default_value = "30")]
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

    let mut client = crate::commands::build_client_from_config(&config)?;

    let count_u64 =
        u64::try_from(count).context("Invalid --count (value too large for this platform)")?;
    let offset_u64 =
        u64::try_from(offset).context("Invalid --offset (value too large for this platform)")?;

    let offset_param = if offset == 0 { None } else { Some(offset_u64) };

    let dashboards = tokio::select! {
        res = client.list_dashboards(Some(count_u64), offset_param) => res?,
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
        let output = formatter.format_dashboards_paginated(&dashboards, detailed, pagination)?;
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
    let output = formatter.format_dashboards(&dashboards, detailed)?;
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
    info!("Viewing dashboard: {}", name);

    let mut client = crate::commands::build_client_from_config(&config)?;

    let dashboard = tokio::select! {
        res = client.get_dashboard(name) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_dashboard(&dashboard)?;

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
