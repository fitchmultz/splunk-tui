//! Multi-profile aggregation and output formatting for list-all command.
//!
//! Responsibilities:
//! - Fetch resources from multiple profiles in parallel.
//! - Format output as JSON, table, CSV, or XML.
//! - Handle per-profile errors gracefully without failing the entire command.
//!
//! Does NOT handle:
//! - Individual resource fetching (see `fetchers.rs`).
//! - Authentication strategy building (see `auth.rs`).
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

use super::auth::build_auth_strategy_from_profile;
use super::fetchers::fetch_all_resources;
use super::types::{ListAllMultiOutput, ProfileResult};

/// Returns the current timestamp in RFC3339 format.
pub fn format_timestamp() -> String {
    chrono::Utc::now().to_rfc3339()
}

/// Fetch resources from multiple profiles in parallel.
pub async fn fetch_multi_profile_resources(
    config_manager: splunk_config::ConfigManager,
    profile_names: Vec<String>,
    resource_types: Vec<String>,
    cancel: &CancellationToken,
) -> Result<ListAllMultiOutput> {
    let timestamp = format_timestamp();
    let mut profile_results = Vec::new();

    // Get all profile configs first
    let profiles_map = config_manager.list_profiles().clone();

    // Create futures for each profile
    let mut futures = Vec::new();
    for profile_name in &profile_names {
        let profile_config = profiles_map.get(profile_name).cloned();
        let resource_types = resource_types.clone();
        let profile_name = profile_name.clone();

        let future = async move {
            if let Some(config) = profile_config {
                fetch_single_profile_resources(profile_name, config, resource_types, cancel).await
            } else {
                ProfileResult {
                    profile_name,
                    base_url: String::new(),
                    resources: vec![],
                    error: Some("Profile configuration not found".to_string()),
                }
            }
        };
        futures.push(future);
    }

    // Execute all futures concurrently
    let results = futures::future::join_all(futures).await;
    profile_results.extend(results);

    Ok(ListAllMultiOutput {
        timestamp,
        profiles: profile_results,
    })
}

/// Fetch resources from a single profile.
async fn fetch_single_profile_resources(
    profile_name: String,
    profile_config: splunk_config::types::ProfileConfig,
    resource_types: Vec<String>,
    cancel: &CancellationToken,
) -> ProfileResult {
    // Build config from profile
    let base_url = profile_config.base_url.clone().unwrap_or_default();

    // Build auth strategy - fail fast if credentials are missing/invalid
    let auth_strategy = match build_auth_strategy_from_profile(&profile_config) {
        Ok(strategy) => strategy,
        Err(error_msg) => {
            return ProfileResult {
                profile_name,
                base_url,
                resources: vec![],
                error: Some(error_msg),
            };
        }
    };

    // Build Splunk client
    let client = match splunk_client::SplunkClient::builder()
        .base_url(base_url.clone())
        .auth_strategy(auth_strategy)
        .skip_verify(profile_config.skip_verify.unwrap_or(false))
        .timeout(std::time::Duration::from_secs(
            profile_config.timeout_seconds.unwrap_or(30),
        ))
        .session_ttl_seconds(profile_config.session_ttl_seconds.unwrap_or(3600))
        .session_expiry_buffer_seconds(profile_config.session_expiry_buffer_seconds.unwrap_or(60))
        .build()
    {
        Ok(c) => c,
        Err(e) => {
            return ProfileResult {
                profile_name,
                base_url,
                resources: vec![],
                error: Some(format!("Failed to build client: {}", e)),
            };
        }
    };

    // Fetch resources
    match fetch_all_resources(&client, resource_types, cancel).await {
        Ok(resources) => ProfileResult {
            profile_name,
            base_url,
            resources,
            error: None,
        },
        Err(e) => ProfileResult {
            profile_name,
            base_url,
            resources: vec![],
            error: Some(e.to_string()),
        },
    }
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

/// Escape a string for CSV output.
fn escape_csv(s: &str) -> String {
    if s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r') {
        format!("\"{}\"", s.replace('"', "\"\""))
    } else {
        s.to_string()
    }
}
