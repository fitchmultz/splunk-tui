//! Internal logs command implementation.

use anyhow::Result;
use splunk_client::{AuthStrategy, SplunkClient};
use tokio::time::{Duration, sleep};
use tracing::info;

use crate::formatters::{OutputFormat, get_formatter};

pub async fn run(
    config: splunk_config::Config,
    count: usize,
    earliest: String,
    tail: bool,
    output_format: &str,
) -> Result<()> {
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

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    if tail {
        info!("Tailing internal logs...");
        let mut last_seen_time: Option<String> = None;

        loop {
            // If we have a last seen time, use it as earliest for next poll
            // Splunk's _time is usually precise enough.
            let current_earliest = last_seen_time.as_deref().unwrap_or(&earliest);

            match client
                .get_internal_logs(count as u64, Some(current_earliest))
                .await
            {
                Ok(logs) => {
                    if !logs.is_empty() {
                        // Filter out logs we've already seen if we're using the same timestamp
                        let new_logs: Vec<_> = if let Some(last_time) = &last_seen_time {
                            logs.into_iter().filter(|l| &l.time > last_time).collect()
                        } else {
                            logs
                        };

                        if !new_logs.is_empty() {
                            // Update last seen time to the latest log entry
                            if let Some(latest) = new_logs.first() {
                                last_seen_time = Some(latest.time.clone());
                            }

                            // Print new logs (we reverse because Splunk usually returns newest first for head)
                            // But get_internal_logs uses | head, which returns first matches.
                            // Actually Splunk search usually returns newest first.
                            let output = formatter.format_logs(&new_logs)?;
                            if !output.trim().is_empty() {
                                println!("{}", output.trim());
                            }
                        }
                    }
                }
                Err(e) => {
                    eprintln!("Error fetching logs: {}", e);
                }
            }

            sleep(Duration::from_secs(2)).await;
        }
    } else {
        info!("Fetching internal logs...");
        let logs = client
            .get_internal_logs(count as u64, Some(&earliest))
            .await?;
        let output = formatter.format_logs(&logs)?;
        println!("{}", output);
    }

    Ok(())
}
