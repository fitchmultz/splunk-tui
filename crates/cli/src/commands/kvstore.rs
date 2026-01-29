//! KVStore command implementation.

use anyhow::{Context, Result};
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

pub async fn run(
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
