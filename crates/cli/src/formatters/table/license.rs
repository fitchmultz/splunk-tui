//! License table formatter.
//!
//! Responsibilities:
//! - Format license information as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::LicenseInfoOutput;
use anyhow::Result;
use splunk_config::constants::DEFAULT_LICENSE_ALERT_PCT;

/// Format license information as formatted text.
pub fn format_license(license: &LicenseInfoOutput) -> Result<String> {
    let mut output = String::new();

    output.push_str("--- License Usage ---\n");
    if license.usage.is_empty() {
        output.push_str("No license usage data available.\n");
    } else {
        output.push_str("Name\tStack ID\tUsed (MB)\tQuota (MB)\t% Used\tAlert\n");
        for u in &license.usage {
            let used_bytes = u.effective_used_bytes();
            let used_mb = used_bytes / 1024 / 1024;
            let quota_mb = u.quota / 1024 / 1024;
            let pct = if u.quota > 0 {
                (used_bytes as f64 / u.quota as f64) * 100.0
            } else {
                0.0
            };
            let alert = if pct > DEFAULT_LICENSE_ALERT_PCT {
                "!"
            } else {
                ""
            };
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{:.1}%\t{}\n",
                u.name,
                u.stack_id.as_deref().unwrap_or("N/A"),
                used_mb,
                quota_mb,
                pct,
                alert
            ));
        }
    }
    output.push('\n');

    output.push_str("--- License Pools ---\n");
    if license.pools.is_empty() {
        output.push_str("No license pools found.\n");
    } else {
        output.push_str("Name\tStack ID\tUsed (MB)\tQuota (MB)\tDescription\n");
        for p in &license.pools {
            let quota_mb = p
                .quota
                .parse::<u64>()
                .ok()
                .map(|q| (q / 1024 / 1024).to_string())
                .unwrap_or_else(|| p.quota.clone());
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                p.name,
                p.stack_id,
                p.used_bytes / 1024 / 1024,
                quota_mb,
                p.description.as_deref().unwrap_or("N/A")
            ));
        }
    }
    output.push('\n');

    output.push_str("--- License Stacks ---\n");
    if license.stacks.is_empty() {
        output.push_str("No license stacks found.\n");
    } else {
        output.push_str("Name\tLabel\tType\tQuota (MB)\n");
        for s in &license.stacks {
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                s.name,
                s.label,
                s.type_name,
                s.quota / 1024 / 1024
            ));
        }
    }

    Ok(output)
}
