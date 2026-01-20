//! Indexes command implementation.

use anyhow::Result;
use splunk_client::{AuthStrategy, SplunkClient};
use tracing::info;

pub async fn run(
    config: splunk_config::Config,
    detailed: bool,
    count: usize,
    _output_format: &str,
) -> Result<()> {
    info!("Listing indexes");

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

    let indexes = client.list_indexes(Some(count as u64), None).await?;

    println!("Found {} indexes:\n", indexes.len());

    for index in indexes {
        println!("  Name: {}", index.name);
        if detailed {
            println!("    Size: {} MB", index.current_db_size_mb);
            println!("    Events: {}", index.total_event_count);
            if let Some(max_size) = index.max_total_data_size_mb {
                println!("    Max Size: {} MB", max_size);
            }
            if let Some(frozen_time) = index.frozen_time_period_in_secs {
                let days = frozen_time / 86400;
                println!("    Retention: {} days", days);
            }
            if let Some(home_path) = index.home_path {
                println!("    Path: {}", home_path);
            }
        }
        println!();
    }

    Ok(())
}
