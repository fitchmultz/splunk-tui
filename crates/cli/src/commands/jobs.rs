//! Jobs command implementation.

use anyhow::{Context, Result};
use splunk_client::SplunkClient;
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
    count: usize,
    output_format: &str,
    quiet: bool,
    output_file: Option<std::path::PathBuf>,
    cancel_token: &crate::cancellation::CancellationToken,
) -> Result<()> {
    let auth_strategy = crate::commands::convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .session_ttl_seconds(config.connection.session_ttl_seconds)
        .session_expiry_buffer_seconds(config.connection.session_expiry_buffer_seconds)
        .build()?;

    // If inspect, cancel, or delete action is specified, don't list jobs
    if inspect.is_some() || cancel.is_some() || delete.is_some() {
        list = false;
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
