//! Health check table formatter.
//!
//! Responsibilities:
//! - Format health check results and KVStore status as formatted text.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::DiagnosticReport;
use anyhow::Result;
use splunk_client::{HealthCheckOutput, KvStoreStatus};

/// Format health check results as formatted text.
pub fn format_health(health: &HealthCheckOutput) -> Result<String> {
    let mut output = String::new();

    if let Some(info) = &health.server_info {
        output.push_str("--- Server Info ---\n");
        output.push_str(&format!("Server Name: {}\n", info.server_name));
        output.push_str(&format!("Version: {}\n", info.version));
        output.push_str(&format!("Build: {}\n", info.build));
        output.push_str(&format!(
            "OS: {}\n",
            info.os_name.as_deref().unwrap_or("N/A")
        ));
        output.push_str(&format!("Roles: {}\n", info.server_roles.join(", ")));
        output.push('\n');
    }

    if let Some(sh) = &health.splunkd_health {
        output.push_str("--- Splunkd Health ---\n");
        output.push_str(&format!("Overall Status: {}\n", sh.health));
        output.push_str("Features:\n");
        let mut features: Vec<_> = sh.features.iter().collect();
        features.sort_by_key(|(name, _)| *name);
        for (name, feature) in features {
            output.push_str(&format!(
                "  {}: {} (Status: {})\n",
                name, feature.health, feature.status
            ));
            for reason in &feature.reasons {
                output.push_str(&format!("    Reason: {}\n", reason));
            }
        }
        output.push('\n');
    }

    if let Some(usage) = &health.license_usage {
        output.push_str("--- License Usage ---\n");
        if usage.is_empty() {
            output.push_str("No license usage data available.\n");
        } else {
            for (i, u) in usage.iter().enumerate() {
                output.push_str(&format!(
                    "Stack {}:\n",
                    u.stack_id.as_deref().unwrap_or("N/A")
                ));
                let used_bytes = u.effective_used_bytes();
                output.push_str(&format!(
                    "  Used: {} MB / Quota: {} MB\n",
                    used_bytes / 1024 / 1024,
                    u.quota / 1024 / 1024
                ));
                if let Some(slaves) = u.slaves_breakdown()
                    && !slaves.is_empty()
                {
                    output.push_str("  Slave Usage:\n");
                    let mut slave_list: Vec<_> = slaves.iter().collect();
                    slave_list.sort_by_key(|(name, _)| *name);
                    for (name, bytes) in slave_list {
                        output.push_str(&format!("    {}: {} MB\n", name, bytes / 1024 / 1024));
                    }
                }
                if i < usage.len() - 1 {
                    output.push('\n');
                }
            }
        }
        output.push('\n');
    }

    if let Some(kv) = &health.kvstore_status {
        output.push_str("--- KVStore Status ---\n");
        output.push_str(&format!(
            "Member: {}:{} ({})",
            kv.current_member.host, kv.current_member.port, kv.current_member.status
        ));
        output.push_str(&format!("Replica Set: {}\n", kv.current_member.replica_set));
        output.push_str(&format!(
            "Oplog: Size {} MB / Used {:.2}%\n",
            kv.replication_status.oplog_size, kv.replication_status.oplog_used
        ));
        output.push('\n');
    }

    if let Some(lp) = &health.log_parsing_health {
        output.push_str("--- Log Parsing Health ---\n");
        output.push_str(&format!(
            "Status: {}\n",
            if lp.is_healthy {
                "Healthy"
            } else {
                "Unhealthy"
            }
        ));
        output.push_str(&format!("Total Errors: {}\n", lp.total_errors));
        output.push_str(&format!("Time Window: {}\n", lp.time_window));
        if !lp.errors.is_empty() {
            output.push_str("Recent Errors:\n");
            for err in &lp.errors {
                output.push_str(&format!(
                    "  [{}] {} ({}): {}\n",
                    err.time, err.sourcetype, err.log_level, err.message
                ));
            }
        }
    }

    Ok(output)
}

/// Format KVStore status as formatted text.
pub fn format_kvstore_status(status: &KvStoreStatus) -> Result<String> {
    let mut output = String::new();

    output.push_str("KVStore Status:\n");
    output.push_str(&format!(
        "  Current Member: {}:{}\n",
        status.current_member.host, status.current_member.port
    ));
    output.push_str(&format!("  Status: {}\n", status.current_member.status));
    output.push_str(&format!(
        "  Replica Set: {}\n",
        status.current_member.replica_set
    ));
    output.push_str(&format!(
        "  Oplog Size: {} MB\n",
        status.replication_status.oplog_size
    ));
    output.push_str(&format!(
        "  Oplog Used: {:.2}%\n",
        status.replication_status.oplog_used
    ));

    Ok(output)
}

/// Format diagnostic report as formatted text.
pub fn format_health_check_report(report: &DiagnosticReport) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str("========================================\n");
    output.push_str("      Splunk CLI Doctor Report\n");
    output.push_str("========================================\n\n");

    // Version and platform info
    output.push_str(&format!("CLI Version: {}\n", report.cli_version));
    output.push_str(&format!("Platform:    {}\n", report.os_arch));
    output.push_str(&format!("Timestamp:   {}\n\n", report.timestamp));

    // Configuration summary
    output.push_str("--- Configuration Summary ---\n");
    output.push_str(&format!(
        "Base URL:        {}\n",
        report.config_summary.base_url
    ));
    output.push_str(&format!(
        "Auth Strategy:   {}\n",
        report.config_summary.auth_strategy
    ));
    output.push_str(&format!(
        "Skip TLS Verify: {}\n",
        report.config_summary.skip_verify
    ));
    output.push_str(&format!(
        "Timeout:         {}s\n",
        report.config_summary.timeout_secs
    ));
    output.push_str(&format!(
        "Max Retries:     {}\n\n",
        report.config_summary.max_retries
    ));

    // Diagnostic checks
    output.push_str("--- Diagnostic Checks ---\n");
    for check in &report.checks {
        let status_icon = match check.status {
            crate::formatters::CheckStatus::Pass => "[PASS]",
            crate::formatters::CheckStatus::Fail => "[FAIL]",
            crate::formatters::CheckStatus::Warning => "[WARN]",
            crate::formatters::CheckStatus::Skipped => "[SKIP]",
        };
        output.push_str(&format!(
            "{} {}: {}\n",
            status_icon, check.name, check.message
        ));
    }

    // Partial errors (if any)
    if !report.partial_errors.is_empty() {
        output.push_str("\n--- Partial Errors ---\n");
        for (endpoint, error) in &report.partial_errors {
            output.push_str(&format!("  {}: {}\n", endpoint, error));
        }
    }

    // Server health details (if available)
    if let Some(health) = &report.health_output {
        output.push('\n');
        output.push_str(&format_health(health)?);
    }

    Ok(output)
}
