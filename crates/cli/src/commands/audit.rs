//! Audit events command implementation.
//!
//! Responsibilities:
//! - List audit events with optional filtering and count limiting
//! - Show detailed information about audit events
//! - Support pagination via offset parameter
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Audit configuration or policy management
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Count and offset parameters are validated for safe pagination
//! - Time-based filtering uses Splunk's standard time format

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, Pagination, TableFormatter, get_formatter, write_to_file};

#[derive(Subcommand)]
pub enum AuditCommand {
    /// List audit events (default)
    List {
        /// Show detailed information about each event
        #[arg(short, long)]
        detailed: bool,
        /// Maximum number of events to list
        #[arg(short, long, default_value = "50")]
        count: usize,
        /// Offset into the event list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Earliest time for events (e.g., "-24h", "2024-01-01T00:00:00")
        #[arg(short, long, default_value = "-24h")]
        earliest: String,
        /// Latest time for events (e.g., "now", "2024-01-02T00:00:00")
        #[arg(short, long, default_value = "now")]
        latest: String,
        /// Filter by user
        #[arg(long)]
        user: Option<String>,
        /// Filter by action
        #[arg(long)]
        action: Option<String>,
    },
    /// Get recent audit events (last 24 hours)
    Recent {
        /// Maximum number of events to list
        #[arg(short, long, default_value = "20")]
        count: usize,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: AuditCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        AuditCommand::List {
            detailed,
            count,
            offset,
            earliest,
            latest,
            user,
            action,
        } => {
            run_list(
                config,
                detailed,
                count,
                offset,
                earliest,
                latest,
                user,
                action,
                output_format,
                output_file,
                cancel,
            )
            .await
        }
        AuditCommand::Recent { count } => {
            run_recent(config, count, output_format, output_file, cancel).await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_list(
    config: splunk_config::Config,
    detailed: bool,
    count: usize,
    offset: usize,
    earliest: String,
    latest: String,
    user: Option<String>,
    action: Option<String>,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!(
        "Listing audit events (count: {}, offset: {}, earliest: {}, latest: {})",
        count, offset, earliest, latest
    );

    let mut client = crate::commands::build_client_from_config(&config)?;

    let params = splunk_client::models::ListAuditEventsParams {
        earliest: Some(earliest),
        latest: Some(latest),
        count: Some(count as u64),
        offset: Some(offset as u64),
        user,
        action,
    };

    let events = tokio::select! {
        res = client.list_audit_events(&params) => res?,
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
        let output = formatter.format_audit_events_paginated(&events, detailed, pagination)?;
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
    let output = formatter.format_audit_events(&events, detailed)?;
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

async fn run_recent(
    config: splunk_config::Config,
    count: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Getting recent audit events (count: {})", count);

    let mut client = crate::commands::build_client_from_config(&config)?;

    let events = tokio::select! {
        res = client.get_recent_audit_events(count as u64) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_audit_events(&events, false)?;

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
