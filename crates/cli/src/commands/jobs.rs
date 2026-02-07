//! Search jobs command implementation.
//!
//! Responsibilities:
//! - List search jobs with optional count limiting
//! - Inspect detailed information about specific jobs
//! - Cancel running jobs by SID
//! - Delete completed jobs by SID
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

use anyhow::{Context, Result};
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

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
    cancel_token: &crate::cancellation::CancellationToken,
) -> Result<()> {
    let client = crate::commands::build_client_from_config(&config)?;

    // If inspect, cancel, delete, or results action is specified, don't list jobs
    if inspect.is_some() || cancel.is_some() || delete.is_some() || results.is_some() {
        list = false;
    }

    // Handle results action
    if let Some(sid) = results {
        info!("Fetching results for job: {}", sid);
        let count = result_count.unwrap_or(100) as u64;
        let spinner =
            crate::progress::Spinner::new(!quiet, format!("Fetching results for job {}", sid));
        let search_results = tokio::select! {
            res = client.get_search_results(&sid, count, result_offset as u64) => res?,
            _ = cancel_token.cancelled() => return Err(Cancelled.into()),
        };
        spinner.finish();

        // Parse output format
        let format = OutputFormat::from_str(output_format)?;
        let formatter = get_formatter(format);

        // Format and print results
        let output = formatter.format_search_results(&search_results.results)?;
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

    // Handle inspect action (NEW)
    if let Some(sid) = inspect {
        info!("Inspecting job: {}", sid);
        let job = tokio::select! {
            res = client.get_job_status(&sid) => res?,
            _ = cancel_token.cancelled() => return Err(Cancelled.into()),
        };

        // Parse output format
        let format = OutputFormat::from_str(output_format)?;
        let formatter = get_formatter(format);

        // Format and print job details
        let output = formatter.format_job_details(&job)?;
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

    if let Some(sid) = cancel {
        info!("Canceling job: {}", sid);
        let spinner = crate::progress::Spinner::new(!quiet, format!("Canceling job {}", sid));
        tokio::select! {
            res = client.cancel_job(&sid) => res?,
            _ = cancel_token.cancelled() => return Err(Cancelled.into()),
        };
        spinner.finish();
        println!("Job {} canceled.", sid);
        return Ok(());
    }

    if let Some(sid) = delete {
        info!("Deleting job: {}", sid);
        let spinner = crate::progress::Spinner::new(!quiet, format!("Deleting job {}", sid));
        tokio::select! {
            res = client.delete_job(&sid) => res?,
            _ = cancel_token.cancelled() => return Err(Cancelled.into()),
        };
        spinner.finish();
        println!("Job {} deleted.", sid);
        return Ok(());
    }

    if list {
        info!("Listing search jobs");
        let jobs = tokio::select! {
            res = client.list_jobs(Some(count as u64), None) => res?,
            _ = cancel_token.cancelled() => return Err(Cancelled.into()),
        };

        // Parse output format
        let format = OutputFormat::from_str(output_format)?;
        let formatter = get_formatter(format);

        // Format and print jobs
        let output = formatter.format_jobs(&jobs)?;
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
    }

    Ok(())
}
