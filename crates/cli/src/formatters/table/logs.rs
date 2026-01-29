//! Logs table formatter.
//!
//! Responsibilities:
//! - Format internal logs as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_client::models::LogEntry;

/// Format logs as a tab-separated table.
pub fn format_logs(logs: &[LogEntry]) -> Result<String> {
    let mut output = String::new();

    if logs.is_empty() {
        return Ok("No logs found.".to_string());
    }

    // Header
    output.push_str("Time\tLevel\tComponent\tMessage\n");

    for log in logs {
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\n",
            log.time, log.level, log.component, log.message
        ));
    }

    Ok(output)
}
