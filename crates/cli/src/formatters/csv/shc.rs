//! SHC CSV formatter.
//!
//! Responsibilities:
//! - Format SHC information as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv};
use crate::formatters::{ShcCaptainOutput, ShcConfigOutput, ShcManagementOutput, ShcStatusOutput};
use anyhow::Result;

/// Format SHC status as CSV.
pub fn format_shc_status(status: &ShcStatusOutput) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "Is Captain",
        "Is Searchable",
        "Captain URI",
        "Member Count",
        "Minimum Member Count",
        "Rolling Restart",
        "Service Ready",
    ]));

    // Row
    output.push_str(&build_csv_row(&[
        status.is_captain.to_string(),
        status.is_searchable.to_string(),
        escape_csv(status.captain_uri.as_deref().unwrap_or("")),
        status.member_count.to_string(),
        status
            .minimum_member_count
            .map(|v| v.to_string())
            .unwrap_or_default(),
        status
            .rolling_restart_flag
            .map(|v| v.to_string())
            .unwrap_or_default(),
        status
            .service_ready_flag
            .map(|v| v.to_string())
            .unwrap_or_default(),
    ]));

    Ok(output)
}

/// Format SHC captain as CSV.
pub fn format_shc_captain(captain: &ShcCaptainOutput) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "ID",
        "Host",
        "Port",
        "GUID",
        "Dynamic Captain",
        "Site",
    ]));

    // Row
    output.push_str(&build_csv_row(&[
        escape_csv(&captain.id),
        escape_csv(&captain.host),
        captain.port.to_string(),
        escape_csv(&captain.guid),
        captain.is_dynamic_captain.to_string(),
        escape_csv(captain.site.as_deref().unwrap_or("")),
    ]));

    Ok(output)
}

/// Format SHC config as CSV.
pub fn format_shc_config(config: &ShcConfigOutput) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "ID",
        "Replication Factor",
        "Captain URI",
        "SHCluster Label",
    ]));

    // Row
    output.push_str(&build_csv_row(&[
        escape_csv(&config.id),
        config
            .replication_factor
            .map(|v| v.to_string())
            .unwrap_or_default(),
        escape_csv(config.captain_uri.as_deref().unwrap_or("")),
        escape_csv(config.shcluster_label.as_deref().unwrap_or("")),
    ]));

    Ok(output)
}

/// Format SHC management operation result as CSV.
pub fn format_shc_management(output: &ShcManagementOutput) -> Result<String> {
    let mut result = String::new();

    // Header
    result.push_str(&build_csv_header(&[
        "Operation",
        "Target",
        "Success",
        "Message",
    ]));

    // Row
    result.push_str(&build_csv_row(&[
        escape_csv(&output.operation),
        escape_csv(&output.target),
        output.success.to_string(),
        escape_csv(&output.message),
    ]));

    Ok(result)
}
