//! Search peers table formatter.
//!
//! Responsibilities:
//! - Format search peer lists as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.
//! - Pagination (handled in imp.rs).

use anyhow::Result;
use splunk_client::models::SearchPeer;

/// Format search peers as a tab-separated table.
pub fn format_search_peers(peers: &[SearchPeer], detailed: bool) -> Result<String> {
    let mut output = String::new();

    if peers.is_empty() {
        return Ok("No search peers found.".to_string());
    }

    // Header
    if detailed {
        output.push_str("Name\tHost\tPort\tStatus\tVersion\tGUID\tLast Connected\tDisabled\n");
    } else {
        output.push_str("Name\tHost\tPort\tStatus\tVersion\n");
    }

    for peer in peers {
        let name = peer.name.clone();
        let host = peer.host.clone();
        let port = peer.port.to_string();
        let status = peer.status;
        let version = peer.version.as_deref().unwrap_or("N/A");

        if detailed {
            let guid = peer.guid.as_deref().unwrap_or("N/A");
            let last_connected = peer.last_connected.as_deref().unwrap_or("N/A");
            let disabled = peer
                .disabled
                .map(|d| if d { "Yes" } else { "No" })
                .unwrap_or("N/A");
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                name, host, port, status, version, guid, last_connected, disabled
            ));
        } else {
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                name, host, port, status, version
            ));
        }
    }

    Ok(output)
}
