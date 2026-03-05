//! Logs XML formatter.
//!
//! Responsibilities:
//! - Format internal logs as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_client::models::LogEntry;

/// Format a single log entry as XML (internal helper).
fn format_log_entry(log: &LogEntry) -> String {
    let mut xml = String::new();
    xml.push_str("  <log>\n");
    xml.push_str(&format!("    <time>{}</time>\n", escape_xml(&log.time)));
    xml.push_str(&format!(
        "    <level>{}</level>\n",
        escape_xml(&log.level.to_string())
    ));
    xml.push_str(&format!(
        "    <component>{}</component>\n",
        escape_xml(&log.component)
    ));
    xml.push_str(&format!(
        "    <message>{}</message>\n",
        escape_xml(&log.message)
    ));
    xml.push_str("  </log>\n");
    xml
}

/// Format logs as XML.
pub fn format_logs(logs: &[LogEntry]) -> Result<String> {
    let mut xml = String::from(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    xml.push('\n');
    xml.push_str("<logs>");
    xml.push('\n');

    for log in logs {
        xml.push_str(&format_log_entry(log));
    }

    xml.push_str("</logs>");
    Ok(xml)
}

/// Format logs for streaming/tail mode.
///
/// Only emits the XML declaration and root element on the first call.
/// Note: The closing `</logs>` tag is not emitted in streaming mode
/// since the stream may be interrupted at any time.
pub fn format_logs_streaming(logs: &[LogEntry], is_first: bool) -> Result<String> {
    let mut xml = String::new();

    if logs.is_empty() {
        return Ok(xml);
    }

    if is_first {
        xml.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
        xml.push('\n');
        xml.push_str("<logs>");
        xml.push('\n');
    }

    for log in logs {
        xml.push_str(&format_log_entry(log));
    }

    Ok(xml)
}
