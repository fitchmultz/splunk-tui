//! Internal logs command implementation.

use anyhow::{Context, Result};

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

pub async fn run(
    config: splunk_config::Config,
    count: usize,
    earliest: String,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    let mut client = crate::commands::build_client_from_config(&config)?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    let logs = tokio::select! {
        res = client.get_internal_logs(count as u64, Some(&earliest)) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    let output = formatter.format_logs(&logs)?;
    if let Some(ref path) = output_file {
        write_to_file(&output, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        println!("{}", output);
    }

    Ok(())
}
