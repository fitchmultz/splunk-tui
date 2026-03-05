//! SHC table formatter.
//!
//! Responsibilities:
//! - Format SHC information as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.
//! - Pagination (handled in mod.rs).

use crate::formatters::table::Pagination;
use crate::formatters::{
    ShcCaptainOutput, ShcConfigOutput, ShcManagementOutput, ShcMemberOutput, ShcStatusOutput,
};
use anyhow::Result;

/// Format SHC status as a formatted text block.
pub fn format_shc_status(status: &ShcStatusOutput) -> Result<String> {
    let mut output = String::from("SHC Status:\n\n");
    output.push_str(&format!(
        "Is Captain: {}\n\
         Is Searchable: {}\n\
         Captain URI: {}\n\
         Member Count: {}\n\
         Minimum Member Count: {}\n\
         Rolling Restart: {}\n\
         Service Ready: {}\n",
        status.is_captain,
        status.is_searchable,
        status.captain_uri.as_deref().unwrap_or("N/A"),
        status.member_count,
        status
            .minimum_member_count
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        status
            .rolling_restart_flag
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        status
            .service_ready_flag
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
    ));
    Ok(output)
}

/// Format SHC members as a table.
pub fn format_shc_members(members: &[ShcMemberOutput], pagination: &Pagination) -> Result<String> {
    let mut output = String::from("SHC Members:\n\n");

    if members.is_empty() {
        output.push_str("No members found.\n");
        return Ok(output);
    }

    // Header
    output.push_str("Host\t\tStatus\tCaptain\tGUID\t\tSite\n");
    output.push_str("----\t\t------\t-------\t----\t\t----\n");

    for member in members {
        let captain_marker = if member.is_captain { "Yes" } else { "" };
        output.push_str(&format!(
            "{}:{}\t{}\t{}\t{}\t{}\n",
            member.host,
            member.port,
            member.status,
            captain_marker,
            &member.guid[..member.guid.len().min(8)],
            member.site.as_deref().unwrap_or("N/A"),
        ));
    }

    output.push_str(&format!(
        "\nShowing {} of {} members (offset: {})\n",
        members.len(),
        pagination.total.unwrap_or(members.len()),
        pagination.offset
    ));

    Ok(output)
}

/// Format SHC captain as a formatted text block.
pub fn format_shc_captain(captain: &ShcCaptainOutput) -> Result<String> {
    let mut output = String::from("SHC Captain:\n\n");
    output.push_str(&format!(
        "ID: {}\n\
         Host: {}\n\
         Port: {}\n\
         GUID: {}\n\
         Dynamic Captain: {}\n\
         Site: {}\n",
        captain.id,
        captain.host,
        captain.port,
        captain.guid,
        captain.is_dynamic_captain,
        captain.site.as_deref().unwrap_or("N/A"),
    ));
    Ok(output)
}

/// Format SHC config as a formatted text block.
pub fn format_shc_config(config: &ShcConfigOutput) -> Result<String> {
    let mut output = String::from("SHC Configuration:\n\n");
    output.push_str(&format!(
        "ID: {}\n\
         Replication Factor: {}\n\
         Captain URI: {}\n\
         SHCluster Label: {}\n",
        config.id,
        config
            .replication_factor
            .map(|v| v.to_string())
            .unwrap_or_else(|| "N/A".to_string()),
        config.captain_uri.as_deref().unwrap_or("N/A"),
        config.shcluster_label.as_deref().unwrap_or("N/A"),
    ));
    Ok(output)
}

/// Format SHC management operation result.
pub fn format_shc_management(output: &ShcManagementOutput) -> Result<String> {
    let status = if output.success { "SUCCESS" } else { "FAILED" };
    Ok(format!(
        "Operation: {}\nTarget: {}\nStatus: {}\nMessage: {}\n",
        output.operation, output.target, status, output.message
    ))
}
