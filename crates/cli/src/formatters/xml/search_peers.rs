//! Search peers XML formatter.
//!
//! Responsibilities:
//! - Format search peer lists as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_client::models::SearchPeer;

/// Format search peers as XML.
pub fn format_search_peers(peers: &[SearchPeer], detailed: bool) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<searchPeers>\n");

    for peer in peers {
        xml.push_str("  <searchPeer>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&peer.name)));
        xml.push_str(&format!("    <host>{}</host>\n", escape_xml(&peer.host)));
        xml.push_str(&format!("    <port>{}</port>\n", peer.port));
        xml.push_str(&format!(
            "    <status>{}</status>\n",
            escape_xml(&peer.status)
        ));

        if let Some(version) = &peer.version {
            xml.push_str(&format!("    <version>{}</version>\n", escape_xml(version)));
        }

        // When detailed, include additional fields
        if detailed {
            if let Some(guid) = &peer.guid {
                xml.push_str(&format!("    <guid>{}</guid>\n", escape_xml(guid)));
            }

            if let Some(last_connected) = &peer.last_connected {
                xml.push_str(&format!(
                    "    <lastConnected>{}</lastConnected>\n",
                    escape_xml(last_connected)
                ));
            }

            if let Some(disabled) = peer.disabled {
                xml.push_str(&format!(
                    "    <disabled>{}</disabled>\n",
                    if disabled { "true" } else { "false" }
                ));
            }
        }

        xml.push_str("  </searchPeer>\n");
    }

    xml.push_str("</searchPeers>");
    Ok(xml)
}
