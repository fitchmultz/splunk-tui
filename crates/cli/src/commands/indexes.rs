//! Indexes command implementation.
//!
//! Responsibilities:
//! - List indexes with optional count limiting and pagination
//! - Create new indexes with configurable parameters
//! - Modify existing index properties
//! - Delete indexes with confirmation
//! - Show detailed index information when requested
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Index data ingestion or searching
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Count and offset parameters are validated for safe pagination
//! - Delete operations require confirmation unless --force is used
//! - Index names are passed through without modification
//! - Server-side total may not be available for all index listings

use anyhow::Result;
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, Pagination, TableFormatter, get_formatter, output_result};
use splunk_config::constants::*;

#[derive(Subcommand)]
pub enum IndexesCommand {
    /// List all indexes (default)
    List {
        /// Show detailed information about each index
        #[arg(short, long)]
        detailed: bool,
        /// Maximum number of indexes to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,
        /// Offset into the index list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
    },
    /// Create a new index
    Create {
        /// Index name (required)
        name: String,
        /// Maximum data size in MB
        #[arg(long)]
        max_data_size_mb: Option<usize>,
        /// Maximum hot buckets
        #[arg(long)]
        max_hot_buckets: Option<usize>,
        /// Maximum warm DB count
        #[arg(long)]
        max_warm_db_count: Option<usize>,
        /// Frozen time period in seconds
        #[arg(long)]
        frozen_time_period_secs: Option<usize>,
        /// Home path
        #[arg(long)]
        home_path: Option<String>,
        /// Cold DB path
        #[arg(long)]
        cold_db_path: Option<String>,
        /// Thawed path
        #[arg(long)]
        thawed_path: Option<String>,
        /// Cold to frozen directory
        #[arg(long)]
        cold_to_frozen_dir: Option<String>,
    },
    /// Modify an existing index
    Modify {
        /// Index name (required)
        name: String,
        /// Maximum data size in MB
        #[arg(long)]
        max_data_size_mb: Option<usize>,
        /// Maximum hot buckets
        #[arg(long)]
        max_hot_buckets: Option<usize>,
        /// Maximum warm DB count
        #[arg(long)]
        max_warm_db_count: Option<usize>,
        /// Frozen time period in seconds
        #[arg(long)]
        frozen_time_period_secs: Option<usize>,
        /// Home path
        #[arg(long)]
        home_path: Option<String>,
        /// Cold DB path
        #[arg(long)]
        cold_db_path: Option<String>,
        /// Thawed path
        #[arg(long)]
        thawed_path: Option<String>,
        /// Cold to frozen directory
        #[arg(long)]
        cold_to_frozen_dir: Option<String>,
    },
    /// Delete an index
    Delete {
        /// Index name (required)
        name: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: IndexesCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        IndexesCommand::List {
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
        IndexesCommand::Create {
            name,
            max_data_size_mb,
            max_hot_buckets,
            max_warm_db_count,
            frozen_time_period_secs,
            home_path,
            cold_db_path,
            thawed_path,
            cold_to_frozen_dir,
        } => {
            run_create(
                config,
                &name,
                max_data_size_mb,
                max_hot_buckets,
                max_warm_db_count,
                frozen_time_period_secs,
                home_path,
                cold_db_path,
                thawed_path,
                cold_to_frozen_dir,
                cancel,
            )
            .await
        }
        IndexesCommand::Modify {
            name,
            max_data_size_mb,
            max_hot_buckets,
            max_warm_db_count,
            frozen_time_period_secs,
            home_path,
            cold_db_path,
            thawed_path,
            cold_to_frozen_dir,
        } => {
            run_modify(
                config,
                &name,
                max_data_size_mb,
                max_hot_buckets,
                max_warm_db_count,
                frozen_time_period_secs,
                home_path,
                cold_db_path,
                thawed_path,
                cold_to_frozen_dir,
                cancel,
            )
            .await
        }
        IndexesCommand::Delete { name, force } => run_delete(config, &name, force, cancel).await,
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
    info!("Listing indexes (count: {}, offset: {})", count, offset);

    let client = crate::commands::build_client_from_config(&config)?;

    // Avoid sending offset=0 unless user explicitly paginates; both are functionally OK.
    let offset_param = if offset == 0 { None } else { Some(offset) };

    let indexes = tokio::select! {
        res = client.list_indexes(Some(count), offset_param) => res?,
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
        let output = formatter.format_indexes_paginated(&indexes, detailed, pagination)?;
        output_result(&output, format, output_file.as_ref())?;
        return Ok(());
    }

    let formatter = get_formatter(format);
    let output = formatter.format_indexes(&indexes, detailed)?;
    output_result(&output, format, output_file.as_ref())?;
    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_create(
    config: splunk_config::Config,
    name: &str,
    max_data_size_mb: Option<usize>,
    max_hot_buckets: Option<usize>,
    max_warm_db_count: Option<usize>,
    frozen_time_period_secs: Option<usize>,
    home_path: Option<String>,
    cold_db_path: Option<String>,
    thawed_path: Option<String>,
    cold_to_frozen_dir: Option<String>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Creating index: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    let params = splunk_client::CreateIndexParams {
        name: name.to_string(),
        max_data_size_mb,
        max_hot_buckets,
        max_warm_db_count,
        frozen_time_period_in_secs: frozen_time_period_secs,
        home_path,
        cold_db_path,
        thawed_path,
        cold_to_frozen_dir,
    };

    tokio::select! {
        res = client.create_index(&params) => {
            let index = res?;
            println!("Index '{}' created successfully.", index.name);
            Ok(())
        }
        _ = cancel.cancelled() => Err(Cancelled.into()),
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_modify(
    config: splunk_config::Config,
    name: &str,
    max_data_size_mb: Option<usize>,
    max_hot_buckets: Option<usize>,
    max_warm_db_count: Option<usize>,
    frozen_time_period_secs: Option<usize>,
    home_path: Option<String>,
    cold_db_path: Option<String>,
    thawed_path: Option<String>,
    cold_to_frozen_dir: Option<String>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Modifying index: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    let params = splunk_client::ModifyIndexParams {
        max_data_size_mb,
        max_hot_buckets,
        max_warm_db_count,
        frozen_time_period_in_secs: frozen_time_period_secs,
        home_path,
        cold_db_path,
        thawed_path,
        cold_to_frozen_dir,
    };

    tokio::select! {
        res = client.modify_index(name, &params) => {
            let index = res?;
            println!("Index '{}' modified successfully.", index.name);
            Ok(())
        }
        _ = cancel.cancelled() => Err(Cancelled.into()),
    }
}

async fn run_delete(
    config: splunk_config::Config,
    name: &str,
    force: bool,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    if !force && !crate::interactive::confirm_delete(name, "index")? {
        return Ok(());
    }

    info!("Deleting index: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    tokio::select! {
        res = client.delete_index(name) => {
            res?;
            println!("Index '{}' deleted successfully.", name);
            Ok(())
        }
        _ = cancel.cancelled() => Err(Cancelled.into()),
    }
}
