//! List-all table formatter.
//!
//! Responsibilities:
//! - Format unified resource overview as formatted tables.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::commands::list_all::ListAllOutput;
use anyhow::Result;

/// Format list-all output as a formatted table.
#[allow(dead_code)]
pub fn format_list_all(output: &ListAllOutput) -> Result<String> {
    let mut out = String::new();

    if output.resources.is_empty() {
        return Ok("No resources found.".to_string());
    }

    out.push_str(&format!("Timestamp: {}\n", output.timestamp));
    out.push('\n');

    let header = format!(
        "{:<20} {:<10} {:<15} {}",
        "Resource Type", "Count", "Status", "Error"
    );
    out.push_str(&header);
    out.push('\n');

    let separator = format!("{:<20} {:<10} {:<15} {}", "====", "=====", "=====", "=====");
    out.push_str(&separator);
    out.push('\n');

    for resource in &output.resources {
        let error = resource.error.as_deref().unwrap_or("");
        out.push_str(&format!(
            "{:<20} {:<10} {:<15} {}\n",
            resource.resource_type, resource.count, resource.status, error
        ));
    }

    Ok(out)
}
