//! Internal logs command implementation.

use anyhow::Result;
use splunk_client::SplunkClient;

use crate::formatters::{OutputFormat, get_formatter};

pub async fn run(
    config: splunk_config::Config,
    count: usize,
    earliest: String,
    output_format: &str,
) -> Result<()> {
    let auth_strategy = crate::commands::convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    let logs = client
        .get_internal_logs(count as u64, Some(&earliest))
        .await?;

    let output = formatter.format_logs(&logs)?;
    println!("{}", output);

    Ok(())
}
