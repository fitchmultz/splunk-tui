//! Indexes command implementation.

use anyhow::Result;
use splunk_client::SplunkClient;
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter};

pub async fn run(
    config: splunk_config::Config,
    detailed: bool,
    count: usize,
    output_format: &str,
) -> Result<()> {
    info!("Listing indexes");

    let auth_strategy = crate::commands::convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    let indexes = client.list_indexes(Some(count as u64), None).await?;

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    // Format and print indexes
    let output = formatter.format_indexes(&indexes, detailed)?;
    print!("{}", output);

    Ok(())
}
