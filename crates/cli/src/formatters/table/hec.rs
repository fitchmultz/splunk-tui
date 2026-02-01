//! HEC (HTTP Event Collector) table formatters.
//!
//! This module provides table formatting for HEC responses.

use anyhow::Result;

/// Format a single HEC response as a table.
pub fn format_hec_response(response: &splunk_client::HecResponse) -> Result<String> {
    let mut output = String::new();
    output.push_str("HEC Event Submission Result\n");
    output.push_str("===========================\n\n");
    output.push_str(&format!("Code:    {}\n", response.code));
    output.push_str(&format!(
        "Status:  {}\n",
        if response.is_success() {
            "Success"
        } else {
            "Failed"
        }
    ));
    output.push_str(&format!("Message: {}\n", response.text));
    if let Some(ack_id) = response.ack_id {
        output.push_str(&format!("Ack ID:  {}\n", ack_id));
    }
    Ok(output)
}

/// Format a HEC batch response as a table.
pub fn format_hec_batch_response(response: &splunk_client::HecBatchResponse) -> Result<String> {
    let mut output = String::new();
    output.push_str("HEC Batch Submission Result\n");
    output.push_str("===========================\n\n");
    output.push_str(&format!("Code:    {}\n", response.code));
    output.push_str(&format!(
        "Status:  {}\n",
        if response.is_success() {
            "Success"
        } else {
            "Failed"
        }
    ));
    output.push_str(&format!("Message: {}\n", response.text));
    if let Some(ref ack_ids) = response.ack_ids {
        output.push_str(&format!(
            "Ack IDs: {}\n",
            ack_ids
                .iter()
                .map(|id| id.to_string())
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    Ok(output)
}

/// Format HEC health status as a table.
pub fn format_hec_health(health: &splunk_client::HecHealth) -> Result<String> {
    let mut output = String::new();
    output.push_str("HEC Health Status\n");
    output.push_str("=================\n\n");
    output.push_str(&format!(
        "Status:      {}\n",
        if health.is_healthy() {
            "Healthy"
        } else {
            "Unhealthy"
        }
    ));
    output.push_str(&format!("HTTP Code:   {}\n", health.code));
    output.push_str(&format!("Message:     {}\n", health.text));
    Ok(output)
}

/// Format HEC acknowledgment status as a table.
pub fn format_hec_ack_status(status: &splunk_client::HecAckStatus) -> Result<String> {
    let mut output = String::new();
    output.push_str("HEC Acknowledgment Status\n");
    output.push_str("=========================\n\n");
    output.push_str(&format!("All Indexed: {}\n\n", status.all_indexed()));

    if status.acks.is_empty() {
        output.push_str("No acknowledgment statuses found.\n");
    } else {
        output.push_str("Acknowledgment ID | Status\n");
        output.push_str("------------------ | ------\n");
        let mut ids: Vec<_> = status.acks.keys().collect();
        ids.sort();
        for id in ids {
            let indexed = status.acks.get(id).unwrap_or(&false);
            output.push_str(&format!(
                "{:18} | {}\n",
                id,
                if *indexed { "Indexed" } else { "Pending" }
            ));
        }
    }

    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_hec_response_success() {
        let response = splunk_client::HecResponse {
            code: 0,
            text: "Success".to_string(),
            ack_id: Some(123),
        };

        let output = format_hec_response(&response).unwrap();
        assert!(output.contains("Success"));
        assert!(output.contains("0"));
        assert!(output.contains("123"));
    }

    #[test]
    fn test_format_hec_response_error() {
        let response = splunk_client::HecResponse {
            code: 2,
            text: "Invalid token".to_string(),
            ack_id: None,
        };

        let output = format_hec_response(&response).unwrap();
        assert!(output.contains("Failed"));
        assert!(output.contains("2"));
        assert!(!output.contains("Ack ID"));
    }

    #[test]
    fn test_format_hec_health() {
        let health = splunk_client::HecHealth {
            text: "HEC is healthy".to_string(),
            code: 200,
        };

        let output = format_hec_health(&health).unwrap();
        assert!(output.contains("Healthy"));
        assert!(output.contains("200"));
    }

    #[test]
    fn test_format_hec_batch_response() {
        let response = splunk_client::HecBatchResponse {
            code: 0,
            text: "Success".to_string(),
            ack_ids: Some(vec![1, 2, 3]),
        };

        let output = format_hec_batch_response(&response).unwrap();
        assert!(output.contains("Success"));
        assert!(output.contains("1, 2, 3"));
    }
}
