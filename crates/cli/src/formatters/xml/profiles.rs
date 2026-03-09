//! Profiles XML formatter.
//!
//! Responsibilities:
//! - Format profile configurations as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_profile_fields, build_profile_summary_row, escape_xml};
use anyhow::Result;
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// Format a single profile as XML.
pub fn format_profile(profile_name: &str, profile: &ProfileConfig) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<profile>\n");
    for field in build_profile_fields(profile_name, profile) {
        xml.push_str(&format!(
            "  <{}>{}</{}>\n",
            field.key,
            escape_xml(&field.value),
            field.key
        ));
    }

    xml.push_str("</profile>");
    Ok(xml)
}

/// Format all profiles as XML.
pub fn format_profiles(profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<profiles>\n");
    for (name, profile) in profiles {
        let row = build_profile_summary_row(name, profile);
        xml.push_str("  <profile>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&row.name)));
        xml.push_str(&format!(
            "    <base_url>{}</base_url>\n",
            escape_xml(&row.base_url)
        ));
        xml.push_str(&format!(
            "    <username>{}</username>\n",
            escape_xml(&row.username)
        ));
        if !row.skip_verify.is_empty() {
            xml.push_str(&format!(
                "    <skip_verify>{}</skip_verify>\n",
                escape_xml(&row.skip_verify)
            ));
        }
        if !row.timeout_seconds.is_empty() {
            xml.push_str(&format!(
                "    <timeout_seconds>{}</timeout_seconds>\n",
                escape_xml(&row.timeout_seconds)
            ));
        }
        if !row.max_retries.is_empty() {
            xml.push_str(&format!(
                "    <max_retries>{}</max_retries>\n",
                escape_xml(&row.max_retries)
            ));
        }
        xml.push_str("  </profile>\n");
    }
    xml.push_str("</profiles>\n");
    Ok(xml)
}
