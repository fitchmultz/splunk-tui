//! Users command implementation.

use anyhow::{Context, Result};
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

pub async fn run(
    config: splunk_config::Config,
    count: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing users");

    let mut client = crate::commands::build_client_from_config(&config)?;

    let users = tokio::select! {
        res = client.list_users(Some(count as u64), None) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    let output = formatter.format_users(&users)?;
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
