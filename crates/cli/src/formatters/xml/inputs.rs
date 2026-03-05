//! Inputs XML formatter.
//!
//! Responsibilities:
//! - Format data input lists as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_client::models::Input;

/// Format inputs as XML.
pub fn format_inputs(inputs: &[Input], _detailed: bool) -> Result<String> {
    let mut output = String::new();
    output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    output.push_str("<inputs>\n");

    for input in inputs {
        output.push_str("  <input>\n");
        output.push_str(&format!("    <name>{}</name>\n", escape_xml(&input.name)));
        output.push_str(&format!(
            "    <type>{}</type>\n",
            escape_xml(&input.input_type.to_string())
        ));
        output.push_str(&format!("    <disabled>{}</disabled>\n", input.disabled));

        if let Some(ref host) = input.host {
            output.push_str(&format!("    <host>{}</host>\n", escape_xml(host)));
        }
        if let Some(ref source) = input.source {
            output.push_str(&format!("    <source>{}</source>\n", escape_xml(source)));
        }
        if let Some(ref sourcetype) = input.sourcetype {
            output.push_str(&format!(
                "    <sourcetype>{}</sourcetype>\n",
                escape_xml(sourcetype)
            ));
        }
        if let Some(ref port) = input.port {
            output.push_str(&format!("    <port>{}</port>\n", escape_xml(port)));
        }
        if let Some(ref path) = input.path {
            output.push_str(&format!("    <path>{}</path>\n", escape_xml(path)));
        }
        if let Some(ref connection_host) = input.connection_host {
            output.push_str(&format!(
                "    <connection_host>{}</connection_host>\n",
                escape_xml(connection_host)
            ));
        }

        output.push_str("  </input>\n");
    }

    output.push_str("</inputs>\n");
    Ok(output)
}
