//! License CSV formatter.
//!
//! Responsibilities:
//! - Format license information as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use crate::formatters::{LicenseInfoOutput, LicenseInstallOutput, LicensePoolOperationOutput};
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

/// Format installed licenses as CSV.
pub fn format_installed_licenses(licenses: &[splunk_client::InstalledLicense]) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "Name",
        "Type",
        "Status",
        "QuotaMB",
        "Expiration",
    ]));

    for license in licenses {
        output.push_str(&build_csv_row(&[
            escape_csv(&license.name),
            escape_csv(&license.license_type),
            escape_csv(&license.status),
            escape_csv(&format!("{}", license.quota_bytes / 1024 / 1024)),
            format_opt_str(license.expiration_time.as_deref(), "N/A"),
        ]));
    }

    Ok(output)
}

/// Format license installation result as CSV.
pub fn format_license_install(result: &LicenseInstallOutput) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&["Success", "Message", "LicenseName"]));

    output.push_str(&build_csv_row(&[
        escape_csv(&result.success.to_string()),
        escape_csv(&result.message),
        format_opt_str(result.license_name.as_deref(), ""),
    ]));

    Ok(output)
}

/// Format license pools as CSV.
pub fn format_license_pools(pools: &[splunk_client::LicensePool]) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "Name",
        "StackID",
        "UsedMB",
        "QuotaMB",
        "Description",
    ]));

    for p in pools {
        let quota_mb = p
            .quota
            .parse::<u64>()
            .ok()
            .map(|q| (q / 1024 / 1024).to_string())
            .unwrap_or_else(|| p.quota.clone());
        output.push_str(&build_csv_row(&[
            escape_csv(&p.name),
            escape_csv(&p.stack_id),
            escape_csv(&format!("{}", p.used_bytes / 1024 / 1024)),
            escape_csv(&quota_mb),
            format_opt_str(p.description.as_deref(), "N/A"),
        ]));
    }

    Ok(output)
}

/// Format license pool operation result as CSV.
pub fn format_license_pool_operation(result: &LicensePoolOperationOutput) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "Operation",
        "PoolName",
        "Success",
        "Message",
    ]));

    output.push_str(&build_csv_row(&[
        escape_csv(&result.operation),
        escape_csv(&result.pool_name),
        escape_csv(&result.success.to_string()),
        escape_csv(&result.message),
    ]));

    Ok(output)
}
