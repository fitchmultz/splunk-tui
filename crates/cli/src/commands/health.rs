//! Health check command implementation.
//!
//! Responsibilities:
//! - Perform comprehensive health checks against Splunk instance
//! - Aggregate health status from multiple endpoints
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Health remediation or automated fixes
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Partial failures are logged but do not fail the overall command
//! - Health check results are aggregated from multiple endpoints

use anyhow::Result;
use tracing::{info, warn};

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, output_result};

pub async fn run(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Performing health check...");

    let client = crate::commands::build_client_from_config(&config)?;

    info!("Connecting to {}", client.base_url());

    // Use shared health check aggregation from client crate
    let health_result = tokio::select! {
        res = client.check_health_aggregate() => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    // Log partial errors as warnings (individual health checks that failed)
    for (endpoint, err) in &health_result.partial_errors {
        warn!("Failed to fetch {}: {}", endpoint, err);
    }

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    // Format and print results
    let output = formatter.format_health(&health_result.output)?;
    output_result(&output, format, output_file.as_ref())?;

    Ok(())
}
