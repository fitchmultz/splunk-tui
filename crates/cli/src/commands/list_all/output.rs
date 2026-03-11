//! Multi-profile aggregation and output formatting for list-all command.
//!
//! Responsibilities:
//! - Fetch resources from multiple profiles in parallel.
//! - Format output as JSON, table, CSV, or XML.
//! - Handle per-profile errors gracefully without failing the entire command.
//!
//! Does NOT handle:
//! - Individual resource fetching implementation (lives in `splunk-client::workflows`).
//! - Type definitions (see `types.rs`).
//!
//! Invariants:
//! - Profile-level errors are captured in ProfileResult, not propagated.
//! - Timestamp is always RFC3339 format.
//! - All futures are joined for concurrent execution.

use crate::cancellation::CancellationToken;
use crate::formatters::OutputFormat;
use crate::formatters::escape_xml;
use anyhow::Result;

use super::types::ListAllMultiOutput;

/// Returns the current timestamp in RFC3339 format.
pub fn format_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Fetch resources from multiple profiles in parallel.
pub async fn fetch_multi_profile_resources(
    profiles: Vec<(String, splunk_config::ProfileConfig)>,
    resource_types: Vec<String>,
    cancel: &CancellationToken,
) -> Result<ListAllMultiOutput> {
    if cancel.is_cancelled() {
        anyhow::bail!("List-all request cancelled");
    }

    splunk_client::workflows::multi_profile::fetch_multi_profile_overview(
        profiles,
        resource_types,
        Some(cancel),
    )
    .await
}

/// Format multi-profile output based on the selected format.
pub fn format_multi_profile_output(
    output: &ListAllMultiOutput,
    format: OutputFormat,
) -> Result<String> {
    match format {
        OutputFormat::Json => Ok(serde_json::to_string_pretty(output)?),
        OutputFormat::Table => format_multi_profile_table(output),
        OutputFormat::Csv => format_multi_profile_csv(output),
        OutputFormat::Xml => format_multi_profile_xml(output),
        OutputFormat::Ndjson => format_multi_profile_ndjson(output),
        OutputFormat::Yaml => Ok(serde_yaml::to_string(output)?),
        OutputFormat::Markdown => format_multi_profile_markdown(output),
    }
}

/// Format multi-profile output as a table.
fn format_multi_profile_table(output: &ListAllMultiOutput) -> Result<String> {
    let mut out = String::new();

    out.push_str(&format!("Timestamp: {}\n", output.timestamp));
    out.push('\n');

    if output.profiles.is_empty() {
        out.push_str("No profiles found.\n");
        return Ok(out);
    }

    for profile in &output.profiles {
        out.push_str(&format!(
            "=== Profile: {} ({}) ===\n",
            profile.profile_name, profile.base_url
        ));

        if let Some(ref error) = profile.error {
            out.push_str(&format!("Error: {}\n", error));
            out.push('\n');
            continue;
        }

        if profile.resources.is_empty() {
            out.push_str("No resources found.\n");
        } else {
            let header = format!(
                "{:<20} {:<10} {:<15} {}",
                "Resource Type", "Count", "Status", "Error"
            );
            out.push_str(&header);
            out.push('\n');

            let separator = format!("{:<20} {:<10} {:<15} {}", "====", "=====", "=====", "=====");
            out.push_str(&separator);
            out.push('\n');

            for resource in &profile.resources {
                let error = resource.error.as_deref().unwrap_or("");
                out.push_str(&format!(
                    "{:<20} {:<10} {:<15} {}\n",
                    resource.resource_type, resource.count, resource.status, error
                ));
            }
        }
        out.push('\n');
    }

    Ok(out)
}

/// Format multi-profile output as CSV.
fn format_multi_profile_csv(output: &ListAllMultiOutput) -> Result<String> {
    let mut csv = String::new();

    csv.push_str("profile_name,base_url,timestamp,resource_type,count,status,error\n");

    for profile in &output.profiles {
        if let Some(ref error) = profile.error {
            // Profile-level error
            csv.push_str(&format!(
                "{},{},{},,,,{}\n",
                escape_csv(&profile.profile_name),
                escape_csv(&profile.base_url),
                escape_csv(&output.timestamp),
                escape_csv(error)
            ));
        } else {
            for resource in &profile.resources {
                let error = resource.error.as_deref().unwrap_or("");
                csv.push_str(&format!(
                    "{},{},{},{},{},{},{}\n",
                    escape_csv(&profile.profile_name),
                    escape_csv(&profile.base_url),
                    escape_csv(&output.timestamp),
                    escape_csv(&resource.resource_type),
                    resource.count,
                    escape_csv(&resource.status),
                    escape_csv(error)
                ));
            }
        }
    }

    Ok(csv)
}

/// Format multi-profile output as XML.
fn format_multi_profile_xml(output: &ListAllMultiOutput) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<list_all_multi>\n");
    xml.push_str(&format!(
        "  <timestamp>{}</timestamp>\n",
        escape_xml(&output.timestamp)
    ));
    xml.push_str("  <profiles>\n");

    for profile in &output.profiles {
        xml.push_str("    <profile>\n");
        xml.push_str(&format!(
            "      <name>{}</name>\n",
            escape_xml(&profile.profile_name)
        ));
        xml.push_str(&format!(
            "      <base_url>{}</base_url>\n",
            escape_xml(&profile.base_url)
        ));

        if let Some(ref error) = profile.error {
            xml.push_str(&format!("      <error>{}</error>\n", escape_xml(error)));
        } else {
            xml.push_str("      <resources>\n");
            for resource in &profile.resources {
                xml.push_str("        <resource>\n");
                xml.push_str(&format!(
                    "          <type>{}</type>\n",
                    escape_xml(&resource.resource_type)
                ));
                xml.push_str(&format!("          <count>{}</count>\n", resource.count));
                xml.push_str(&format!(
                    "          <status>{}</status>\n",
                    escape_xml(&resource.status)
                ));
                if let Some(ref error) = resource.error {
                    xml.push_str(&format!("          <error>{}</error>\n", escape_xml(error)));
                }
                xml.push_str("        </resource>\n");
            }
            xml.push_str("      </resources>\n");
        }

        xml.push_str("    </profile>\n");
    }

    xml.push_str("  </profiles>\n");
    xml.push_str("</list_all_multi>");
    Ok(xml)
}

/// Format multi-profile output as NDJSON.
fn format_multi_profile_ndjson(output: &ListAllMultiOutput) -> Result<String> {
    let mut ndjson = String::new();

    for profile in &output.profiles {
        if let Some(ref error) = profile.error {
            // Profile-level error
            let line = serde_json::json!({
                "profile_name": &profile.profile_name,
                "base_url": &profile.base_url,
                "timestamp": &output.timestamp,
                "resource_type": null,
                "count": null,
                "status": "error",
                "error": error
            });
            ndjson.push_str(&serde_json::to_string(&line)?);
            ndjson.push('\n');
        } else if profile.resources.is_empty() {
            // No resources found for this profile
            let line = serde_json::json!({
                "profile_name": &profile.profile_name,
                "base_url": &profile.base_url,
                "timestamp": &output.timestamp,
                "resource_type": null,
                "count": null,
                "status": "empty",
                "error": null
            });
            ndjson.push_str(&serde_json::to_string(&line)?);
            ndjson.push('\n');
        } else {
            for resource in &profile.resources {
                let line = serde_json::json!({
                    "profile_name": &profile.profile_name,
                    "base_url": &profile.base_url,
                    "timestamp": &output.timestamp,
                    "resource_type": &resource.resource_type,
                    "count": resource.count,
                    "status": &resource.status,
                    "error": resource.error.as_deref().unwrap_or("")
                });
                ndjson.push_str(&serde_json::to_string(&line)?);
                ndjson.push('\n');
            }
        }
    }

    Ok(ndjson)
}

/// Format multi-profile output as Markdown.
fn format_multi_profile_markdown(output: &ListAllMultiOutput) -> Result<String> {
    let mut md = String::new();
    md.push_str("# Multi-Profile Resource Summary\n\n");
    md.push_str(&format!("**Timestamp**: {}\n\n", output.timestamp));

    if output.profiles.is_empty() {
        md.push_str("_No profiles found._\n");
        return Ok(md);
    }

    for profile in &output.profiles {
        md.push_str(&format!("## Profile: {}\n\n", profile.profile_name));
        md.push_str(&format!("- **Base URL**: {}\n", profile.base_url));

        if let Some(ref error) = profile.error {
            md.push_str(&format!("- **Error**: {}\n", error));
        } else if profile.resources.is_empty() {
            md.push_str("- **Status**: No resources found\n");
        } else {
            md.push_str("\n### Resources\n\n");
            md.push_str("| Type | Count | Status |\n");
            md.push_str("|------|-------|--------|\n");
            for resource in &profile.resources {
                md.push_str(&format!(
                    "| {} | {} | {} |\n",
                    resource.resource_type, resource.count, resource.status
                ));
            }
        }
        md.push('\n');
    }

    Ok(md)
}

/// Escape a string for CSV output.
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
