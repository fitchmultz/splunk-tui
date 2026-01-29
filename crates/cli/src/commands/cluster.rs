//! Cluster command implementation.

use anyhow::{Context, Result};
use tracing::{info, warn};

use crate::cancellation::Cancelled;
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
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!(
        "Fetching cluster information (detailed: {}, offset: {}, page_size: {})",
        detailed, offset, page_size
    );

    // Validate pagination inputs (client-side pagination must be safe)
    if page_size == 0 {
        anyhow::bail!("--page-size must be greater than 0");
    }

    let mut client = crate::commands::build_client_from_config(&config)?;

    let cluster_info = tokio::select! {
        res = client.get_cluster_info() => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    match cluster_info {
        Ok(cluster_info) => {
            // Fetch peers if detailed (fetch ALL once, then paginate locally)
            let (peers_output, peers_pagination) = if detailed {
                let peers_result = tokio::select! {
                    res = client.get_cluster_peers() => res,
                    _ = cancel.cancelled() => return Err(Cancelled.into()),
                };
                match peers_result {
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
