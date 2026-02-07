//! Cluster command implementation.
//!
//! Responsibilities:
//! - Fetch and display cluster information and peers
//! - Manage cluster maintenance mode
//! - Rebalance cluster primaries
//! - Decommission and remove cluster peers
//!
//! Does NOT handle:
//! - Low-level cluster API calls (handled by client crate)
//! - Output formatting details (see formatters module)
//!
//! Invariants:
//! - Cluster operations require appropriate administrative privileges
//! - Peer decommissioning is irreversible and requires confirmation

use std::path::PathBuf;

use anyhow::{Context, Result};
use clap::Subcommand;
use tracing::{info, warn};

use crate::cancellation::Cancelled;
use crate::formatters::{
    ClusterInfoOutput, ClusterManagementOutput, ClusterPeerOutput, OutputFormat, Pagination,
    TableFormatter, get_formatter, write_to_file,
};

/// Cluster management subcommands.
#[derive(Debug, Subcommand)]
pub enum ClusterCommand {
    /// Show cluster status and information (default)
    #[command(alias = "info")]
    Show {
        /// Show detailed cluster information with peers
        #[arg(short, long)]
        detailed: bool,
        /// Offset into the cluster peer list (zero-based). Only applies with --detailed.
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Number of peers per page. Only applies with --detailed.
        #[arg(short, long, default_value = "50", visible_alias = "page-size")]
        count: usize,
    },

    /// Show cluster peers
    Peers {
        /// Offset into the cluster peer list (zero-based)
        #[arg(long, default_value = "0")]
        offset: usize,
        /// Number of peers per page
        #[arg(short, long, default_value = "50", visible_alias = "page-size")]
        count: usize,
    },

    /// Manage cluster maintenance mode
    #[command(subcommand)]
    Maintenance(MaintenanceCommand),

    /// Rebalance primary buckets across peers
    Rebalance,

    /// Manage cluster peers
    #[command(subcommand)]
    PeersManage(PeersCommand),
}

/// Maintenance mode subcommands.
#[derive(Debug, Subcommand)]
pub enum MaintenanceCommand {
    /// Enable maintenance mode
    Enable,
    /// Disable maintenance mode
    Disable,
    /// Show current maintenance mode status
    Status,
}

/// Peer management subcommands.
#[derive(Debug, Subcommand)]
pub enum PeersCommand {
    /// Decommission a peer (graceful shutdown)
    Decommission {
        /// Peer name or GUID to decommission
        #[arg(value_name = "PEER")]
        peer: String,
    },
    /// Remove a peer from the cluster
    Remove {
        /// Peer GUID to remove (must be in Down or GracefulShutdown status)
        #[arg(value_name = "PEER_GUID")]
        peer_guid: String,
        /// Force removal without confirmation
        #[arg(short, long)]
        force: bool,
    },
}

/// Run the cluster command.
pub async fn run(
    config: splunk_config::Config,
    command: ClusterCommand,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    match command {
        ClusterCommand::Show {
            detailed,
            offset,
            count,
        } => {
            run_show(
                config,
                detailed,
                offset,
                count,
                output_format,
                output_file,
                cancel,
            )
            .await
        }
        ClusterCommand::Peers { offset, count } => {
            run_peers(config, offset, count, output_format, output_file, cancel).await
        }
        ClusterCommand::Maintenance(maintenance_cmd) => {
            run_maintenance(config, maintenance_cmd, output_format, output_file, cancel).await
        }
        ClusterCommand::Rebalance => {
            run_rebalance(config, output_format, output_file, cancel).await
        }
        ClusterCommand::PeersManage(peers_cmd) => {
            run_peers_manage(config, peers_cmd, output_format, output_file, cancel).await
        }
    }
}

/// Fetch a page of cluster peers with pagination.
async fn fetch_cluster_peers_page(
    client: &mut splunk_client::SplunkClient,
    offset: usize,
    count: usize,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<(Option<Vec<ClusterPeerOutput>>, Option<Pagination>)> {
    let peers_result = tokio::select! {
        res = client.get_cluster_peers() => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    };

    match peers_result {
        Ok(peers) => {
            let total = peers.len();
            let page: Vec<_> = peers
                .into_iter()
                .skip(offset)
                .take(count)
                .map(ClusterPeerOutput::from)
                .collect();

            Ok((
                Some(page),
                Some(Pagination {
                    offset,
                    page_size: count,
                    total: Some(total),
                }),
            ))
        }
        Err(e) => {
            warn!("Failed to fetch cluster peers: {}", e);
            Ok((None, None))
        }
    }
}

/// Render cluster info output to file or stdout.
async fn render_cluster_output(
    info: &ClusterInfoOutput,
    detailed: bool,
    pagination: Option<Pagination>,
    format: OutputFormat,
    output_file: Option<&PathBuf>,
) -> Result<()> {
    let output = if format == OutputFormat::Table {
        let formatter = TableFormatter;
        formatter.format_cluster_info_paginated(info, detailed, pagination)?
    } else {
        let formatter = get_formatter(format);
        formatter.format_cluster_info(info, detailed)?
    };

    if let Some(path) = output_file {
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

async fn run_show(
    config: splunk_config::Config,
    detailed: bool,
    offset: usize,
    count: usize,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!(
        "Fetching cluster information (detailed: {}, offset: {}, count: {})",
        detailed, offset, count
    );

    // Validate pagination inputs (client-side pagination must be safe)
    if count == 0 {
        anyhow::bail!("The --count value must be greater than 0");
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
                fetch_cluster_peers_page(&mut client, offset, count, cancel).await?
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
                maintenance_mode: cluster_info.maintenance_mode,
                peers: peers_output,
            };

            let format = OutputFormat::from_str(output_format)?;
            render_cluster_output(
                &info,
                detailed,
                peers_pagination,
                format,
                output_file.as_ref(),
            )
            .await?;
        }
        Err(e) => {
            // Not all Splunk instances are clustered
            warn!("Failed to fetch cluster info: {}", e);
            println!("Note: This Splunk instance may not be configured as a cluster.");
        }
    }

    Ok(())
}

async fn run_peers(
    config: splunk_config::Config,
    offset: usize,
    count: usize,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!(
        "Fetching cluster peers (offset: {}, count: {})",
        offset, count
    );

    if count == 0 {
        anyhow::bail!("The --count value must be greater than 0");
    }

    let client = crate::commands::build_client_from_config(&config)?;

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
                .take(count)
                .map(ClusterPeerOutput::from)
                .collect();

            let pagination = Pagination {
                offset,
                page_size: count,
                total: Some(total),
            };

            let format = OutputFormat::from_str(output_format)?;
            let formatter = get_formatter(format);
            let output = formatter.format_cluster_peers(&page, &pagination)?;

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
            warn!("Failed to fetch cluster peers: {}", e);
            println!("Note: This Splunk instance may not be configured as a cluster.");
        }
    }

    Ok(())
}

async fn run_maintenance(
    config: splunk_config::Config,
    command: MaintenanceCommand,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    let client = crate::commands::build_client_from_config(&config)?;

    match command {
        MaintenanceCommand::Enable => {
            info!("Enabling maintenance mode");
            let result = tokio::select! {
                res = client.enable_maintenance_mode() => res,
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            }?;
            handle_management_result(
                result,
                "enable maintenance mode",
                output_format,
                output_file,
            )
            .await?;
        }
        MaintenanceCommand::Disable => {
            info!("Disabling maintenance mode");
            let result = tokio::select! {
                res = client.disable_maintenance_mode() => res,
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            }?;
            handle_management_result(
                result,
                "disable maintenance mode",
                output_format,
                output_file,
            )
            .await?;
        }
        MaintenanceCommand::Status => {
            info!("Fetching maintenance mode status");
            let info = tokio::select! {
                res = client.get_cluster_info() => res,
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            }?;
            let status = if info.maintenance_mode == Some(true) {
                "enabled"
            } else {
                "disabled"
            };
            println!("Maintenance mode: {}", status);
        }
    }

    Ok(())
}

async fn run_rebalance(
    config: splunk_config::Config,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    info!("Rebalancing cluster primaries");

    let client = crate::commands::build_client_from_config(&config)?;

    let result = tokio::select! {
        res = client.rebalance_cluster() => res,
        _ = cancel.cancelled() => return Err(Cancelled.into()),
    }?;

    handle_management_result(result, "rebalance cluster", output_format, output_file).await?;

    Ok(())
}

async fn run_peers_manage(
    config: splunk_config::Config,
    command: PeersCommand,
    output_format: &str,
    output_file: Option<PathBuf>,
    cancel: &crate::cancellation::CancellationToken,
) -> Result<()> {
    let client = crate::commands::build_client_from_config(&config)?;

    match command {
        PeersCommand::Decommission { peer } => {
            info!("Decommissioning peer: {}", peer);
            let peer_result = tokio::select! {
                res = client.decommission_peer(&peer) => res,
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            }?;

            let result = ClusterManagementOutput {
                operation: "decommission".to_string(),
                target: peer,
                success: true,
                message: format!(
                    "Peer {} decommission initiated. Current status: {}, state: {}",
                    peer_result.host, peer_result.status, peer_result.peer_state
                ),
            };

            handle_management_output(result, output_format, output_file).await?;
        }
        PeersCommand::Remove { peer_guid, force } => {
            if !force && !crate::interactive::confirm_delete(&peer_guid, "cluster peer")? {
                return Ok(());
            }

            info!("Removing peer: {}", peer_guid);
            let peer_guids = vec![peer_guid.clone()];
            let result = tokio::select! {
                res = client.remove_peers(&peer_guids) => res,
                _ = cancel.cancelled() => return Err(Cancelled.into()),
            }?;

            handle_management_result(
                result,
                &format!("remove peer {}", peer_guid),
                output_format,
                output_file,
            )
            .await?;
        }
    }

    Ok(())
}

async fn handle_management_result(
    result: splunk_client::ClusterManagementResponse,
    operation: &str,
    output_format: &str,
    output_file: Option<PathBuf>,
) -> Result<()> {
    let output = ClusterManagementOutput {
        operation: operation.to_string(),
        target: "cluster".to_string(),
        success: result.success,
        message: result
            .message
            .unwrap_or_else(|| format!("{} completed", operation)),
    };

    handle_management_output(output, output_format, output_file).await
}

async fn handle_management_output(
    output: ClusterManagementOutput,
    output_format: &str,
    output_file: Option<PathBuf>,
) -> Result<()> {
    let format = OutputFormat::from_str(output_format)?;
    let formatter = get_formatter(format);
    let formatted = formatter.format_cluster_management(&output)?;

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
