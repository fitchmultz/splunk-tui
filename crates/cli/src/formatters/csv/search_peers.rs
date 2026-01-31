//! Search Peers CSV formatter.
//!
//! Responsibilities:
//! - Format search peers as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_client::models::SearchPeer;

/// Format search peers as CSV.
pub fn format_search_peers(peers: &[SearchPeer], detailed: bool) -> Result<String> {
    let mut output = String::new();

    // Header
    let mut headers = vec!["name", "host", "port", "status", "version"];
    if detailed {
        headers.extend(vec!["guid", "last_connected", "disabled"]);
    }
    output.push_str(&build_csv_header(&headers));

    for peer in peers {
        let mut values = vec![
            escape_csv(&peer.name),
            escape_csv(&peer.host),
            escape_csv(&peer.port.to_string()),
            escape_csv(&peer.status),
            format_opt_str(peer.version.as_deref(), ""),
        ];

        if detailed {
            let disabled = peer
                .disabled
                .map(|d| if d { "true" } else { "false" })
                .unwrap_or("");
            values.extend(vec![
                format_opt_str(peer.guid.as_deref(), ""),
                format_opt_str(peer.last_connected.as_deref(), ""),
                escape_csv(disabled),
            ]);
        }

        output.push_str(&build_csv_row(&values));
    }

    Ok(output)
}
