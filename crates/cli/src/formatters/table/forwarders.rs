//! Forwarders table formatter.
//!
//! Responsibilities:
//! - Format forwarder lists as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.
//! - Pagination (handled in imp.rs).

use anyhow::Result;
use splunk_client::Forwarder;

/// Format forwarders as a tab-separated table.
pub fn format_forwarders(forwarders: &[Forwarder], detailed: bool) -> Result<String> {
    let mut output = String::new();

    if forwarders.is_empty() {
        return Ok("No forwarders found.".to_string());
    }

    // Header
    if detailed {
        output.push_str(
            "Name\tHostname\tClient Name\tIP Address\tVersion\tLast Phone\tUtsname\tServer Classes\n",
        );
    } else {
        output.push_str("Name\tHostname\tIP Address\tVersion\tLast Phone\n");
    }

    for forwarder in forwarders {
        let name = forwarder.name.clone();
        let hostname = forwarder.hostname.as_deref().unwrap_or("N/A");
        let ip = forwarder.ip_address.as_deref().unwrap_or("N/A");
        let version = forwarder.version.as_deref().unwrap_or("N/A");
        let last_phone = forwarder.last_phone.as_deref().unwrap_or("N/A");

        if detailed {
            let client_name = forwarder.client_name.as_deref().unwrap_or("N/A");
            let utsname = forwarder.utsname.as_deref().unwrap_or("N/A");
            let server_classes = forwarder
                .server_classes
                .as_ref()
                .map(|sc| sc.join(", "))
                .unwrap_or_else(|| "N/A".to_string());
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                name, hostname, client_name, ip, version, last_phone, utsname, server_classes
            ));
        } else {
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                name, hostname, ip, version, last_phone
            ));
        }
    }

    Ok(output)
}
