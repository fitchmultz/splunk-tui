//! License command implementation.
//!
//! Responsibilities:
//! - Fetch license usage, pools, and stacks from Splunk.
//! - Format and display license information.
//!
//! Does NOT handle:
//! - License management (activation, deletion).
//!
//! Invariants / Assumptions:
//! - Requires an authenticated Splunk client.

use anyhow::{Context, Result};
use clap::Args;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{LicenseInfoOutput, OutputFormat, get_formatter, write_to_file};

/// Display license information.
#[derive(Args, Debug)]
pub struct LicenseArgs {}

/// Run the license command.
pub async fn run(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Fetching license information...");

    let mut client = crate::commands::build_client_from_config(&config)?;

    let usage = tokio::select! {
        res = client.get_license_usage() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };
    let pools = tokio::select! {
        res = client.list_license_pools() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };
    let stacks = tokio::select! {
        res = client.list_license_stacks() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let output = LicenseInfoOutput {
        usage,
        pools,
        stacks,
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let formatted = formatter.format_license(&output)?;

    if let Some(ref path) = output_file {
        write_to_file(&formatted, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        println!("{}", formatted);
    }

    Ok(())
}
