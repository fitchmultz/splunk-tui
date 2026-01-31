//! List All CSV formatter.
//!
//! Responsibilities:
//! - Format unified resource overview as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::commands::list_all::ListAllOutput;
use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;

/// Format list-all output as CSV.
#[allow(dead_code)]
pub fn format_list_all(output: &ListAllOutput) -> Result<String> {
    let mut csv = String::new();

    csv.push_str(&build_csv_header(&[
        "timestamp",
        "resource_type",
        "count",
        "status",
        "error",
    ]));

    for resource in &output.resources {
        csv.push_str(&build_csv_row(&[
            escape_csv(&output.timestamp),
            escape_csv(&resource.resource_type),
            escape_csv(&resource.count.to_string()),
            escape_csv(&resource.status),
            format_opt_str(resource.error.as_deref(), ""),
        ]));
    }

    Ok(csv)
}
