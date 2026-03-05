//! Logs CSV formatter.
//!
//! Responsibilities:
//! - Format log entries as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv};
use anyhow::Result;
use splunk_client::models::LogEntry;

/// Format logs as CSV.
pub fn format_logs(logs: &[LogEntry]) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "Time",
        "Level",
        "Component",
        "Message",
    ]));

    for log in logs {
        output.push_str(&build_csv_row(&[
            escape_csv(&log.time),
            escape_csv(&log.level.to_string()),
            escape_csv(&log.component),
            escape_csv(&log.message),
        ]));
    }

    Ok(output)
}

/// Format logs for streaming/tail mode.
pub fn format_logs_streaming(logs: &[LogEntry], is_first: bool) -> Result<String> {
    let mut output = String::new();

    if logs.is_empty() {
        return Ok(output);
    }

    if is_first {
        output.push_str(&build_csv_header(&[
            "Time",
            "Level",
            "Component",
            "Message",
        ]));
    }

    for log in logs {
        output.push_str(&build_csv_row(&[
            escape_csv(&log.time),
            escape_csv(&log.level.to_string()),
            escape_csv(&log.component),
            escape_csv(&log.message),
        ]));
    }

    Ok(output)
}
