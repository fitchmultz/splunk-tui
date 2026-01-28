//! Health command implementation.

use anyhow::{Context, Result};
use splunk_client::SplunkClient;
use tracing::{info, warn};

use crate::formatters::{HealthCheckOutput, OutputFormat, get_formatter, write_to_file};

pub async fn run(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    _cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Performing health check...");

    let auth_strategy = crate::commands::convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .session_ttl_seconds(config.connection.session_ttl_seconds)
        .session_expiry_buffer_seconds(config.connection.session_expiry_buffer_seconds)
        .build()?;

    info!("Connecting to {}", client.base_url());

    // Fetch health data parts
    // Server info is required for a basic health check
    let server_info = Some(client.get_server_info().await?);

    let splunkd_health = match client.get_health().await {
        Ok(health) => Some(health),
        Err(e) => {
            warn!("Failed to fetch splunkd health: {}", e);
            None
        }
    };

    let license_usage = match client.get_license_usage().await {
        Ok(usage) => Some(usage),
        Err(e) => {
            warn!("Failed to fetch license usage: {}", e);
            None
        }
    };

    let kvstore_status = match client.get_kvstore_status().await {
        Ok(status) => Some(status),
        Err(e) => {
            warn!("Failed to fetch KVStore status: {}", e);
            None
        }
    };

    let log_parsing_health = match client.check_log_parsing_health().await {
        Ok(health) => Some(health),
        Err(e) => {
            warn!("Failed to fetch log parsing health: {}", e);
            None
        }
    };

    let health_output = HealthCheckOutput {
        server_info,
        splunkd_health,
        license_usage,
        kvstore_status,
        log_parsing_health,
    };

    // Parse output format
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);

    // Format and print results
    let output = formatter.format_health(&health_output)?;
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
