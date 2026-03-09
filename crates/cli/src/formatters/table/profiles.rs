//! Profiles table formatter.
//!
//! Responsibilities:
//! - Format profile configurations as formatted tables.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_profile_fields, build_profile_summary_row};
use anyhow::Result;
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// Format a single profile as formatted text.
pub fn format_profile(profile_name: &str, profile: &ProfileConfig) -> Result<String> {
    let mut output = String::new();

    let fields = build_profile_fields(profile_name, profile);
    for (index, field) in fields.iter().enumerate() {
        let suffix = if index + 1 == fields.len() { "" } else { "\n" };
        output.push_str(&format!(
            "{:<20} {}{}",
            format!("{}:", field.label),
            field.value,
            suffix
        ));
    }

    Ok(output)
}

/// Format all profiles as a formatted table.
pub fn format_profiles(profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
    if profiles.is_empty() {
        return Ok(
            "Failed to list profiles: No profiles configured. Use 'splunk-cli config set <profile-name>' to add one."
                .to_string(),
        );
    }

    let mut output = format!("{:<20} {:<40} {:<15}\n", "Profile", "Base URL", "Username");
    output.push_str(&format!("{}\n", "-".repeat(75)));

    for (name, profile) in profiles {
        let row = build_profile_summary_row(name, profile);
        output.push_str(&format!(
            "{:<20} {:<40} {:<15}\n",
            row.name, row.base_url, row.username
        ));
    }

    Ok(output)
}
