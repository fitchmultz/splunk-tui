//! HEC (HTTP Event Collector) XML formatters.
//!
//! This module provides XML formatting for HEC responses.

use anyhow::Result;

/// Format a single HEC response as XML.
pub fn format_hec_response(response: &splunk_client::HecResponse) -> Result<String> {
    let mut output = String::new();
    output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    output.push_str("<hec_response>\n");
    output.push_str(&format!("  <code>{}</code>\n", response.code));
    output.push_str(&format!("  <text>{}</text>\n", escape_xml(&response.text)));
    output.push_str(&format!("  <success>{}</success>\n", response.is_success()));
    if let Some(ack_id) = response.ack_id {
        output.push_str(&format!("  <ack_id>{}</ack_id>\n", ack_id));
    }
    output.push_str("</hec_response>\n");
    Ok(output)
}

/// Format a HEC batch response as XML.
pub fn format_hec_batch_response(response: &splunk_client::HecBatchResponse) -> Result<String> {
    let mut output = String::new();
    output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    output.push_str("<hec_batch_response>\n");
    output.push_str(&format!("  <code>{}</code>\n", response.code));
    output.push_str(&format!("  <text>{}</text>\n", escape_xml(&response.text)));
    output.push_str(&format!("  <success>{}</success>\n", response.is_success()));
    if let Some(ref ack_ids) = response.ack_ids {
        output.push_str("  <ack_ids>\n");
        for id in ack_ids {
            output.push_str(&format!("    <ack_id>{}</ack_id>\n", id));
        }
        output.push_str("  </ack_ids>\n");
    }
    output.push_str("</hec_batch_response>\n");
    Ok(output)
}

/// Format HEC health status as XML.
pub fn format_hec_health(health: &splunk_client::HecHealth) -> Result<String> {
    let mut output = String::new();
    output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    output.push_str("<hec_health>\n");
    output.push_str(&format!("  <code>{}</code>\n", health.code));
    output.push_str(&format!("  <text>{}</text>\n", escape_xml(&health.text)));
    output.push_str(&format!("  <healthy>{}</healthy>\n", health.is_healthy()));
    output.push_str("</hec_health>\n");
    Ok(output)
}

/// Format HEC acknowledgment status as XML.
pub fn format_hec_ack_status(status: &splunk_client::HecAckStatus) -> Result<String> {
    let mut output = String::new();
    output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
    output.push_str("<hec_ack_status>\n");
    output.push_str(&format!(
        "  <all_indexed>{}</all_indexed>\n",
        status.all_indexed()
    ));
    output.push_str("  <acks>\n");

    let mut ids: Vec<_> = status.acks.keys().collect();
    ids.sort();

    for id in ids {
        let indexed = status.acks.get(id).unwrap_or(&false);
        output.push_str("    <ack>\n");
        output.push_str(&format!("      <id>{}</id>\n", id));
        output.push_str(&format!("      <indexed>{}</indexed>\n", indexed));
        output.push_str("    </ack>\n");
    }

    output.push_str("  </acks>\n");
    output.push_str("</hec_ack_status>\n");
    Ok(output)
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_hec_response_xml() {
        let response = splunk_client::HecResponse {
            code: 0,
            text: "Success".to_string(),
            ack_id: Some(123),
        };

        let output = format_hec_response(&response).unwrap();
        assert!(output.contains("<?xml version=\"1.0\" encoding=\"UTF-8\"?>"));
        assert!(output.contains("<hec_response>"));
        assert!(output.contains("<code>0</code>"));
        assert!(output.contains("<text>Success</text>"));
        assert!(output.contains("<ack_id>123</ack_id>"));
    }

    #[test]
    fn test_format_hec_health_xml() {
        let health = splunk_client::HecHealth {
            text: "HEC is healthy".to_string(),
            code: 200,
        };

        let output = format_hec_health(&health).unwrap();
        assert!(output.contains("<hec_health>"));
        assert!(output.contains("<code>200</code>"));
        assert!(output.contains("<healthy>true</healthy>"));
    }

    #[test]
    fn test_format_hec_ack_status_xml() {
        use std::collections::HashMap;

        let mut acks = HashMap::new();
        acks.insert(1, true);
        acks.insert(2, false);

        let status = splunk_client::HecAckStatus { acks };

        let output = format_hec_ack_status(&status).unwrap();
        assert!(output.contains("<hec_ack_status>"));
        assert!(output.contains("<all_indexed>false</all_indexed>"));
        assert!(output.contains("<id>1</id>"));
        assert!(output.contains("<indexed>true</indexed>"));
    }
}
