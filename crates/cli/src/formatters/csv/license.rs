//! License CSV formatter.
//!
//! Responsibilities:
//! - Format license information as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::LicenseInfoOutput;
use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;

/// Format license info as CSV.
pub fn format_license(license: &LicenseInfoOutput) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "Type",
        "Name",
        "StackID",
        "UsedMB",
        "QuotaMB",
        "PctUsed",
        "Label",
        "Type_Name",
        "Description",
    ]));

    // Usage
    for u in &license.usage {
        let used_bytes = u.effective_used_bytes();
        let pct = if u.quota > 0 {
            (used_bytes as f64 / u.quota as f64) * 100.0
        } else {
            0.0
        };
        output.push_str(&build_csv_row(&[
            escape_csv("Usage"),
            escape_csv(&u.name),
            format_opt_str(u.stack_id.as_deref(), "N/A"),
            escape_csv(&format!("{}", used_bytes / 1024 / 1024)),
            escape_csv(&format!("{}", u.quota / 1024 / 1024)),
            escape_csv(&format!("{:.2}", pct)),
            escape_csv(""),
            escape_csv(""),
            escape_csv(" "),
        ]));
    }

    // Pools
    for p in &license.pools {
        let quota_mb = p
            .quota
            .parse::<u64>()
            .ok()
            .map(|q| (q / 1024 / 1024).to_string())
            .unwrap_or_else(|| p.quota.clone());
        output.push_str(&build_csv_row(&[
            escape_csv("Pool"),
            escape_csv(&p.name),
            escape_csv(&p.stack_id),
            escape_csv(&format!("{}", p.used_bytes / 1024 / 1024)),
            escape_csv(&quota_mb),
            escape_csv(""),
            escape_csv(""),
            escape_csv(""),
            format_opt_str(p.description.as_deref(), "N/A"),
        ]));
    }

    // Stacks
    for s in &license.stacks {
        output.push_str(&build_csv_row(&[
            escape_csv("Stack"),
            escape_csv(&s.name),
            escape_csv(""),
            escape_csv("0"),
            escape_csv(&format!("{}", s.quota / 1024 / 1024)),
            escape_csv(""),
            escape_csv(&s.label),
            escape_csv(&s.type_name),
            escape_csv(""),
        ]));
    }

    Ok(output)
}
