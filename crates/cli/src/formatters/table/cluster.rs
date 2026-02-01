//! Cluster table formatter.
//!
//! Responsibilities:
//! - Format cluster information as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.
//! - Pagination (handled in mod.rs).

use crate::formatters::{
    ClusterInfoOutput, ClusterManagementOutput, ClusterPeerOutput, Pagination,
};
use anyhow::Result;

/// Format cluster info as a formatted text block.
pub fn format_cluster_info(cluster_info: &ClusterInfoOutput, detailed: bool) -> Result<String> {
    let mut output = format!(
        "Cluster Information:\n\
         ID: {}\n\
         Label: {}\n\
         Mode: {}\n\
         Manager URI: {}\n\
         Replication Factor: {}\n\
         Search Factor: {}\n\
         Status: {}\n",
        cluster_info.id,
        cluster_info.label.as_deref().unwrap_or("N/A"),
        cluster_info.mode,
        cluster_info.manager_uri.as_deref().unwrap_or("N/A"),
        cluster_info
            .replication_factor
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        cluster_info
            .search_factor
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        cluster_info.status.as_deref().unwrap_or("N/A")
    );

    if detailed && let Some(peers) = &cluster_info.peers {
        output.push_str(&format!("\nCluster Peers ({})\n", peers.len()));
        for peer in peers {
            output.push_str(&format!(
                "\n  Host: {}:{}\n\
                    ID: {}\n\
                    Status: {}\n\
                    State: {}\n",
                peer.host, peer.port, peer.id, peer.status, peer.peer_state
            ));
            if let Some(label) = &peer.label {
                output.push_str(&format!("    Label: {}\n", label));
            }
            if let Some(site) = &peer.site {
                output.push_str(&format!("    Site: {}\n", site));
            }
            if peer.is_captain {
                output.push_str("    Captain: Yes\n");
            }
        }
    }

    Ok(output)
}

/// Format cluster peers as a table.
pub fn format_cluster_peers(
    peers: &[ClusterPeerOutput],
    pagination: &Pagination,
) -> Result<String> {
    let mut output = String::from("Cluster Peers:\n\n");

    if peers.is_empty() {
        output.push_str("No peers found.\n");
        return Ok(output);
    }

    // Header
    output.push_str("Host\t\tStatus\tState\t\tSite\tCaptain\n");
    output.push_str("----\t\t------\t-----\t\t----\t-------\n");

    for peer in peers {
        let captain_marker = if peer.is_captain { "Yes" } else { "" };
        output.push_str(&format!(
            "{}:{}\t{}\t{}\t{}\t{}\n",
            peer.host,
            peer.port,
            peer.status,
            peer.peer_state,
            peer.site.as_deref().unwrap_or("-"),
            captain_marker
        ));
    }

    output.push_str(&format!(
        "\nShowing {} of {} peers (offset: {})\n",
        peers.len(),
        pagination.total.unwrap_or(peers.len()),
        pagination.offset
    ));

    Ok(output)
}

/// Format cluster management operation result.
pub fn format_cluster_management(output: &ClusterManagementOutput) -> Result<String> {
    let status = if output.success { "SUCCESS" } else { "FAILED" };
    Ok(format!(
        "Operation: {}\nTarget: {}\nStatus: {}\nMessage: {}\n",
        output.operation, output.target, status, output.message
    ))
}
