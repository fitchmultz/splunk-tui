//! List-all XML formatter.
//!
//! Responsibilities:
//! - Format unified resource overview as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::commands::list_all::ListAllOutput;
use crate::formatters::common::escape_xml;
use anyhow::Result;

/// Format list-all output as XML.
#[allow(dead_code)]
pub fn format_list_all(output: &ListAllOutput) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<list_all>\n");
    xml.push_str(&format!(
        "  <timestamp>{}</timestamp>\n",
        escape_xml(&output.timestamp)
    ));
    xml.push_str("  <resources>\n");

    for resource in &output.resources {
        xml.push_str("    <resource>\n");
        xml.push_str(&format!(
            "      <type>{}</type>\n",
            escape_xml(&resource.resource_type)
        ));
        xml.push_str(&format!("      <count>{}</count>\n", resource.count));
        xml.push_str(&format!(
            "      <status>{}</status>\n",
            escape_xml(&resource.status)
        ));
        if let Some(error) = &resource.error {
            xml.push_str(&format!("      <error>{}</error>\n", escape_xml(error)));
        }
        xml.push_str("    </resource>\n");
    }

    xml.push_str("  </resources>\n");
    xml.push_str("</list_all>");
    Ok(xml)
}
