//! Health CSV formatter.
//!
//! Responsibilities:
//! - Format health check and KV store status as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

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
        .map(|h| h.health.as_str())
        .unwrap_or("N/A");

    let (used, quota) = if let Some(usage) = &health.license_usage {
        let used: u64 = usage.iter().map(|u| u.effective_used_bytes()).sum();
        let quota: u64 = usage.iter().map(|u| u.quota).sum();
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
        .map(|kv| kv.current_member.status.as_str())
        .unwrap_or("N/A");
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
        escape_csv(health_status),
        escape_csv(&used),
        escape_csv(&quota),
        escape_csv(kv_status),
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
        escape_csv(&status.current_member.status),
        escape_csv(&status.current_member.replica_set),
        escape_csv(&status.replication_status.oplog_size.to_string()),
        escape_csv(&status.replication_status.oplog_used.to_string()),
    ];
    output.push_str(&build_csv_row(&row));

    Ok(output)
}
