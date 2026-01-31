//! Lookups CSV formatter.
//!
//! Responsibilities:
//! - Format lookup tables as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv};
use anyhow::Result;
use splunk_client::LookupTable;

/// Format lookup tables as CSV.
pub fn format_lookups(lookups: &[LookupTable]) -> Result<String> {
    if lookups.is_empty() {
        return Ok(String::new());
    }

    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "Name", "Filename", "Owner", "App", "Sharing", "Size",
    ]));

    for lookup in lookups {
        output.push_str(&build_csv_row(&[
            escape_csv(&lookup.name),
            escape_csv(&lookup.filename),
            escape_csv(&lookup.owner),
            escape_csv(&lookup.app),
            escape_csv(&lookup.sharing),
            escape_csv(&lookup.size.to_string()),
        ]));
    }

    Ok(output)
}
