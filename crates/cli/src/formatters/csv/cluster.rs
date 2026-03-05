//! Cluster CSV formatter.
//!
//! Responsibilities:
//! - Format cluster information as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use crate::formatters::{ClusterInfoOutput, ClusterManagementOutput};
use anyhow::Result;

/// Format cluster info as CSV.
pub fn format_cluster_info(cluster_info: &ClusterInfoOutput, detailed: bool) -> Result<String> {
    let mut output = String::new();

    // Cluster info header
    output.push_str(&build_csv_header(&[
        "Type",
        "ID",
        "Label",
        "Mode",
        "ManagerURI",
        "ReplicationFactor",
        "SearchFactor",
    ]));

    // Cluster info row
    let fields = vec![
        escape_csv("ClusterInfo"),
        escape_csv(&cluster_info.id),
        format_opt_str(cluster_info.label.as_deref(), "N/A"),
        escape_csv(&cluster_info.mode),
        format_opt_str(cluster_info.manager_uri.as_deref(), "N/A"),
        format_opt_str(
            cluster_info
                .replication_factor
                .map(|v| v.to_string())
                .as_deref(),
            "N/A",
        ),
        format_opt_str(
            cluster_info.search_factor.map(|v| v.to_string()).as_deref(),
            "N/A",
        ),
    ];
    output.push_str(&build_csv_row(&fields));

    // Peers rows (if detailed)
    if detailed && let Some(peers) = &cluster_info.peers {
        output.push('\n');
        output.push_str(&build_csv_header(&[
            "Type",
            "Address",
            "ID",
            "Status",
            "State",
            "Label",
            "Site",
            "IsCaptain",
        ]));

        for peer in peers {
            let peer_fields = vec![
                escape_csv("Peer"),
                escape_csv(&format!("{}:{}", peer.host, peer.port)),
                escape_csv(&peer.id),
                escape_csv(&peer.status),
                escape_csv(&peer.peer_state),
                format_opt_str(peer.label.as_deref(), "N/A"),
                format_opt_str(peer.site.as_deref(), "N/A"),
                escape_csv(if peer.is_captain { "Yes" } else { "No" }),
            ];
            output.push_str(&build_csv_row(&peer_fields));
        }
    }

    Ok(output)
}

/// Format cluster management operation result as CSV.
pub fn format_cluster_management(output: &ClusterManagementOutput) -> Result<String> {
    let mut result = String::new();
    result.push_str(&build_csv_header(&[
        "Operation",
        "Target",
        "Success",
        "Message",
    ]));
    let fields = vec![
        escape_csv(&output.operation),
        escape_csv(&output.target),
        escape_csv(&output.success.to_string()),
        escape_csv(&output.message),
    ];
    result.push_str(&build_csv_row(&fields));
    Ok(result)
}
