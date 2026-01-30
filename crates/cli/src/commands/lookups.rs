//! Lookups command implementation.

use anyhow::{Context, Result};
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

pub async fn run(
    config: splunk_config::Config,
    count: usize,
    offset: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel_token: &crate::cancellation::CancellationToken,
) -> Result<()> {
    let mut client = crate::commands::build_client_from_config(&config)?;

    info!("Listing lookup tables");
    let lookups = tokio::select! {
        res = client.list_lookup_tables(Some(count as u32), Some(offset as u32)) => res?,
        _ = cancel_token.cancelled() => return Err(Cancelled.into()),
    };

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    // Format and print lookups
    let output = formatter.format_lookups(&lookups)?;
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
