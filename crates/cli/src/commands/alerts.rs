//! Fired alerts (triggered alert instances) command implementation.
//!
//! Responsibilities:
//! - List fired alerts with optional count limiting
//! - Show detailed information about specific fired alerts
//! - Format output via shared formatters
//!
//! Does NOT handle:
//! - Alert definition management (see saved_searches module)
//! - Direct REST API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Count parameter is validated to be positive
//! - Alert names are passed through without modification

use anyhow::Result;
use clap::Subcommand;
use tracing::info;

use splunk_config::constants::*;

use crate::formatters::{OutputFormat, get_formatter, output_result};

#[derive(Subcommand)]
pub enum AlertsCommand {
    /// List fired alerts (triggered alert instances)
    List {
        /// Maximum number of fired alerts to list
        #[arg(short, long, default_value_t = DEFAULT_LIST_PAGE_SIZE)]
        count: usize,
    },
    /// Show detailed information about a fired alert
    Info {
        /// Name of the fired alert
        #[arg(value_name = "NAME")]
        name: String,
    },
}

pub async fn run(
    config: splunk_config::Config,
    command: AlertsCommand,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        AlertsCommand::List { count } => {
            run_list(config, count, output_format, output_file.clone(), cancel).await
        }
        AlertsCommand::Info { name } => {
            run_info(config, &name, output_format, output_file.clone(), cancel).await
        }
    }
}

async fn run_list(
    config: splunk_config::Config,
    count: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Listing fired alerts (count: {})", count);

    let client = crate::commands::build_client_from_config(&config)?;

    let alerts = cancellable!(client.list_fired_alerts(Some(count), None), cancel)?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_fired_alerts(&alerts)?;
    output_result(&output, format, output_file.as_ref())?;
    Ok(())
}

async fn run_info(
    config: splunk_config::Config,
    name: &str,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Getting fired alert info for: {}", name);

    let client = crate::commands::build_client_from_config(&config)?;

    let alert = cancellable!(client.get_fired_alert(name), cancel)?;

    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let output = formatter.format_fired_alert_info(&alert)?;
    output_result(&output, format, output_file.as_ref())?;
    Ok(())
}
