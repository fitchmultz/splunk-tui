//! Cluster command implementation.

use anyhow::{Context, Result};
use splunk_client::SplunkClient;
use tracing::{info, warn};

use crate::formatters::{
    ClusterInfoOutput, ClusterPeerOutput, OutputFormat, Pagination, TableFormatter, get_formatter,
    write_to_file,
};

pub async fn run(
    config: splunk_config::Config,
    detailed: bool,
    offset: usize,
    page_size: usize,
    output_format: &str,
    output_file: Option<std::path::PathBuf>,
    _cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!(
        "Fetching cluster information (detailed: {}, offset: {}, page_size: {})",
        detailed, offset, page_size
    );

    // Validate pagination inputs (client-side pagination must be safe)
    if page_size == 0 {
        anyhow::bail!("--page-size must be greater than 0");
    }

    let auth_strategy = crate::commands::convert_auth_strategy(&config.auth.strategy);

    let mut client = SplunkClient::builder()
        .base_url(config.connection.base_url)
        .auth_strategy(auth_strategy)
        .skip_verify(config.connection.skip_verify)
        .timeout(config.connection.timeout)
        .build()?;

    match client.get_cluster_info().await {
        Ok(cluster_info) => {
            // Fetch peers if detailed (fetch ALL once, then paginate locally)
            let (peers_output, peers_pagination) = if detailed {
                match client.get_cluster_peers().await {
                    Ok(peers) => {
                        let total = peers.len();

                        // Slice safely (empty slice if offset beyond end)
                        let page: Vec<_> = peers
                            .into_iter()
                            .skip(offset)
                            .take(page_size)
                            .map(ClusterPeerOutput::from)
                            .collect();

                        (
                            Some(page),
                            Some(Pagination {
                                offset,
                                page_size,
                                total: Some(total),
                            }),
                        )
                    }
                    Err(e) => {
                        warn!("Could not fetch cluster peers: {}", e);
                        (None, None)
                    }
                }
            } else {
                (None, None)
            };

            let info = ClusterInfoOutput {
                id: cluster_info.id,
                label: cluster_info.label,
                mode: cluster_info.mode,
                manager_uri: cluster_info.manager_uri,
                replication_factor: cluster_info.replication_factor,
                search_factor: cluster_info.search_factor,
                status: cluster_info.status,
                peers: peers_output,
            };

            // Parse output format
            let format = OutputFormat::from_str(output_format)?;

            // Table output gets pagination footer; machine-readable formats must not.
            if format == OutputFormat::Table {
                let formatter = TableFormatter;
                let output =
                    formatter.format_cluster_info_paginated(&info, detailed, peers_pagination)?;
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
                return Ok(());
            }

            let formatter = get_formatter(format);
            let output = formatter.format_cluster_info(&info, detailed)?;
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
        }
        Err(e) => {
            // Not all Splunk instances are clustered
            warn!("Could not fetch cluster info: {}", e);
            println!("Note: This Splunk instance may not be configured as a cluster.");
        }
    }

    Ok(())
}
