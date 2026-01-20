//! Search command implementation.

use anyhow::Result;
use splunk_client::{AuthStrategy, SplunkClient};
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter};

pub async fn run(
    config: splunk_config::Config,
    query: String,
    wait: bool,
    earliest: Option<&str>,
    latest: Option<&str>,
    max_results: usize,
    output_format: &str,
) -> Result<()> {
    info!("Executing search: {}", query);

    let auth_strategy = match config.auth.strategy {
        splunk_config::AuthStrategy::SessionToken { username, password } => {
            AuthStrategy::SessionToken { username, password }
        }
        splunk_config::AuthStrategy::ApiToken { token } => AuthStrategy::ApiToken { token },
    };

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    info!("Connecting to {}", client.base_url());

    let results = client
        .search(&query, wait, earliest, latest, Some(max_results as u64))
        .await?;

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    // Format and print results
    let output = formatter.format_search_results(&results)?;
    print!("{}", output);

    Ok(())
}
