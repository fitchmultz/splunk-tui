//! Indexes XML formatter.
//!
//! Responsibilities:
//! - Format index lists as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_client::Index;

/// Format indexes as XML.
pub fn format_indexes(indexes: &[Index], detailed: bool) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<indexes>\n");

    for index in indexes {
        xml.push_str("  <index>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&index.name)));
        xml.push_str(&format!(
            "    <sizeMB>{}</sizeMB>\n",
            index.current_db_size_mb
        ));
        xml.push_str(&format!(
            "    <events>{}</events>\n",
            index.total_event_count
        ));
        if let Some(max_size) = index.max_total_data_size_mb {
            xml.push_str(&format!("    <maxSizeMB>{}</maxSizeMB>\n", max_size));
        }
        // When detailed, include additional path and retention fields
        if detailed {
            if let Some(frozen_time) = index.frozen_time_period_in_secs {
                xml.push_str(&format!(
                    "    <retentionSecs>{}</retentionSecs>\n",
                    frozen_time
                ));
            }
            if let Some(home_path) = &index.home_path {
                xml.push_str(&format!(
                    "    <homePath>{}</homePath>\n",
                    escape_xml(home_path)
                ));
            }
            if let Some(cold_path) = &index.cold_db_path {
                xml.push_str(&format!(
                    "    <coldPath>{}</coldPath>\n",
                    escape_xml(cold_path)
                ));
            }
            if let Some(thawed_path) = &index.thawed_path {
                xml.push_str(&format!(
                    "    <thawedPath>{}</thawedPath>\n",
                    escape_xml(thawed_path)
                ));
            }
        }
        xml.push_str("  </index>\n");
    }

    xml.push_str("</indexes>");
    Ok(xml)
}
