//! License command implementation.

use anyhow::{Context, Result};
use clap::Args;
use splunk_client::SplunkClient;
use tracing::info;

use crate::formatters::{LicenseInfoOutput, OutputFormat, get_formatter, write_to_file};

/// Display license information.
#[derive(Args, Debug)]
pub struct LicenseArgs {
    /// Output format (json, table, csv, xml).
    #[arg(short, long, default_value = "table")]
    pub format: String,
}

/// Run the license command.
pub async fn run(
    config: splunk_config::Config,
    args: &LicenseArgs,
    output_file: Option<std::path::PathBuf>,
) -> Result<()> {
    info!("Fetching license information...");

    let auth_strategy = crate::commands::convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    let usage = client.get_license_usage().await?;
    let pools = client.list_license_pools().await?;
    let stacks = client.list_license_stacks().await?;

    let output = LicenseInfoOutput {
        usage,
        pools,
        stacks,
    };

    let format = OutputFormat::from_str(&args.format)?;
    let formatter = get_formatter(format);
    let formatted = formatter.format_license(&output)?;

    if let Some(ref path) = output_file {
        write_to_file(&formatted, path)
            .with_context(|| format!("Failed to write output to {}", path.display()))?;
        eprintln!(
            "Results written to {} ({:?} format)",
            path.display(),
            format
        );
    } else {
        println!("{}", formatted);
    }

    Ok(())
}
