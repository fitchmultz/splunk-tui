//! Health CSV formatter.
//!
//! Responsibilities:
//! - Format health check and KV store status as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::DiagnosticReport;
use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv};
use anyhow::Result;
use splunk_client::{HealthCheckOutput, KvStoreStatus};

/// Format health check as CSV.
pub fn format_health(health: &HealthCheckOutput) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "server_name",
        "version",
        "health_status",
        "license_used_mb",
        "license_quota_mb",
        "kvstore_status",
        "log_parsing_healthy",
        "log_parsing_errors",
    ]));

    // Data row
    let server_name = health
        .server_info
        .as_ref()
        .map(|i| i.server_name.as_str())
        .unwrap_or("N/A");
    let version = health
        .server_info
        .as_ref()
        .map(|i| i.version.as_str())
        .unwrap_or("N/A");
    let health_status = health
        .splunkd_health
        .as_ref()
        .map(|h| h.health.to_string())
        .unwrap_or_else(|| "N/A".to_string());

    let (used, quota) = if let Some(usage) = &health.license_usage {
        let used: usize = usage.iter().map(|u| u.effective_used_bytes()).sum();
        let quota: usize = usage.iter().map(|u| u.quota).sum();
        (
            (used / 1024 / 1024).to_string(),
            (quota / 1024 / 1024).to_string(),
        )
    } else {
        ("N/A".to_string(), "N/A".to_string())
    };

    let kv_status = health
        .kvstore_status
        .as_ref()
        .map(|kv| kv.current_member.status.to_string())
        .unwrap_or_else(|| "N/A".to_string());
    let parsing_healthy = health
        .log_parsing_health
        .as_ref()
        .map(|lp| if lp.is_healthy { "Yes" } else { "No" })
        .unwrap_or("N/A");
    let parsing_errors = health
        .log_parsing_health
        .as_ref()
        .map(|lp| lp.total_errors.to_string())
        .unwrap_or_else(|| "N/A".to_string());

    let row = vec![
        escape_csv(server_name),
        escape_csv(version),
        escape_csv(&health_status),
        escape_csv(&used),
        escape_csv(&quota),
        escape_csv(&kv_status),
        escape_csv(parsing_healthy),
        escape_csv(&parsing_errors),
    ];
    output.push_str(&build_csv_row(&row));

    Ok(output)
}

/// Format KV store status as CSV.
pub fn format_kvstore_status(status: &KvStoreStatus) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "host",
        "port",
        "status",
        "replica_set",
        "oplog_size_mb",
        "oplog_used_percent",
    ]));

    // Data row
    let row = vec![
        escape_csv(&status.current_member.host),
        escape_csv(&status.current_member.port.to_string()),
        escape_csv(&status.current_member.status.to_string()),
        escape_csv(&status.current_member.replica_set),
        escape_csv(&status.replication_status.oplog_size.to_string()),
        escape_csv(&status.replication_status.oplog_used.to_string()),
    ];
    output.push_str(&build_csv_row(&row));

    Ok(output)
}

/// Format diagnostic report as CSV.
pub fn format_health_check_report(report: &DiagnosticReport) -> Result<String> {
    let mut output = String::new();

    // Metadata section
    output.push_str(&build_csv_header(&["property", "value"]));
    output.push_str(&build_csv_row(&[
        escape_csv("cli_version"),
        escape_csv(&report.cli_version),
    ]));
    output.push_str(&build_csv_row(&[
        escape_csv("os_arch"),
        escape_csv(&report.os_arch),
    ]));
    output.push_str(&build_csv_row(&[
        escape_csv("timestamp"),
        escape_csv(&report.timestamp),
    ]));
    output.push_str(&build_csv_row(&[
        escape_csv("base_url"),
        escape_csv(&report.config_summary.base_url),
    ]));
    output.push_str(&build_csv_row(&[
        escape_csv("auth_strategy"),
        escape_csv(&report.config_summary.auth_strategy),
    ]));
    output.push_str(&build_csv_row(&[
        escape_csv("skip_verify"),
        escape_csv(&report.config_summary.skip_verify.to_string()),
    ]));
    output.push_str(&build_csv_row(&[
        escape_csv("timeout_secs"),
        escape_csv(&report.config_summary.timeout_secs.to_string()),
    ]));
    output.push_str(&build_csv_row(&[
        escape_csv("max_retries"),
        escape_csv(&report.config_summary.max_retries.to_string()),
    ]));

    // Blank line and checks header
    output.push('\n');
    output.push_str(&build_csv_header(&["check_name", "status", "message"]));

    // Check results
    for check in &report.checks {
        let status_str = match check.status {
            crate::formatters::CheckStatus::Pass => "pass",
            crate::formatters::CheckStatus::Fail => "fail",
            crate::formatters::CheckStatus::Warning => "warning",
            crate::formatters::CheckStatus::Skipped => "skipped",
        };
        output.push_str(&build_csv_row(&[
            escape_csv(&check.name),
            escape_csv(status_str),
            escape_csv(&check.message),
        ]));
    }

    // Partial errors (if any)
    if !report.partial_errors.is_empty() {
        output.push('\n');
        output.push_str(&build_csv_header(&["endpoint", "error"]));
        for (endpoint, error) in &report.partial_errors {
            output.push_str(&build_csv_row(&[escape_csv(endpoint), escape_csv(error)]));
        }
    }

    Ok(output)
}
