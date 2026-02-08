//! Lookup tables command implementation.
//!
//! Responsibilities:
//! - List lookup tables with optional count limiting and pagination
//! - Download lookup table files as CSV
//! - Upload or replace lookup table files
//! - Delete lookup tables with confirmation
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Lookup table content editing
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Count and offset parameters are validated for safe pagination
//! - Delete operations require confirmation unless --force is used

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, output_result};
use splunk_config::constants::*;

#[derive(Subcommand)]
pub enum LookupsCommand {
    /// List all lookup tables (default)
    List {
        /// Maximum number of lookup tables to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,
        /// Offset into the lookup table list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
    },
    /// Download a lookup table file as CSV
    Download {
        /// The lookup name to download
        name: String,
        /// Output file path (defaults to lookup name + .csv)
        #[arg(short, long, value_name = "FILE")]
        output: Option<std::path::PathBuf>,
        /// App namespace (defaults to "search")
        #[arg(long)]
        app: Option<String>,
        /// Owner namespace (defaults to "-" for all users)
        #[arg(long)]
        owner: Option<String>,
    },
    /// Upload or replace a lookup table file
    Upload {
        /// The CSV file to upload
        file: std::path::PathBuf,
        /// The lookup name (defaults to filename without extension)
        #[arg(short, long)]
        name: Option<String>,
        /// App namespace (defaults to "search")
        #[arg(long)]
        app: Option<String>,
        /// Owner namespace (defaults to "-" for all users)
        #[arg(long)]
        owner: Option<String>,
        /// Sharing level (user, app, global)
        #[arg(long)]
        sharing: Option<String>,
    },
    /// Delete a lookup table file
    Delete {
        /// The lookup name to delete
        name: String,
        /// App namespace (defaults to "search")
        #[arg(long)]
        app: Option<String>,
        /// Owner namespace (defaults to "-" for all users)
        #[arg(long)]
        owner: Option<String>,
        /// Skip confirmation prompt
        #[arg(short, long)]
        force: bool,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: Option<LookupsCommand>,
    count: usize,  // deprecated fallback
    offset: usize, // deprecated fallback
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel_token: &crate::cancellation::CancellationToken,
) -> Result<()> {
    // Handle backward compatibility
    let cmd = match command {
        Some(cmd) => cmd,
        None => LookupsCommand::List { count, offset },
    };

    match cmd {
        LookupsCommand::List { count, offset } => {
            run_list(
                config,
                count,
                offset,
                output_format,
                output_file,
                cancel_token,
            )
            .await
        }
        LookupsCommand::Download {
            name,
            output,
            app,
            owner,
        } => run_download(config, &name, output, app, owner, cancel_token).await,
        LookupsCommand::Upload {
            file,
            name,
            app,
            owner,
            sharing,
        } => run_upload(config, &file, name, app, owner, sharing, cancel_token).await,
        LookupsCommand::Delete {
            name,
            app,
            owner,
            force,
        } => run_delete(config, &name, app, owner, force, cancel_token).await,
    }
}

async fn run_list(
    config: splunk_config::Config,
    count: usize,
    offset: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel_token: &crate::cancellation::CancellationToken,
) -> Result<()> {
    let client = crate::commands::build_client_from_config(&config)?;

    info!("Listing lookup tables");
    let lookups = tokio::select! {
        res = client.list_lookup_tables(Some(count), Some(offset)) => res?,
        _ = cancel_token.cancelled() => return Err(Cancelled.into()),
    };

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    // Format and print lookups
    let output = formatter.format_lookups(&lookups)?;
    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}

async fn run_download(
    config: splunk_config::Config,
    name: &str,
    output: Option<std::path::PathBuf>,
    app: Option<String>,
    owner: Option<String>,
    cancel_token: &crate::cancellation::CancellationToken,
) -> Result<()> {
    let client = crate::commands::build_client_from_config(&config)?;

    info!("Downloading lookup table: {}", name);
    let content = tokio::select! {
        res = client.download_lookup_table(name, app.as_deref(), owner.as_deref()) => res?,
        _ = cancel_token.cancelled() => return Err(Cancelled.into()),
    };

    // Determine output path
    let output_path = match output {
        Some(path) => path,
        None => std::path::PathBuf::from(format!("{}.csv", name)),
    };

    // Write content to file
    tokio::fs::write(&output_path, content)
        .await
        .with_context(|| format!("Failed to write output to {}", output_path.display()))?;

    println!("Lookup '{}' downloaded to {}", name, output_path.display());

    Ok(())
}

async fn run_upload(
    config: splunk_config::Config,
    file: &std::path::PathBuf,
    name: Option<String>,
    app: Option<String>,
    owner: Option<String>,
    sharing: Option<String>,
    cancel_token: &crate::cancellation::CancellationToken,
) -> Result<()> {
    // Read file content
    let content = tokio::fs::read(file)
        .await
        .with_context(|| format!("Failed to read file: {}", file.display()))?;

    // Determine lookup name from filename if not provided
    let lookup_name = match name {
        Some(n) => n,
        None => file
            .file_stem()
            .and_then(|s| s.to_str())
            .context("Failed to determine lookup name from filename")?
            .to_string(),
    };

    // Determine filename from path
    let filename = file
        .file_name()
        .and_then(|s| s.to_str())
        .context("Failed to determine filename from path")?
        .to_string();

    let client = crate::commands::build_client_from_config(&config)?;

    let params = splunk_client::UploadLookupParams {
        name: lookup_name.clone(),
        filename,
        content,
        app,
        owner,
        sharing,
    };

    info!("Uploading lookup table: {}", lookup_name);
    let lookup = tokio::select! {
        res = client.upload_lookup_table(&params) => res?,
        _ = cancel_token.cancelled() => return Err(Cancelled.into()),
    };

    println!("Lookup '{}' uploaded successfully.", lookup.name);

    Ok(())
}

async fn run_delete(
    config: splunk_config::Config,
    name: &str,
    app: Option<String>,
    owner: Option<String>,
    force: bool,
    cancel_token: &crate::cancellation::CancellationToken,
) -> Result<()> {
    if !force && !crate::interactive::confirm_delete(name, "lookup")? {
        return Ok(());
    }

    info!("Deleting lookup table: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    tokio::select! {
        res = client.delete_lookup_table(name, app.as_deref(), owner.as_deref()) => {
            res?;
            println!("Lookup '{}' deleted successfully.", name);
            Ok(())
        }
        _ = cancel_token.cancelled() => Err(Cancelled.into()),
    }
}
