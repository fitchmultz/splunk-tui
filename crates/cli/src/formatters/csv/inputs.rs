//! Inputs CSV formatter.
//!
//! Responsibilities:
//! - Format data inputs as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_client::models::Input;

/// Format inputs as CSV.
pub fn format_inputs(inputs: &[Input], detailed: bool) -> Result<String> {
    let mut output = String::new();

    // Header
    let mut headers = vec![
        "name",
        "input_type",
        "host",
        "source",
        "sourcetype",
        "disabled",
    ];
    if detailed {
        headers.extend(vec![
            "port",
            "path",
            "connection_host",
            "blacklist",
            "whitelist",
            "recursive",
            "command",
            "interval",
        ]);
    }
    output.push_str(&build_csv_header(&headers));

    for input in inputs {
        let disabled = if input.disabled { "true" } else { "false" };
        let mut values = vec![
            escape_csv(&input.name),
            escape_csv(&input.input_type),
            format_opt_str(input.host.as_deref(), ""),
            format_opt_str(input.source.as_deref(), ""),
            format_opt_str(input.sourcetype.as_deref(), ""),
            escape_csv(disabled),
        ];

        if detailed {
            let recursive = input
                .recursive
                .map(|r| if r { "true" } else { "false" })
                .unwrap_or("");
            values.extend(vec![
                format_opt_str(input.port.as_deref(), ""),
                format_opt_str(input.path.as_deref(), ""),
                format_opt_str(input.connection_host.as_deref(), ""),
                format_opt_str(input.blacklist.as_deref(), ""),
                format_opt_str(input.whitelist.as_deref(), ""),
                escape_csv(recursive),
                format_opt_str(input.command.as_deref(), ""),
                format_opt_str(input.interval.as_deref(), ""),
            ]);
        }

        output.push_str(&build_csv_row(&values));
    }

    Ok(output)
}
