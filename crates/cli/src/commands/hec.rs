//! HEC (HTTP Event Collector) command implementation.
//!
//! This module provides CLI commands for sending events to Splunk via HEC.
//! HEC uses a separate endpoint (typically port 8088) and separate authentication
//! (HEC tokens) from the standard Splunk REST API.
//!
//! # Environment Variables
//! - `SPLUNK_HEC_URL`: HEC endpoint URL (e.g., `https://localhost:8088`)
//! - `SPLUNK_HEC_TOKEN`: HEC authentication token
//!
//! # What this module handles:
//! - Sending single events with optional metadata
//! - Sending batches of events from JSON files
//! - Checking HEC health status
//! - Querying acknowledgment status for guaranteed delivery
//!
//! # What this module does NOT handle:
//! - Direct HTTP implementation (see `crates/client`)
//! - Token management or configuration storage

use anyhow::{Context, Result};
use clap::Subcommand;
use std::path::PathBuf;
use tracing::info;

use crate::cancellation::Cancelled;
use crate::formatters::{OutputFormat, get_formatter, write_to_file};

/// HEC subcommands.
#[derive(Subcommand)]
pub enum HecCommand {
    /// Send a single event to HEC.
    Send {
        /// The event data as JSON string or @file.json to read from file.
        event: String,

        /// HEC URL (e.g., https://localhost:8088).
        #[arg(long, env = "SPLUNK_HEC_URL")]
        hec_url: String,

        /// HEC token for authentication.
        #[arg(long, env = "SPLUNK_HEC_TOKEN")]
        hec_token: String,

        /// Destination index (optional).
        #[arg(long)]
        index: Option<String>,

        /// Source field (optional).
        #[arg(long)]
        source: Option<String>,

        /// Sourcetype field (optional).
        #[arg(long)]
        sourcetype: Option<String>,

        /// Host field (optional).
        #[arg(long)]
        host: Option<String>,

        /// Event timestamp as Unix epoch (optional).
        #[arg(long)]
        time: Option<f64>,
    },

    /// Send a batch of events to HEC.
    SendBatch {
        /// Path to JSON file containing array of events.
        events_file: PathBuf,

        /// HEC URL (e.g., https://localhost:8088).
        #[arg(long, env = "SPLUNK_HEC_URL")]
        hec_url: String,

        /// HEC token for authentication.
        #[arg(long, env = "SPLUNK_HEC_TOKEN")]
        hec_token: String,

        /// Use newline-delimited JSON format instead of JSON array.
        #[arg(long)]
        ndjson: bool,
    },

    /// Check HEC health endpoint.
    Health {
        /// HEC URL (e.g., https://localhost:8088).
        #[arg(long, env = "SPLUNK_HEC_URL")]
        hec_url: String,

        /// HEC token for authentication.
        #[arg(long, env = "SPLUNK_HEC_TOKEN")]
        hec_token: String,
    },

    /// Check acknowledgment status for previously sent events.
    CheckAck {
        /// HEC URL (e.g., https://localhost:8088).
        #[arg(long, env = "SPLUNK_HEC_URL")]
        hec_url: String,

        /// HEC token for authentication.
        #[arg(long, env = "SPLUNK_HEC_TOKEN")]
        hec_token: String,

        /// Acknowledgment IDs to check (comma-separated).
        #[arg(long, value_delimiter = ',')]
        ack_ids: Vec<u64>,
    },
}

/// Run the HEC command.
pub async fn run(
    command: HecCommand,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        HecCommand::Send {
            event,
            hec_url,
            hec_token,
            index,
            source,
            sourcetype,
            host,
            time,
        } => {
            run_send(
                event,
                hec_url,
                hec_token,
                index,
                source,
                sourcetype,
                host,
                time,
                output_format,
                output_file,
                cancel,
            )
            .await
        }
        HecCommand::SendBatch {
            events_file,
            hec_url,
            hec_token,
            ndjson,
        } => {
            run_send_batch(
                events_file,
                hec_url,
                hec_token,
                ndjson,
                output_format,
                output_file,
                cancel,
            )
            .await
        }
        HecCommand::Health { hec_url, hec_token } => {
            run_health(hec_url, hec_token, output_format, output_file, cancel).await
        }
        HecCommand::CheckAck {
            hec_url,
            hec_token,
            ack_ids,
        } => {
            run_check_ack(
                hec_url,
                hec_token,
                ack_ids,
                output_format,
                output_file,
                cancel,
            )
            .await
        }
    }
}

#[allow(clippy::too_many_arguments)]
async fn run_send(
    event_data: String,
    hec_url: String,
    hec_token: String,
    index: Option<String>,
    source: Option<String>,
    sourcetype: Option<String>,
    host: Option<String>,
    time: Option<f64>,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Sending single event to HEC at {}", hec_url);

    // Parse event data (handle @file.json syntax)
    let event_json = if let Some(file_path) = event_data.strip_prefix('@') {
        let content = std::fs::read_to_string(file_path)
            .with_context(|| format!("Failed to read event file: {}", file_path))?;
        serde_json::from_str(&content)
            .with_context(|| format!("Invalid JSON in event file: {}", file_path))?
    } else {
        serde_json::from_str(&event_data).with_context(|| "Invalid JSON in event argument")?
    };

    // Build HEC event
    let mut event = splunk_client::HecEvent::new(event_json);
    if let Some(idx) = index {
        event.index = Some(idx);
    }
    if let Some(src) = source {
        event.source = Some(src);
    }
    if let Some(st) = sourcetype {
        event.sourcetype = Some(st);
    }
    if let Some(h) = host {
        event.host = Some(h);
    }
    if let Some(t) = time {
        event.time = Some(t);
    }

    // Create a minimal client (HEC doesn't use standard auth)
    let client = splunk_client::SplunkClient::builder()
        .base_url(hec_url.clone())
        .auth_strategy(splunk_client::AuthStrategy::ApiToken {
            token: secrecy::SecretString::new(hec_token.clone().into()),
        })
        .build()
        .context("Failed to create HEC client")?;

    let response = tokio::select! {
        res = client.hec_send_event(&hec_url, &hec_token, &event) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    // Format output
    let format = OutputFormat::from_str(output_format)?;
    let output = if format == OutputFormat::Table {
        format_hec_response_table(&response)?
    } else {
        let formatter = get_formatter(format);
        formatter.format_hec_response(&response)?
    };

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

async fn run_send_batch(
    events_file: PathBuf,
    hec_url: String,
    hec_token: String,
    ndjson: bool,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!(
        "Sending batch of events from {} to HEC at {}",
        events_file.display(),
        hec_url
    );

    // Read events from file
    let content = std::fs::read_to_string(&events_file)
        .with_context(|| format!("Failed to read events file: {}", events_file.display()))?;

    let events: Vec<splunk_client::HecEvent> = serde_json::from_str(&content)
        .with_context(|| format!("Invalid JSON in events file: {}", events_file.display()))?;

    if events.is_empty() {
        anyhow::bail!("No events found in file");
    }

    // Create a minimal client (HEC doesn't use standard auth)
    let client = splunk_client::SplunkClient::builder()
        .base_url(hec_url.clone())
        .auth_strategy(splunk_client::AuthStrategy::ApiToken {
            token: secrecy::SecretString::new(hec_token.clone().into()),
        })
        .build()
        .context("Failed to create HEC client")?;

    let response = tokio::select! {
        res = client.hec_send_batch(&hec_url, &hec_token, &events, ndjson) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    // Format output
    let format = OutputFormat::from_str(output_format)?;
    let output = if format == OutputFormat::Table {
        format_hec_batch_response_table(&response)?
    } else {
        let formatter = get_formatter(format);
        formatter.format_hec_batch_response(&response)?
    };

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

async fn run_health(
    hec_url: String,
    hec_token: String,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Checking HEC health at {}", hec_url);

    // Create a minimal client (HEC doesn't use standard auth)
    let client = splunk_client::SplunkClient::builder()
        .base_url(hec_url.clone())
        .auth_strategy(splunk_client::AuthStrategy::ApiToken {
            token: secrecy::SecretString::new(hec_token.clone().into()),
        })
        .build()
        .context("Failed to create HEC client")?;

    let health = tokio::select! {
        res = client.hec_health_check(&hec_url, &hec_token) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    // Format output
    let format = OutputFormat::from_str(output_format)?;
    let output = if format == OutputFormat::Table {
        format_hec_health_table(&health)?
    } else {
        let formatter = get_formatter(format);
        formatter.format_hec_health(&health)?
    };

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

async fn run_check_ack(
    hec_url: String,
    hec_token: String,
    ack_ids: Vec<u64>,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!(
        "Checking HEC acknowledgment status for {} IDs",
        ack_ids.len()
    );

    if ack_ids.is_empty() {
        anyhow::bail!("No acknowledgment IDs provided");
    }

    // Create a minimal client (HEC doesn't use standard auth)
    let client = splunk_client::SplunkClient::builder()
        .base_url(hec_url.clone())
        .auth_strategy(splunk_client::AuthStrategy::ApiToken {
            token: secrecy::SecretString::new(hec_token.clone().into()),
        })
        .build()
        .context("Failed to create HEC client")?;

    let status = tokio::select! {
        res = client.hec_check_acks(&hec_url, &hec_token, &ack_ids) => res?,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    // Format output
    let format = OutputFormat::from_str(output_format)?;
    let output = if format == OutputFormat::Table {
        format_hec_ack_status_table(&status)?
    } else {
        let formatter = get_formatter(format);
        formatter.format_hec_ack_status(&status)?
    };

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

// Table formatting functions for HEC responses
fn format_hec_response_table(response: &splunk_client::HecResponse) -> Result<String> {
    let mut output = String::new();
    output.push_str("HEC Event Submission Result\n");
    output.push_str("===========================\n\n");
    output.push_str(&format!("Code:    {}\n", response.code));
    output.push_str(&format!(
        "Status:  {}\n",
        if response.is_success() {
            "Success"
        } else {
            "Failed"
        }
    ));
    output.push_str(&format!("Message: {}\n", response.text));
    if let Some(ack_id) = response.ack_id {
        output.push_str(&format!("Ack ID:  {}\n", ack_id));
    }
    Ok(output)
}

fn format_hec_batch_response_table(response: &splunk_client::HecBatchResponse) -> Result<String> {
    let mut output = String::new();
    output.push_str("HEC Batch Submission Result\n");
    output.push_str("===========================\n\n");
    output.push_str(&format!("Code:    {}\n", response.code));
    output.push_str(&format!(
        "Status:  {}\n",
        if response.is_success() {
            "Success"
        } else {
            "Failed"
        }
    ));
    output.push_str(&format!("Message: {}\n", response.text));
    if let Some(ref ack_ids) = response.ack_ids {
        output.push_str(&format!(
            "Ack IDs: {}\n",
            ack_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Ok(output)
}

fn format_hec_health_table(health: &splunk_client::HecHealth) -> Result<String> {
    let mut output = String::new();
    output.push_str("HEC Health Status\n");
    output.push_str("=================\n\n");
    output.push_str(&format!(
        "Status:      {}\n",
        if health.is_healthy() {
            "Healthy"
        } else {
            "Unhealthy"
        }
    ));
    output.push_str(&format!("HTTP Code:   {}\n", health.code));
    output.push_str(&format!("Message:     {}\n", health.text));
    Ok(output)
}

fn format_hec_ack_status_table(status: &splunk_client::HecAckStatus) -> Result<String> {
    let mut output = String::new();
    output.push_str("HEC Acknowledgment Status\n");
    output.push_str("=========================\n\n");
    output.push_str(&format!("All Indexed: {}\n\n", status.all_indexed()));

    if status.acks.is_empty() {
        output.push_str("No acknowledgment statuses found.\n");
    } else {
        output.push_str("Acknowledgment ID | Status\n");
        output.push_str("------------------ | ------\n");
        let mut ids: Vec<_> = status.acks.keys().collect();
        ids.sort();
        for id in ids {
            let indexed = status.acks.get(id).unwrap_or(&false);
            output.push_str(&format!(
                "{:18} | {}\n",
                id,
                if *indexed { "Indexed" } else { "Pending" }
            ));
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_hec_response_table_success() {
        let response = splunk_client::HecResponse {
            code: 0,
            text: "Success".to_string(),
            ack_id: Some(123),
        };

        let output = format_hec_response_table(&response).unwrap();
        assert!(output.contains("Success"));
        assert!(output.contains("0"));
        assert!(output.contains("123"));
    }

    #[test]
    fn test_format_hec_response_table_error() {
        let response = splunk_client::HecResponse {
            code: 2,
            text: "Invalid token".to_string(),
            ack_id: None,
        };

        let output = format_hec_response_table(&response).unwrap();
        assert!(output.contains("Failed"));
        assert!(output.contains("2"));
        assert!(!output.contains("Ack ID"));
    }

    #[test]
    fn test_format_hec_health_table() {
        let health = splunk_client::HecHealth {
            text: "HEC is healthy".to_string(),
            code: 200,
        };

        let output = format_hec_health_table(&health).unwrap();
        assert!(output.contains("Healthy"));
        assert!(output.contains("200"));
    }
}
