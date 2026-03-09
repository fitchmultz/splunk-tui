//! Profiles CSV formatter.
//!
//! Responsibilities:
//! - Format profile configurations as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{
    build_csv_header, build_csv_row, build_profile_fields, build_profile_summary_row,
    format_opt_str,
};
use anyhow::Result;
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// Format single profile as CSV.
pub fn format_profile(profile_name: &str, profile: &ProfileConfig) -> Result<String> {
    let mut csv = String::new();
    csv.push_str(&build_csv_header(&["field", "value"]));
    for field in build_profile_fields(profile_name, profile) {
        csv.push_str(&build_csv_row(&[
            crate::formatters::common::escape_csv(field.label),
            crate::formatters::common::escape_csv(&field.value),
        ]));
    }

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
        let row = build_profile_summary_row(name, profile);
        csv.push_str(&build_csv_row(&[
            crate::formatters::common::escape_csv(&row.name),
            format_opt_str(Some(&row.base_url), "N/A"),
            format_opt_str(Some(&row.username), "N/A"),
            crate::formatters::common::escape_csv(&row.skip_verify),
            crate::formatters::common::escape_csv(&row.timeout_seconds),
            crate::formatters::common::escape_csv(&row.max_retries),
        ]));
    }

    Ok(csv)
}
