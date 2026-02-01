//! HEC (HTTP Event Collector) CSV formatters.
//!
//! This module provides CSV formatting for HEC responses.

use anyhow::Result;

/// Format a single HEC response as CSV.
pub fn format_hec_response(response: &splunk_client::HecResponse) -> Result<String> {
    let mut output = String::new();
    output.push_str("code,text,ack_id\n");
    output.push_str(&format!(
        "{},\"{}\",{}\n",
        response.code,
        response.text.replace('"', "\"\""),
        response.ack_id.map_or(String::new(), |id| id.to_string())
    ));
    Ok(output)
}

/// Format a HEC batch response as CSV.
pub fn format_hec_batch_response(response: &splunk_client::HecBatchResponse) -> Result<String> {
    let mut output = String::new();
    output.push_str("code,text,ack_ids\n");
    let ack_ids_str = response
        .ack_ids
        .as_ref()
        .map(|ids| {
            ids.iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(";")
        })
        .unwrap_or_default();
    output.push_str(&format!(
        "{},\"{}\",\"{}\"\n",
        response.code,
        response.text.replace('"', "\"\""),
        ack_ids_str
    ));
    Ok(output)
}

/// Format HEC health status as CSV.
pub fn format_hec_health(health: &splunk_client::HecHealth) -> Result<String> {
    let mut output = String::new();
    output.push_str("code,text,healthy\n");
    output.push_str(&format!(
        "{},\"{}\",{}\n",
        health.code,
        health.text.replace('"', "\"\""),
        health.is_healthy()
    ));
    Ok(output)
}

/// Format HEC acknowledgment status as CSV.
pub fn format_hec_ack_status(status: &splunk_client::HecAckStatus) -> Result<String> {
    let mut output = String::new();
    output.push_str("ack_id,indexed\n");

    let mut ids: Vec<_> = status.acks.keys().collect();
    ids.sort();

    for id in ids {
        let indexed = status.acks.get(id).unwrap_or(&false);
        output.push_str(&format!("{},{}\n", id, indexed));
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_hec_response_csv() {
        let response = splunk_client::HecResponse {
            code: 0,
            text: "Success".to_string(),
            ack_id: Some(123),
        };

        let output = format_hec_response(&response).unwrap();
        assert!(output.contains("code,text,ack_id"));
        assert!(output.contains("0,\"Success\",123"));
    }

    #[test]
    fn test_format_hec_health_csv() {
        let health = splunk_client::HecHealth {
            text: "HEC is healthy".to_string(),
            code: 200,
        };

        let output = format_hec_health(&health).unwrap();
        assert!(output.contains("code,text,healthy"));
        assert!(output.contains("200,\"HEC is healthy\",true"));
    }

    #[test]
    fn test_format_hec_ack_status_csv() {
        use std::collections::HashMap;

        let mut acks = HashMap::new();
        acks.insert(1, true);
        acks.insert(2, false);

        let status = splunk_client::HecAckStatus { acks };

        let output = format_hec_ack_status(&status).unwrap();
        assert!(output.contains("ack_id,indexed"));
        assert!(output.contains("1,true"));
        assert!(output.contains("2,false"));
    }
}
