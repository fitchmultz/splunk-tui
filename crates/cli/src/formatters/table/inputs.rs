//! Inputs table formatter.
//!
//! Responsibilities:
//! - Format data input lists as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.
//! - Pagination (handled in imp.rs).

use anyhow::Result;
use splunk_client::models::Input;

/// Format inputs as a tab-separated table.
pub fn format_inputs(inputs: &[Input], detailed: bool) -> Result<String> {
    let mut output = String::new();

    if inputs.is_empty() {
        return Ok("No inputs found.".to_string());
    }

    // Header
    if detailed {
        output.push_str("Name\tType\tHost\tSource\tSourcetype\tDisabled\tPort\tPath\n");
    } else {
        output.push_str("Name\tType\tHost\tSource\tSourcetype\tDisabled\n");
    }

    for input in inputs {
        let name = input.name.clone();
        let input_type = input.input_type.to_string();
        let host = input.host.as_deref().unwrap_or("N/A");
        let source = input.source.as_deref().unwrap_or("N/A");
        let sourcetype = input.sourcetype.as_deref().unwrap_or("N/A");
        let disabled = if input.disabled { "Yes" } else { "No" };

        if detailed {
            let port = input.port.as_deref().unwrap_or("N/A");
            let path = input.path.as_deref().unwrap_or("N/A");
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                name, input_type, host, source, sourcetype, disabled, port, path
            ));
        } else {
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                name, input_type, host, source, sourcetype, disabled
            ));
        }
    }

    Ok(output)
}
