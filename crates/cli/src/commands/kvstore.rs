//! KVStore command implementation.

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, Pagination, TableFormatter, get_formatter, write_to_file};

#[derive(Subcommand)]
pub enum KvstoreCommand {
    /// Get KVStore status (default)
    Status,
    /// List all collections
    List {
        /// App context (default: all apps)
        #[arg(short, long)]
        app: Option<String>,
        /// Owner context (default: nobody)
        #[arg(short, long)]
        owner: Option<String>,
        /// Maximum number of collections to list
        #[arg(short, long, default_value = "30")]
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
        #[arg(short, long, default_value = "search")]
        app: String,
        /// Owner context
        #[arg(short, long, default_value = "nobody")]
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
        #[arg(short, long, default_value = "search")]
        app: String,
        /// Owner context
        #[arg(short, long, default_value = "nobody")]
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
        #[arg(short, long, default_value = "search")]
        app: String,
        /// Owner context
        #[arg(short, long, default_value = "nobody")]
        owner: String,
        /// MongoDB-style query (JSON)
        #[arg(short, long)]
        query: Option<String>,
        /// Maximum number of records
        #[arg(short, long, default_value = "100")]
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

    let mut client = crate::commands::build_client_from_config(&config)?;

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

    let mut client = crate::commands::build_client_from_config(&config)?;

    let count_u64 = u64::try_from(count).context("Invalid --count (value too large)")?;
    let offset_u64 = u64::try_from(offset).context("Invalid --offset (value too large)")?;
    let offset_param = if offset == 0 { None } else { Some(offset_u64) };

    let collections = tokio::select! {
        res = client.list_collections(app.as_deref(), owner.as_deref(), Some(count_u64), offset_param) => res?,
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
    let output = formatter.format_kvstore_collections(&collections)?;
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

    let mut client = crate::commands::build_client_from_config(&config)?;

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
    if !force {
        print!(
            "Are you sure you want to delete collection '{}'? [y/N] ",
            name
        );
        use std::io::Write;
        std::io::stdout().flush()?;

        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;

        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Delete cancelled.");
            return Ok(());
        }
    }

    info!("Deleting KVStore collection: {}", name);

    let mut client = crate::commands::build_client_from_config(&config)?;

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

    let mut client = crate::commands::build_client_from_config(&config)?;

    let count_u64 = u64::try_from(count).context("Invalid --count (value too large)")?;
    let offset_u64 = u64::try_from(offset).context("Invalid --offset (value too large)")?;
    let offset_param = if offset == 0 { None } else { Some(offset_u64) };

    let records = tokio::select! {
        res = client.list_collection_records(
            &name, &app, &owner, query.as_deref(), Some(count_u64), offset_param
        ) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    // Format and print results
    let output = formatter.format_kvstore_records(&records)?;
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
