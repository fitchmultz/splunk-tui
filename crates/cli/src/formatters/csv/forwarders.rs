//! Forwarders CSV formatter.
//!
//! Responsibilities:
//! - Format forwarder lists as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_client::Forwarder;

/// Format forwarders as CSV.
pub fn format_forwarders(forwarders: &[Forwarder], detailed: bool) -> Result<String> {
    let mut output = String::new();

    // Header
    let mut headers = vec![
        "name",
        "hostname",
        "client_name",
        "ip_address",
        "version",
        "last_phone",
    ];
    if detailed {
        headers.extend(vec!["utsname", "repository_location", "server_classes"]);
    }
    output.push_str(&build_csv_header(&headers));

    for forwarder in forwarders {
        let mut values = vec![
            escape_csv(&forwarder.name),
            format_opt_str(forwarder.hostname.as_deref(), ""),
            format_opt_str(forwarder.client_name.as_deref(), ""),
            format_opt_str(forwarder.ip_address.as_deref(), ""),
            format_opt_str(forwarder.version.as_deref(), ""),
            format_opt_str(forwarder.last_phone.as_deref(), ""),
        ];

        if detailed {
            let server_classes = forwarder
                .server_classes
                .as_ref()
                .map(|sc| sc.join(";"))
                .unwrap_or_default();
            values.extend(vec![
                format_opt_str(forwarder.utsname.as_deref(), ""),
                format_opt_str(forwarder.repository_location.as_deref(), ""),
                escape_csv(&server_classes),
            ]);
        }

        output.push_str(&build_csv_row(&values));
    }

    Ok(output)
}
