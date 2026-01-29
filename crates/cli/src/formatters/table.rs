//! Table formatter implementation.
//!
//! Responsibilities:
//! - Format resources as tab-separated tables.
//! - Provide paginated variants for interactive use.
//!
//! Does NOT handle:
//! - Other output formats.
//! - File I/O.

use crate::commands::list_all::ListAllOutput;
use crate::formatters::common::format_json_value;
use crate::formatters::{ClusterInfoOutput, Formatter, LicenseInfoOutput};
use anyhow::Result;
use splunk_client::models::LogEntry;
use splunk_client::{
    App, HealthCheckOutput, Index, KvStoreStatus, SavedSearch, SearchJobStatus, User,
};
use splunk_config::constants::DEFAULT_LICENSE_ALERT_PCT;
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// Pagination metadata for table output.
///
/// - `offset` is zero-based
/// - `page_size` is the requested page size
/// - `total` is optional; when absent, footer omits total/page-count
#[derive(Debug, Clone, Copy)]
pub struct Pagination {
    pub offset: usize,
    pub page_size: usize,
    pub total: Option<usize>,
}

/// Table formatter.
pub struct TableFormatter;

impl Formatter for TableFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        if results.is_empty() {
            return Ok("No results found.".to_string());
        }

        let mut output = String::new();

        // Get all unique keys from all results
        let mut all_keys: Vec<String> = Vec::new();
        for result in results {
            if let Some(obj) = result.as_object() {
                for key in obj.keys() {
                    if !all_keys.contains(key) {
                        all_keys.push(key.clone());
                    }
                }
            }
        }

        // Sort keys for consistent output
        all_keys.sort();

        // Print header
        output.push_str(&all_keys.join("\t"));
        output.push('\n');

        // Print rows
        for result in results {
            if let Some(obj) = result.as_object() {
                let row: Vec<String> = all_keys
                    .iter()
                    .map(|key| obj.get(key).map(format_json_value).unwrap_or_default())
                    .collect();
                output.push_str(&row.join("\t"));
                output.push('\n');
            }
        }

        Ok(output)
    }

    fn format_indexes(&self, indexes: &[Index], detailed: bool) -> Result<String> {
        let mut output = String::new();

        if indexes.is_empty() {
            return Ok("No indexes found.".to_string());
        }

        // Header
        if detailed {
            output.push_str("Name\tSize (MB)\tEvents\tMax Size (MB)\tRetention (s)\tHome Path\tCold Path\tThawed Path\n");
        } else {
            output.push_str("Name\tSize (MB)\tEvents\tMax Size (MB)\n");
        }

        for index in indexes {
            let max_size = index
                .max_total_data_size_mb
                .map(|v: u64| v.to_string())
                .unwrap_or_else(|| "N/A".to_string());

            if detailed {
                let retention = index
                    .frozen_time_period_in_secs
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                let home_path = index.home_path.as_deref().unwrap_or("N/A");
                let cold_path = index.cold_db_path.as_deref().unwrap_or("N/A");
                let thawed_path = index.thawed_path.as_deref().unwrap_or("N/A");
                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\t{}\t{}\t{}\n",
                    index.name,
                    index.current_db_size_mb,
                    index.total_event_count,
                    max_size,
                    retention,
                    home_path,
                    cold_path,
                    thawed_path
                ));
            } else {
                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\n",
                    index.name, index.current_db_size_mb, index.total_event_count, max_size
                ));
            }
        }

        Ok(output)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        let mut output = String::new();

        if jobs.is_empty() {
            return Ok("No jobs found.".to_string());
        }

        // Header
        output.push_str("SID\tDone\tProgress\tResults\tDuration\n");

        for job in jobs {
            output.push_str(&format!(
                "{}\t{}\t{:.1}%\t{}\t{:.2}s\n",
                job.sid,
                if job.is_done { "Y" } else { "N" },
                job.done_progress * 100.0,
                job.result_count,
                job.run_duration
            ));
        }

        Ok(output)
    }

    fn format_cluster_info(
        &self,
        cluster_info: &ClusterInfoOutput,
        detailed: bool,
    ) -> Result<String> {
        let mut output = format!(
            "Cluster Information:\n\
             ID: {}\n\
             Label: {}\n\
             Mode: {}\n\
             Manager URI: {}\n\
             Replication Factor: {}\n\
             Search Factor: {}\n\
             Status: {}\n",
            cluster_info.id,
            cluster_info.label.as_deref().unwrap_or("N/A"),
            cluster_info.mode,
            cluster_info.manager_uri.as_deref().unwrap_or("N/A"),
            cluster_info
                .replication_factor
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            cluster_info
                .search_factor
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            cluster_info.status.as_deref().unwrap_or("N/A")
        );

        if detailed && let Some(peers) = &cluster_info.peers {
            output.push_str(&format!("\nCluster Peers ({}):\n", peers.len()));
            for peer in peers {
                output.push_str(&format!(
                    "\n  Host: {}:{}\n\
                        ID: {}\n\
                        Status: {}\n\
                        State: {}\n",
                    peer.host, peer.port, peer.id, peer.status, peer.peer_state
                ));
                if let Some(label) = &peer.label {
                    output.push_str(&format!("    Label: {}\n", label));
                }
                if let Some(site) = &peer.site {
                    output.push_str(&format!("    Site: {}\n", site));
                }
                if peer.is_captain {
                    output.push_str("    Captain: Yes\n");
                }
            }
        }

        Ok(output)
    }

    fn format_health(&self, health: &HealthCheckOutput) -> Result<String> {
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

    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String> {
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

    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String> {
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

    fn format_logs(&self, logs: &[LogEntry]) -> Result<String> {
        let mut output = String::new();

        if logs.is_empty() {
            return Ok("No logs found.".to_string());
        }

        // Header
        output.push_str("Time\tLevel\tComponent\tMessage\n");

        for log in logs {
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                log.time, log.level, log.component, log.message
            ));
        }

        Ok(output)
    }

    fn format_users(&self, users: &[User]) -> Result<String> {
        let mut output = String::new();

        if users.is_empty() {
            output.push_str("No users found.\n");
            return Ok(output);
        }

        // Header
        output.push_str(&format!(
            "{:<20} {:<30} {:<15} {:<15}\n",
            "NAME", "REAL NAME", "TYPE", "ROLES"
        ));
        output.push_str(&format!(
            "{:<20} {:<30} {:<15} {:<15}\n",
            "====", "=========", "====", "====="
        ));

        // Rows
        for user in users {
            let name = &user.name;
            let realname = user.realname.as_deref().unwrap_or("-");
            let user_type = user.user_type.as_deref().unwrap_or("-");
            let roles = if user.roles.is_empty() {
                "-".to_string()
            } else {
                user.roles.join(", ")
            };

            output.push_str(&format!(
                "{:<20} {:<30} {:<15} {:<15}\n",
                name, realname, user_type, roles
            ));
        }

        Ok(output)
    }

    fn format_apps(&self, apps: &[App]) -> Result<String> {
        let mut output = String::new();

        if apps.is_empty() {
            output.push_str("No apps found.");
            return Ok(output);
        }

        // Header
        output.push_str(&format!(
            "{:<25} {:<20} {:<10} {:<10} {:<20}\n",
            "NAME", "LABEL", "VERSION", "DISABLED", "AUTHOR"
        ));
        output.push_str(&format!(
            "{:<25} {:<20} {:<10} {:<10} {:<20}\n",
            "=====", "=====", "=======", "========", "======="
        ));

        // Rows
        for app in apps {
            let label = app.label.as_deref().unwrap_or("-");
            let version = app.version.as_deref().unwrap_or("-");
            let author = app.author.as_deref().unwrap_or("-");

            output.push_str(&format!(
                "{:<25} {:<20} {:<10} {:<10} {:<20}\n",
                app.name, label, version, app.disabled, author
            ));
        }

        Ok(output)
    }

    fn format_app_info(&self, app: &App) -> Result<String> {
        let mut output = String::new();

        output.push_str("--- App Information ---\n");
        output.push_str(&format!("Name: {}\n", app.name));
        output.push_str(&format!(
            "Label: {}\n",
            app.label.as_deref().unwrap_or("N/A")
        ));
        output.push_str(&format!(
            "Version: {}\n",
            app.version.as_deref().unwrap_or("N/A")
        ));
        output.push_str(&format!("Disabled: {}\n", app.disabled));
        output.push_str(&format!(
            "Author: {}\n",
            app.author.as_deref().unwrap_or("N/A")
        ));
        if let Some(ref desc) = app.description {
            output.push_str(&format!("Description: {}\n", desc));
        }
        if let Some(configured) = app.is_configured {
            output.push_str(&format!("Configured: {}\n", configured));
        }
        if let Some(visible) = app.is_visible {
            output.push_str(&format!("Visible: {}\n", visible));
        }

        Ok(output)
    }

    fn format_list_all(&self, output: &ListAllOutput) -> Result<String> {
        let mut out = String::new();

        if output.resources.is_empty() {
            return Ok("No resources found.".to_string());
        }

        out.push_str(&format!("Timestamp: {}\n", output.timestamp));
        out.push('\n');

        let header = format!(
            "{:<20} {:<10} {:<15} {}",
            "Resource Type", "Count", "Status", "Error"
        );
        out.push_str(&header);
        out.push('\n');

        let separator = format!("{:<20} {:<10} {:<15} {}", "====", "=====", "=====", "=====");
        out.push_str(&separator);
        out.push('\n');

        for resource in &output.resources {
            let error = resource.error.as_deref().unwrap_or("");
            out.push_str(&format!(
                "{:<20} {:<10} {:<15} {}\n",
                resource.resource_type, resource.count, resource.status, error
            ));
        }

        Ok(out)
    }

    fn format_saved_searches(&self, searches: &[SavedSearch]) -> Result<String> {
        let mut output = String::new();

        if searches.is_empty() {
            output.push_str("No saved searches found.");
            return Ok(output);
        }

        output.push_str(&format!(
            "{:<30} {:<10} {:<40}\n",
            "NAME", "DISABLED", "DESCRIPTION"
        ));
        output.push_str(&format!(
            "{:<30} {:<10} {:<40}\n",
            "====", "========", "==========="
        ));

        for search in searches {
            let description = search.description.as_deref().unwrap_or("");
            let truncated_desc = if description.len() > 40 {
                format!("{}...", &description[..37])
            } else {
                description.to_string()
            };

            output.push_str(&format!(
                "{:<30} {:<10} {:<40}\n",
                search.name,
                if search.disabled { "Yes" } else { "No" },
                truncated_desc
            ));
        }

        Ok(output)
    }

    fn format_saved_search_info(&self, search: &SavedSearch) -> Result<String> {
        let mut output = String::new();

        output.push_str("--- Saved Search Information ---\n");
        output.push_str(&format!("Name: {}\n", search.name));
        output.push_str(&format!("Disabled: {}\n", search.disabled));
        output.push_str(&format!("Search Query:\n{}\n", search.search));
        if let Some(ref desc) = search.description {
            output.push_str(&format!("Description: {}\n", desc));
        }

        Ok(output)
    }

    fn format_job_details(&self, job: &SearchJobStatus) -> Result<String> {
        let mut output = String::new();

        output.push_str("--- Job Details ---\n");
        output.push_str(&format!("SID: {}\n", job.sid));

        // Status with progress
        let status_text = if job.is_done {
            "Done"
        } else if job.done_progress > 0.0 {
            &format!("Running ({:.0}%)", job.done_progress * 100.0)
        } else {
            "Running"
        };
        output.push_str(&format!("Status: {}\n", status_text));

        // Duration
        output.push_str(&format!("Duration: {:.2} seconds\n", job.run_duration));

        // Counts
        output.push_str(&format!("Event Count: {}\n", job.event_count));
        output.push_str(&format!("Scan Count: {}\n", job.scan_count));
        output.push_str(&format!("Result Count: {}\n", job.result_count));

        // Disk usage
        output.push_str(&format!("Disk Usage: {} MB\n", job.disk_usage));

        // Optional fields
        output.push_str(&format!(
            "Priority: {}\n",
            job.priority.map_or("N/A".to_string(), |p| p.to_string())
        ));
        output.push_str(&format!(
            "Label: {}\n",
            job.label.as_deref().unwrap_or("N/A")
        ));
        output.push_str(&format!(
            "Cursor Time: {}\n",
            job.cursor_time.as_deref().unwrap_or("N/A")
        ));
        output.push_str(&format!(
            "Finalized: {}\n",
            if job.is_finalized { "Yes" } else { "No" }
        ));

        Ok(output)
    }

    fn format_profile(&self, profile_name: &str, profile: &ProfileConfig) -> Result<String> {
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

    fn format_profiles(&self, profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
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
}

impl TableFormatter {
    /// Table-only formatter for indexes with pagination footer.
    ///
    /// NOTE: This does not attempt to discover a server-side total for indexes (not exposed by the
    /// current client API return type). Footer omits total/page-count when `total` is None.
    pub fn format_indexes_paginated(
        &self,
        indexes: &[Index],
        detailed: bool,
        pagination: Pagination,
    ) -> Result<String> {
        if indexes.is_empty() {
            if pagination.offset > 0 {
                return Ok(format!(
                    "No indexes found for offset {}.",
                    pagination.offset
                ));
            }
            return Ok("No indexes found.".to_string());
        }

        // Reuse existing table rendering, then append footer.
        let mut output = self.format_indexes(indexes, detailed)?;

        if let Some(footer) = build_pagination_footer(pagination, indexes.len()) {
            output.push('\n');
            output.push_str(&footer);
            output.push('\n');
        }

        Ok(output)
    }

    /// Table-only formatter for cluster output with pagination footer (peers only).
    #[allow(clippy::collapsible_if)]
    pub fn format_cluster_info_paginated(
        &self,
        cluster_info: &ClusterInfoOutput,
        detailed: bool,
        peers_pagination: Option<Pagination>,
    ) -> Result<String> {
        let mut output = format!(
            "Cluster Information:\n\
             ID: {}\n\
             Label: {}\n\
             Mode: {}\n\
             Manager URI: {}\n\
             Replication Factor: {}\n\
             Search Factor: {}\n\
             Status: {}\n",
            cluster_info.id,
            cluster_info.label.as_deref().unwrap_or("N/A"),
            cluster_info.mode,
            cluster_info.manager_uri.as_deref().unwrap_or("N/A"),
            cluster_info
                .replication_factor
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            cluster_info
                .search_factor
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            cluster_info.status.as_deref().unwrap_or("N/A")
        );

        if detailed {
            if let Some(peers) = &cluster_info.peers {
                output.push_str("\nCluster Peers:\n");

                if peers.is_empty() {
                    // Offset out of range is especially important to explain in table output.
                    if let Some(p) = peers_pagination
                        && let Some(total) = p.total
                        && total > 0
                        && p.offset >= total
                    {
                        output.push_str(&format!(
                            "  No peers found for offset {} (total {}).\n",
                            p.offset, total
                        ));
                    } else {
                        output.push_str("  No peers found.\n");
                    }
                } else {
                    for peer in peers {
                        output.push_str(&format!(
                            "\n  Host: {}:{}\n\
                                ID: {}\n\
                                Status: {}\n\
                                State: {}\n",
                            peer.host, peer.port, peer.id, peer.status, peer.peer_state
                        ));
                        if let Some(label) = &peer.label {
                            output.push_str(&format!("    Label: {}\n", label));
                        }
                        if let Some(site) = &peer.site {
                            output.push_str(&format!("    Site: {}\n", site));
                        }
                        if peer.is_captain {
                            output.push_str("    Captain: Yes\n");
                        }
                    }
                }

                if let Some(p) = peers_pagination
                    && let Some(footer) = build_pagination_footer(p, peers.len())
                {
                    output.push('\n');
                    output.push_str(&footer);
                    output.push('\n');
                }
            }
        }

        Ok(output)
    }
}

/// Build a pagination footer string.
///
/// - `offset` is zero-based
/// - `page_size` is the requested page size
/// - `total` is optional; when absent, footer omits total/page-count
pub fn build_pagination_footer(p: Pagination, shown: usize) -> Option<String> {
    if p.page_size == 0 {
        // Avoid division by zero; caller should validate for client-side pagination.
        return None;
    }

    // If nothing is shown, caller should usually emit a friendlier message.
    if shown == 0 {
        if let Some(total) = p.total {
            if total == 0 {
                return Some("No results.".to_string());
            }
            if p.offset >= total {
                return Some(format!(
                    "Showing 0 of {} (offset {} out of range)",
                    total, p.offset
                ));
            }
            return Some(format!("Showing 0 of {}", total));
        }
        return Some("No results.".to_string());
    }

    let start = p.offset.saturating_add(1);
    let end = p.offset.saturating_add(shown);
    let page = (p.offset / p.page_size).saturating_add(1);

    if let Some(total) = p.total {
        let total_pages: usize = if total == 0 {
            0
        } else {
            (total.saturating_add(p.page_size).saturating_sub(1)) / p.page_size
        };
        Some(format!(
            "Showing {}-{} of {} (page {} of {})",
            start, end, total, page, total_pages
        ))
    } else {
        Some(format!("Showing {}-{} (page {})", start, end, page))
    }
}
