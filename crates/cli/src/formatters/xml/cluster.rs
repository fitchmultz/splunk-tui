//! Cluster XML formatter.
//!
//! Responsibilities:
//! - Format cluster information as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::ClusterInfoOutput;
use crate::formatters::common::escape_xml;
use anyhow::Result;

/// Format cluster info as XML.
pub fn format_cluster_info(cluster_info: &ClusterInfoOutput, detailed: bool) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<cluster>\n");
    xml.push_str(&format!("  <id>{}</id>\n", escape_xml(&cluster_info.id)));
    if let Some(label) = &cluster_info.label {
        xml.push_str(&format!("  <label>{}</label>\n", escape_xml(label)));
    }
    xml.push_str(&format!(
        "  <mode>{}</mode>\n",
        escape_xml(&cluster_info.mode)
    ));
    if let Some(manager_uri) = &cluster_info.manager_uri {
        xml.push_str(&format!(
            "  <managerUri>{}</managerUri>\n",
            escape_xml(manager_uri)
        ));
    }
    if let Some(replication_factor) = cluster_info.replication_factor {
        xml.push_str(&format!(
            "  <replicationFactor>{}</replicationFactor>\n",
            replication_factor
        ));
    }
    if let Some(search_factor) = cluster_info.search_factor {
        xml.push_str(&format!(
            "  <searchFactor>{}</searchFactor>\n",
            search_factor
        ));
    }
    if let Some(status) = &cluster_info.status {
        xml.push_str(&format!("  <status>{}</status>\n", escape_xml(status)));
    }

    // Add peers if detailed
    if detailed && let Some(peers) = &cluster_info.peers {
        xml.push_str("  <peers>\n");
        for peer in peers {
            xml.push_str("    <peer>\n");
            xml.push_str(&format!("      <host>{}</host>\n", escape_xml(&peer.host)));
            xml.push_str(&format!("      <port>{}</port>\n", peer.port));
            xml.push_str(&format!("      <id>{}</id>\n", escape_xml(&peer.id)));
            xml.push_str(&format!(
                "      <status>{}</status>\n",
                escape_xml(&peer.status)
            ));
            xml.push_str(&format!(
                "      <peerState>{}</peerState>\n",
                escape_xml(&peer.peer_state)
            ));
            if let Some(label) = &peer.label {
                xml.push_str(&format!("      <label>{}</label>\n", escape_xml(label)));
            }
            if let Some(site) = &peer.site {
                xml.push_str(&format!("      <site>{}</site>\n", escape_xml(site)));
            }
            xml.push_str(&format!(
                "      <isCaptain>{}</isCaptain>\n",
                peer.is_captain
            ));
            xml.push_str("    </peer>\n");
        }
        xml.push_str("  </peers>\n");
    }

    xml.push_str("</cluster>");
    Ok(xml)
}
