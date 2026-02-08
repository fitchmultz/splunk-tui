//! Search jobs command implementation.
//!
//! Responsibilities:
//! - List search jobs with optional count limiting
//! - Inspect detailed information about specific jobs
//! - Cancel running jobs by SID (single or batch)
//! - Delete completed jobs by SID (single or batch)
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Job creation (see search module)
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Job SIDs are validated as non-empty strings
//! - Cancel/delete operations are idempotent (safe to retry)
//! - Only the job owner or admin can cancel/delete jobs
//! - Batch operations process jobs sequentially to avoid API throttling

use anyhow::{Context, Result};
use clap::Subcommand;
use std::io::Write;
use std::path::PathBuf;
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter, output_result};

/// Jobs subcommands for batch operations.
#[derive(Subcommand)]
pub enum JobsCommand {
    /// Cancel one or more search jobs
    Cancel {
        /// Job SIDs to cancel
        sids: Vec<String>,

        /// Read SIDs from file (one per line, comments start with #)
        #[arg(long, value_name = "FILE", conflicts_with = "sids")]
        file: Option<PathBuf>,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },

    /// Delete one or more search jobs
    Delete {
        /// Job SIDs to delete
        sids: Vec<String>,

        /// Read SIDs from file (one per line, comments start with #)
        #[arg(long, value_name = "FILE", conflicts_with = "sids")]
        file: Option<PathBuf>,

        /// Skip confirmation prompt
        #[arg(long)]
        force: bool,
    },
}

#[allow(clippy::too_many_arguments)]
pub async fn run(
    config: splunk_config::Config,
    mut list: bool,
    inspect: Option<String>,
    cancel: Option<String>,
    delete: Option<String>,
    results: Option<String>,
    result_count: Option<usize>,
    result_offset: usize,
    count: usize,
    output_format: &str,
    quiet: bool,
    output_file: Option<std::path::PathBuf>,
    command: Option<JobsCommand>,
    cancel_token: &crate::cancellation::CancellationToken,
) -> Result<()> {
    let client = crate::commands::build_client_from_config(&config)?;

    // Handle new subcommand-based operations first
    if let Some(cmd) = command {
        match cmd {
            JobsCommand::Cancel { sids, file, force } => {
                return run_cancel_batch(
                    std::sync::Arc::new(client),
                    sids,
                    file,
                    force,
                    quiet,
                    cancel_token,
                )
                .await;
            }
            JobsCommand::Delete { sids, file, force } => {
                return run_delete_batch(
                    std::sync::Arc::new(client),
                    sids,
                    file,
                    force,
                    quiet,
                    cancel_token,
                )
                .await;
            }
        }
    }

    // If inspect, cancel, delete, or results action is specified, don't list jobs
    if inspect.is_some() || cancel.is_some() || delete.is_some() || results.is_some() {
        list = false;
    }

    // Handle results action
    if let Some(sid) = results {
        info!("Fetching results for job: {}", sid);
        let count = result_count.unwrap_or(100);
        let spinner =
            crate::progress::Spinner::new(!quiet, format!("Fetching results for job {}", sid));
        let search_results = cancellable!(
            client.get_search_results(&sid, count, result_offset),
            cancel_token
        )?;
        spinner.finish();

        // Parse output format
        let format = OutputFormat::from_str(output_format)?;
        let formatter = get_formatter(format);

        // Format and print results
        let output = formatter.format_search_results(&search_results.results)?;
        output_result(&output, format, output_file.as_ref())?;
        return Ok(());
    }

    // Handle inspect action
    if let Some(sid) = inspect {
        info!("Inspecting job: {}", sid);
        let job = cancellable!(client.get_job_status(&sid), cancel_token)?;

        // Parse output format
        let format = OutputFormat::from_str(output_format)?;
        let formatter = get_formatter(format);

        // Format and print job details
        let output = formatter.format_job_details(&job)?;
        output_result(&output, format, output_file.as_ref())?;
        return Ok(());
    }

    // Handle legacy single cancel action
    if let Some(sid) = cancel {
        info!("Canceling job: {}", sid);
        let spinner = crate::progress::Spinner::new(!quiet, format!("Canceling job {}", sid));
        cancellable!(client.cancel_job(&sid), cancel_token)?;
        spinner.finish();
        println!("Job {} canceled.", sid);
        return Ok(());
    }

    // Handle legacy single delete action
    if let Some(sid) = delete {
        info!("Deleting job: {}", sid);
        let spinner = crate::progress::Spinner::new(!quiet, format!("Deleting job {}", sid));
        cancellable!(client.delete_job(&sid), cancel_token)?;
        spinner.finish();
        println!("Job {} deleted.", sid);
        return Ok(());
    }

    if list {
        info!("Listing search jobs");
        let jobs = cancellable!(client.list_jobs(Some(count), None), cancel_token)?;

        // Parse output format
        let format = OutputFormat::from_str(output_format)?;
        let formatter = get_formatter(format);

        // Format and print jobs
        let output = formatter.format_jobs(&jobs)?;
        output_result(&output, format, output_file.as_ref())?;
    }

    Ok(())
}

/// Run batch cancel operation.
async fn run_cancel_batch(
    client: std::sync::Arc<splunk_client::SplunkClient>,
    sids: Vec<String>,
    file: Option<PathBuf>,
    force: bool,
    quiet: bool,
    cancel_token: &crate::cancellation::CancellationToken,
) -> Result<()> {
    // Collect SIDs from args and/or file
    let mut all_sids = sids;

    if let Some(path) = file {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read SIDs file: {}", path.display()))?;
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                all_sids.push(trimmed.to_string());
            }
        }
    }

    if all_sids.is_empty() {
        anyhow::bail!("Failed to cancel jobs: No job SIDs provided");
    }

    // Deduplicate SIDs
    all_sids.sort_unstable();
    all_sids.dedup();

    // Confirmation prompt (unless --force)
    if !force && all_sids.len() > 1 {
        print!(
            "Are you sure you want to cancel {} jobs? [y/N] ",
            all_sids.len()
        );
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Cancel operation aborted.");
            return Ok(());
        }
    }

    // Process jobs sequentially (following TUI pattern)
    let spinner =
        crate::progress::Spinner::new(!quiet, format!("Canceling {} jobs", all_sids.len()));
    let mut success_count = 0;
    let mut errors = Vec::new();

    for sid in &all_sids {
        // Check cancellation
        if cancel_token.is_cancelled() {
            spinner.finish();
            anyhow::bail!("Operation cancelled by user");
        }

        match client.cancel_job(sid).await {
            Ok(_) => success_count += 1,
            Err(e) => errors.push(format!("{}: {}", sid, e)),
        }
    }

    spinner.finish();

    // Report results
    if success_count > 0 {
        println!("Cancelled {} job(s)", success_count);
    }

    if !errors.is_empty() {
        eprintln!("\nErrors:");
        for err in &errors {
            eprintln!("  {}", err);
        }
        // Return error if any failures occurred
        anyhow::bail!("Failed to cancel {} job(s)", errors.len());
    }

    Ok(())
}

/// Run batch delete operation.
async fn run_delete_batch(
    client: std::sync::Arc<splunk_client::SplunkClient>,
    sids: Vec<String>,
    file: Option<PathBuf>,
    force: bool,
    quiet: bool,
    cancel_token: &crate::cancellation::CancellationToken,
) -> Result<()> {
    // Collect SIDs from args and/or file
    let mut all_sids = sids;

    if let Some(path) = file {
        let content = std::fs::read_to_string(&path)
            .with_context(|| format!("Failed to read SIDs file: {}", path.display()))?;
        for line in content.lines() {
            let trimmed = line.trim();
            if !trimmed.is_empty() && !trimmed.starts_with('#') {
                all_sids.push(trimmed.to_string());
            }
        }
    }

    if all_sids.is_empty() {
        anyhow::bail!("Failed to delete jobs: No job SIDs provided");
    }

    // Deduplicate SIDs
    all_sids.sort_unstable();
    all_sids.dedup();

    // Confirmation prompt (unless --force)
    if !force && all_sids.len() > 1 {
        print!(
            "Are you sure you want to delete {} jobs? [y/N] ",
            all_sids.len()
        );
        std::io::stdout().flush()?;
        let mut input = String::new();
        std::io::stdin().read_line(&mut input)?;
        if !input.trim().eq_ignore_ascii_case("y") {
            println!("Delete operation aborted.");
            return Ok(());
        }
    }

    // Process jobs sequentially (following TUI pattern)
    let spinner =
        crate::progress::Spinner::new(!quiet, format!("Deleting {} jobs", all_sids.len()));
    let mut success_count = 0;
    let mut errors = Vec::new();

    for sid in &all_sids {
        // Check cancellation
        if cancel_token.is_cancelled() {
            spinner.finish();
            anyhow::bail!("Operation cancelled by user");
        }

        match client.delete_job(sid).await {
            Ok(_) => success_count += 1,
            Err(e) => errors.push(format!("{}: {}", sid, e)),
        }
    }

    spinner.finish();

    // Report results
    if success_count > 0 {
        println!("Deleted {} job(s)", success_count);
    }

    if !errors.is_empty() {
        eprintln!("\nErrors:");
        for err in &errors {
            eprintln!("  {}", err);
        }
        // Return error if any failures occurred
        anyhow::bail!("Failed to delete {} job(s)", errors.len());
    }

    Ok(())
}
