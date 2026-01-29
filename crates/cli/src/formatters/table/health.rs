//! Health check table formatter.
//!
//! Responsibilities:
//! - Format health check results and KVStore status as formatted text.
//!
//! Does NOT handle:
//! - Other resource types.

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
