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

use anyhow::Result;
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter, output_result};

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

    // Handle inspect action (NEW)
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

    if let Some(sid) = cancel {
        info!("Canceling job: {}", sid);
        let spinner = crate::progress::Spinner::new(!quiet, format!("Canceling job {}", sid));
        cancellable!(client.cancel_job(&sid), cancel_token)?;
        spinner.finish();
        println!("Job {} canceled.", sid);
        return Ok(());
    }

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
