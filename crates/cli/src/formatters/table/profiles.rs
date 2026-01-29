//! Profiles table formatter.
//!
//! Responsibilities:
//! - Format profile configurations as formatted tables.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// Format a single profile as formatted text.
pub fn format_profile(profile_name: &str, profile: &ProfileConfig) -> Result<String> {
    let mut output = String::new();

    output.push_str(&format!("{:<20} {}\n", "Profile Name:", profile_name));

    let base_url = profile.base_url.as_deref().unwrap_or("(not set)");
    output.push_str(&format!("{:<20} {}\n", "Base URL:", base_url));

    let username = profile.username.as_deref().unwrap_or("(not set)");
    output.push_str(&format!("{:<20} {}\n", "Username:", username));

    let password_display = match &profile.password {
        Some(_) => "****",
        None => "(not set)",
    };
    output.push_str(&format!("{:<20} {}\n", "Password:", password_display));

    let token_display = match &profile.api_token {
        Some(_) => "****",
        None => "(not set)",
    };
    output.push_str(&format!("{:<20} {}\n", "API Token:", token_display));

    let skip_verify = profile
        .skip_verify
        .map_or("(not set)".to_string(), |b| b.to_string());
    output.push_str(&format!("{:<20} {}\n", "Skip TLS Verify:", skip_verify));

    let timeout = profile
        .timeout_seconds
        .map_or("(not set)".to_string(), |t| t.to_string());
    output.push_str(&format!("{:<20} {}\n", "Timeout (sec):", timeout));

    let max_retries = profile
        .max_retries
        .map_or("(not set)".to_string(), |r| r.to_string());
    output.push_str(&format!("{:<20} {}", "Max Retries:", max_retries));

    Ok(output)
}

/// Format all profiles as a formatted table.
pub fn format_profiles(profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
    if profiles.is_empty() {
        return Ok(
            "No profiles configured. Use 'splunk-cli config set <profile-name>' to add one."
                .to_string(),
        );
    }

    let mut output = format!("{:<20} {:<40} {:<15}\n", "Profile", "Base URL", "Username");
    output.push_str(&format!("{}\n", "-".repeat(75)));

    for (name, profile) in profiles {
        let base_url = profile.base_url.as_deref().unwrap_or("-");
        let username = profile.username.as_deref().unwrap_or("-");
        output.push_str(&format!("{:<20} {:<40} {:<15}\n", name, base_url, username));
    }

    Ok(output)
}
