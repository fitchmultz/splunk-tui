//! Profiles CSV formatter.
//!
//! Responsibilities:
//! - Format profile configurations as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// Format single profile as CSV.
pub fn format_profile(profile_name: &str, profile: &ProfileConfig) -> Result<String> {
    let mut csv = String::new();

    csv.push_str(&build_csv_header(&["field", "value"]));

    csv.push_str(&build_csv_row(&[
        escape_csv("Profile Name"),
        escape_csv(profile_name),
    ]));

    csv.push_str(&build_csv_row(&[
        escape_csv("Base URL"),
        format_opt_str(profile.base_url.as_deref(), "N/A"),
    ]));

    csv.push_str(&build_csv_row(&[
        escape_csv("Username"),
        format_opt_str(profile.username.as_deref(), "N/A"),
    ]));

    let password_display = match &profile.password {
        Some(_) => "****",
        None => "N/A",
    };
    csv.push_str(&build_csv_row(&[
        escape_csv("Password"),
        escape_csv(password_display),
    ]));

    let token_display = match &profile.api_token {
        Some(_) => "****",
        None => "(not set)",
    };
    csv.push_str(&build_csv_row(&[
        escape_csv("API Token"),
        escape_csv(token_display),
    ]));

    let skip_verify = profile
        .skip_verify
        .map_or("N/A".to_string(), |b| b.to_string());
    csv.push_str(&build_csv_row(&[
        escape_csv("Skip TLS Verify"),
        escape_csv(&skip_verify),
    ]));

    let timeout = profile
        .timeout_seconds
        .map_or("(not set)".to_string(), |t| t.to_string());
    csv.push_str(&build_csv_row(&[
        escape_csv("Timeout (sec)"),
        escape_csv(&timeout),
    ]));

    let max_retries = profile
        .max_retries
        .map_or("(not set)".to_string(), |r| r.to_string());
    csv.push_str(&build_csv_row(&[
        escape_csv("Max Retries"),
        escape_csv(&max_retries),
    ]));

    Ok(csv)
}

/// Format all profiles as CSV.
pub fn format_profiles(profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
    let mut csv = String::new();
    csv.push_str(&build_csv_header(&[
        "profile",
        "base_url",
        "username",
        "skip_verify",
        "timeout_seconds",
        "max_retries",
    ]));

    for (name, profile) in profiles {
        csv.push_str(&build_csv_row(&[
            escape_csv(name),
            format_opt_str(profile.base_url.as_deref(), "N/A"),
            format_opt_str(profile.username.as_deref(), "N/A"),
            escape_csv(
                &profile
                    .skip_verify
                    .map_or("N/A".to_string(), |b| b.to_string()),
            ),
            escape_csv(
                &profile
                    .timeout_seconds
                    .map_or("".to_string(), |t| t.to_string()),
            ),
            escape_csv(
                &profile
                    .max_retries
                    .map_or("".to_string(), |r| r.to_string()),
            ),
        ]));
    }

    Ok(csv)
}
