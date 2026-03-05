//! Profiles XML formatter.
//!
//! Responsibilities:
//! - Format profile configurations as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// Format a single profile as XML.
pub fn format_profile(profile_name: &str, profile: &ProfileConfig) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<profile>\n");

    xml.push_str(&format!("  <name>{}</name>\n", escape_xml(profile_name)));

    let base_url = profile.base_url.as_deref().unwrap_or("N/A");
    xml.push_str(&format!(
        "  <base_url>{}</base_url>\n",
        escape_xml(base_url)
    ));

    let username = profile.username.as_deref().unwrap_or("N/A");
    xml.push_str(&format!(
        "  <username>{}</username>\n",
        escape_xml(username)
    ));

    let password_display = match &profile.password {
        Some(_) => "****",
        None => "N/A",
    };
    xml.push_str(&format!(
        "  <password>{}</password>\n",
        escape_xml(password_display)
    ));

    let token_display = match &profile.api_token {
        Some(_) => "****",
        None => "N/A",
    };
    xml.push_str(&format!(
        "  <api_token>{}</api_token>\n",
        escape_xml(token_display)
    ));

    if let Some(skip_verify) = profile.skip_verify {
        xml.push_str(&format!("  <skip_verify>{}</skip_verify>\n", skip_verify));
    }

    if let Some(timeout) = profile.timeout_seconds {
        xml.push_str(&format!(
            "  <timeout_seconds>{}</timeout_seconds>\n",
            timeout
        ));
    }

    if let Some(max_retries) = profile.max_retries {
        xml.push_str(&format!("  <max_retries>{}</max_retries>\n", max_retries));
    }

    xml.push_str("</profile>");
    Ok(xml)
}

/// Format all profiles as XML.
pub fn format_profiles(profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<profiles>\n");
    for (name, profile) in profiles {
        xml.push_str("  <profile>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(name)));
        if let Some(ref url) = profile.base_url {
            xml.push_str(&format!("    <base_url>{}</base_url>\n", escape_xml(url)));
        }
        if let Some(ref user) = profile.username {
            xml.push_str(&format!("    <username>{}</username>\n", escape_xml(user)));
        }
        if let Some(skip) = profile.skip_verify {
            xml.push_str(&format!("    <skip_verify>{}</skip_verify>\n", skip));
        }
        if let Some(timeout) = profile.timeout_seconds {
            xml.push_str(&format!(
                "    <timeout_seconds>{}</timeout_seconds>\n",
                timeout
            ));
        }
        if let Some(retries) = profile.max_retries {
            xml.push_str(&format!("    <max_retries>{}</max_retries>\n", retries));
        }
        xml.push_str("  </profile>\n");
    }
    xml.push_str("</profiles>\n");
    Ok(xml)
}
