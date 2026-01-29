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

/// Format logs as XML.
pub fn format_logs(logs: &[LogEntry]) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<logs>\n");

    for log in logs {
        xml.push_str("  <log>\n");
        xml.push_str(&format!("    <time>{}</time>\n", escape_xml(&log.time)));
        xml.push_str(&format!("    <level>{}</level>\n", escape_xml(&log.level)));
        xml.push_str(&format!(
            "    <component>{}</component>\n",
            escape_xml(&log.component)
        ));
        xml.push_str(&format!(
            "    <message>{}</message>\n",
            escape_xml(&log.message)
        ));
        xml.push_str("  </log>\n");
    }

    xml.push_str("</logs>");
    Ok(xml)
}
