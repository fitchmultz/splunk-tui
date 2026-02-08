//! KV Store command implementation.
//!
//! Responsibilities:
//! - Get KV Store status and health information
//! - List collections with optional app/owner filtering and pagination
//! - Create new collections with optional field schemas
//! - Delete collections with confirmation
//! - Query collection data with optional filters
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Collection record modification (create/update/delete)
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Collection names are validated as non-empty
//! - App and owner contexts default to reasonable values (search/nobody)
//! - Delete operations require confirmation unless --force is used
//! - Query parameters are passed through without modification

use anyhow::Result;
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, Pagination, TableFormatter, get_formatter, output_result};
use splunk_config::constants::*;

#[derive(Subcommand)]
pub enum KvstoreCommand {
    /// Get KVStore status (default)
    Status,
    /// List all collections
    List {
        /// App context (default: all apps)
        #[arg(long)]
        app: Option<String>,
        /// Owner context (default: nobody)
        #[arg(long)]
        owner: Option<String>,
        /// Maximum number of collections to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,
        /// Offset into the collection list
        #[arg(long, default_value = "0")]
        offset: usize,
    },
    /// Create a new collection
    Create {
        /// Collection name
        name: String,
        /// App context
        #[arg(long, default_value = "search")]
        app: String,
        /// Owner context
        #[arg(long, default_value = "nobody")]
        owner: String,
        /// Field schema as JSON string
        #[arg(long)]
        fields: Option<String>,
        /// Accelerated fields as JSON string
        #[arg(long)]
        accelerated_fields: Option<String>,
    },
    /// Delete a collection
    Delete {
        /// Collection name
        name: String,
        /// App context
        #[arg(long, default_value = "search")]
        app: String,
        /// Owner context
        #[arg(long, default_value = "nobody")]
        owner: String,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
    /// Query collection data
    Data {
        /// Collection name
        name: String,
        /// App context
        #[arg(long, default_value = "search")]
        app: String,
        /// Owner context
        #[arg(long, default_value = "nobody")]
        owner: String,
        /// MongoDB-style query (JSON)
        #[arg(short, long)]
        query: Option<String>,
        /// Maximum number of records
        #[arg(short, long, default_value_t = DEFAULT_MAX_RESULTS)]
        count: usize,
        /// Offset into results
        #[arg(long, default_value = "0")]
        offset: usize,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: KvstoreCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        KvstoreCommand::Status => run_status(config, output_format, output_file, cancel).await,
        KvstoreCommand::List {
            app,
            owner,
            count,
            offset,
        } => {
            run_list(
                config,
                app,
                owner,
                count,
                offset,
                output_format,
                output_file,
                cancel,
            )
            .await
        }
        KvstoreCommand::Create {
            name,
            app,
            owner,
            fields,
            accelerated_fields,
        } => run_create(config, name, app, owner, fields, accelerated_fields, cancel).await,
        KvstoreCommand::Delete {
            name,
            app,
            owner,
            force,
        } => run_delete(config, name, app, owner, force, cancel).await,
        KvstoreCommand::Data {
            name,
            app,
            owner,
            query,
            count,
            offset,
        } => {
            run_data(
                config,
                name,
                app,
                owner,
                query,
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

async fn run_status(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Fetching KVStore status...");

    let client = crate::commands::build_client_from_config(&config)?;

    info!("Connecting to {}", client.base_url());

    let kvstore_status = tokio::select! {
        res = client.get_kvstore_status() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    // Format and print results
    let output = formatter.format_kvstore_status(&kvstore_status)?;
    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}

#[allow(clippy::too_many_arguments)]
async fn run_list(
    config: splunk_config::Config,
    app: Option<String>,
    owner: Option<String>,
    count: usize,
    offset: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing KVStore collections...");

    let client = crate::commands::build_client_from_config(&config)?;

    let offset_param = if offset == 0 { None } else { Some(offset) };

    let collections = tokio::select! {
        res = client.list_collections(app.as_deref(), owner.as_deref(), Some(count), offset_param) => res?,
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
        let output = formatter.format_kvstore_collections_paginated(&collections, pagination)?;
        output_result(&output, format, output_file.as_ref())?;
        return Ok(());
    }

    let formatter = get_formatter(format);
    let output = formatter.format_kvstore_collections(&collections)?;
    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}

async fn run_create(
    config: splunk_config::Config,
    name: String,
    app: String,
    owner: String,
    fields: Option<String>,
    accelerated_fields: Option<String>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Creating KVStore collection: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    let params = splunk_client::models::CreateCollectionParams {
        name: name.clone(),
        app: Some(app.clone()),
        owner: Some(owner.clone()),
        fields,
        accelerated_fields,
    };

    tokio::select! {
        res = client.create_collection(&params) => {
            let collection = res?;
            println!("Collection '{}' created successfully in app '{}'.", collection.name, app);
            Ok(())
        }
        _ = cancel.cancelled() => Err(Cancelled.into()),
    }
}

async fn run_delete(
    config: splunk_config::Config,
    name: String,
    app: String,
    owner: String,
    force: bool,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    if !force && !crate::interactive::confirm_delete(&name, "collection")? {
        return Ok(());
    }

    info!("Deleting KVStore collection: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    tokio::select! {
        res = client.delete_collection(&name, &app, &owner) => {
            res?;
            println!("Collection '{}' deleted successfully.", name);
            Ok(())
        }
        _ = cancel.cancelled() => Err(Cancelled.into()),
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_data(
    config: splunk_config::Config,
    name: String,
    app: String,
    owner: String,
    query: Option<String>,
    count: usize,
    offset: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Querying KVStore collection data: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    let offset_param = if offset == 0 { None } else { Some(offset) };

    let records = tokio::select! {
        res = client.list_collection_records(
            &name, &app, &owner, query.as_deref(), Some(count), offset_param
        ) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    // Format and print results
    let output = formatter.format_kvstore_records(&records)?;
    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}
