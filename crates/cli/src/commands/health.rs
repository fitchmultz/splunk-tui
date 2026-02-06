//! Health command implementation.

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

pub async fn run(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Performing health check...");

    let mut client = crate::commands::build_client_from_config(&config)?;

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
