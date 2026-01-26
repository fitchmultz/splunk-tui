//! Users command implementation.

use anyhow::{Context, Result};
use splunk_client::SplunkClient;
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter, write_to_file};

pub async fn run(
    config: splunk_config::Config,
    count: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    _cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing users");

    let auth_strategy = crate::commands::convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    let users = client.list_users(Some(count as u64), None).await?;

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
