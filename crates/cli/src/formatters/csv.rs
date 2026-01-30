//! CSV formatter implementation.
//!
//! Responsibilities:
//! - Format resources as RFC 4180 compliant CSV.
//! - Flatten nested JSON structures for tabular output.
//!
//! Does NOT handle:
//! - Other output formats.
//! - Table-style pagination.

use crate::commands::list_all::ListAllOutput;
use crate::formatters::common::{escape_csv, flatten_json_object, get_all_flattened_keys};
use crate::formatters::{ClusterInfoOutput, Formatter, LicenseInfoOutput};
use anyhow::Result;
use splunk_client::models::LogEntry;
use splunk_client::models::SearchPeer;
use splunk_client::{
    App, Forwarder, HealthCheckOutput, Index, KvStoreStatus, SavedSearch, SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// CSV formatter.
pub struct CsvFormatter;

impl Formatter for CsvFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        if results.is_empty() {
            return Ok(String::new());
        }

        let mut output = String::new();

        // Get all unique flattened keys from all results (sorted)
        let all_keys = get_all_flattened_keys(results);

        // Print header (escaped)
        let header: Vec<String> = all_keys.iter().map(|k| escape_csv(k)).collect();
        output.push_str(&header.join(","));
        output.push('\n');

        // Print rows with flattened values
        for result in results {
            let mut flat = std::collections::BTreeMap::new();
            flatten_json_object(result, "", &mut flat);

            let row: Vec<String> = all_keys
                .iter()
                .map(|key| {
                    let value = flat.get(key).cloned().unwrap_or_default();
                    escape_csv(&value)
                })
                .collect();
            output.push_str(&row.join(","));
            output.push('\n');
        }

        Ok(output)
    }

    fn format_indexes(&self, indexes: &[Index], detailed: bool) -> Result<String> {
        let mut output = String::new();

        if indexes.is_empty() {
            return Ok(String::new());
        }

        // Header (escaped)
        output.push_str(&escape_csv("Name"));
        output.push(',');
        output.push_str(&escape_csv("SizeMB"));
        output.push(',');
        output.push_str(&escape_csv("Events"));
        output.push(',');
        output.push_str(&escape_csv("MaxSizeMB"));
        if detailed {
            output.push(',');
            output.push_str(&escape_csv("RetentionSecs"));
            output.push(',');
            output.push_str(&escape_csv("HomePath"));
            output.push(',');
            output.push_str(&escape_csv("ColdPath"));
            output.push(',');
            output.push_str(&escape_csv("ThawedPath"));
        }
        output.push('\n');

        for index in indexes {
            let max_size = index
                .max_total_data_size_mb
                .map(|v: u64| v.to_string())
                .unwrap_or_else(|| "N/A".to_string());
            output.push_str(&escape_csv(&index.name));
            output.push(',');
            output.push_str(&escape_csv(&index.current_db_size_mb.to_string()));
            output.push(',');
            output.push_str(&escape_csv(&index.total_event_count.to_string()));
            output.push(',');
            output.push_str(&escape_csv(&max_size));
            if detailed {
                let retention = index
                    .frozen_time_period_in_secs
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "N/A".to_string());
                let home_path = index.home_path.as_deref().unwrap_or("N/A");
                let cold_path = index.cold_db_path.as_deref().unwrap_or("N/A");
                let thawed_path = index.thawed_path.as_deref().unwrap_or("N/A");
                output.push(',');
                output.push_str(&escape_csv(&retention));
                output.push(',');
                output.push_str(&escape_csv(home_path));
                output.push(',');
                output.push_str(&escape_csv(cold_path));
                output.push(',');
                output.push_str(&escape_csv(thawed_path));
            }
            output.push('\n');
        }

        Ok(output)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        let mut output = String::new();

        if jobs.is_empty() {
            return Ok(String::new());
        }

        // Header (escaped)
        output.push_str(&escape_csv("SID"));
        output.push(',');
        output.push_str(&escape_csv("Done"));
        output.push(',');
        output.push_str(&escape_csv("Progress"));
        output.push(',');
        output.push_str(&escape_csv("Results"));
        output.push(',');
        output.push_str(&escape_csv("Duration"));
        output.push('\n');

        for job in jobs {
            output.push_str(&escape_csv(&job.sid));
            output.push(',');
            output.push_str(&escape_csv(if job.is_done { "Y" } else { "N" }));
            output.push(',');
            output.push_str(&escape_csv(&format!("{:.1}", job.done_progress * 100.0)));
            output.push(',');
            output.push_str(&escape_csv(&job.result_count.to_string()));
            output.push(',');
            output.push_str(&escape_csv(&format!("{:.2}", job.run_duration)));
            output.push('\n');
        }

        Ok(output)
    }

    fn format_cluster_info(
        &self,
        cluster_info: &ClusterInfoOutput,
        detailed: bool,
    ) -> Result<String> {
        let mut output = String::new();

        // Cluster info row
        let fields = [
            escape_csv("ClusterInfo"),
            escape_csv(&cluster_info.id),
            escape_csv(cluster_info.label.as_deref().unwrap_or("N/A")),
            escape_csv(&cluster_info.mode),
            escape_csv(cluster_info.manager_uri.as_deref().unwrap_or("N/A")),
            escape_csv(
                &cluster_info
                    .replication_factor
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
            ),
            escape_csv(
                &cluster_info
                    .search_factor
                    .map(|v| v.to_string())
                    .unwrap_or_else(|| "N/A".to_string()),
            ),
        ];
        output.push_str(&fields.join(","));
        output.push('\n');

        // Peers rows (if detailed)
        if detailed && let Some(peers) = &cluster_info.peers {
            for peer in peers {
                let peer_fields = [
                    escape_csv("Peer"),
                    escape_csv(&format!("{}:{}", peer.host, peer.port)),
                    escape_csv(&peer.id),
                    escape_csv(&peer.status),
                    escape_csv(&peer.peer_state),
                    escape_csv(peer.label.as_deref().unwrap_or("N/A")),
                    escape_csv(peer.site.as_deref().unwrap_or("N/A")),
                    escape_csv(if peer.is_captain { "Yes" } else { "No" }),
                ];
                output.push_str(&peer_fields.join(","));
                output.push('\n');
            }
        }

        Ok(output)
    }

    fn format_health(&self, health: &HealthCheckOutput) -> Result<String> {
        let mut output = String::new();

        // Header
        let header = [
            "server_name",
            "version",
            "health_status",
            "license_used_mb",
            "license_quota_mb",
            "kvstore_status",
            "log_parsing_healthy",
            "log_parsing_errors",
        ];
        let escaped_header: Vec<String> = header.iter().map(|h| escape_csv(h)).collect();
        output.push_str(&escaped_header.join(","));
        output.push('\n');

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

        let row = [
            escape_csv(server_name),
            escape_csv(version),
            escape_csv(health_status),
            escape_csv(&used),
            escape_csv(&quota),
            escape_csv(kv_status),
            escape_csv(parsing_healthy),
            escape_csv(&parsing_errors),
        ];
        output.push_str(&row.join(","));
        output.push('\n');

        Ok(output)
    }

    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String> {
        let mut output = String::new();

        // Header
        let header = [
            "host",
            "port",
            "status",
            "replica_set",
            "oplog_size_mb",
            "oplog_used_percent",
        ];
        let escaped_header: Vec<String> = header.iter().map(|h| escape_csv(h)).collect();
        output.push_str(&escaped_header.join(","));
        output.push('\n');

        // Data row
        let row = [
            escape_csv(&status.current_member.host),
            escape_csv(&status.current_member.port.to_string()),
            escape_csv(&status.current_member.status),
            escape_csv(&status.current_member.replica_set),
            escape_csv(&status.replication_status.oplog_size.to_string()),
            escape_csv(&status.replication_status.oplog_used.to_string()),
        ];
        output.push_str(&row.join(","));
        output.push('\n');

        Ok(output)
    }

    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String> {
        let mut output = String::new();

        // Header
        output.push_str("Type,Name,StackID,UsedMB,QuotaMB,PctUsed,Label,Type_Name,Description\n");

        // Usage
        for u in &license.usage {
            let used_bytes = u.effective_used_bytes();
            let pct = if u.quota > 0 {
                (used_bytes as f64 / u.quota as f64) * 100.0
            } else {
                0.0
            };
            output.push_str(&format!(
                "Usage,{},{},{},{},{:.2},,, \n",
                escape_csv(&u.name),
                escape_csv(u.stack_id.as_deref().unwrap_or("N/A")),
                used_bytes / 1024 / 1024,
                u.quota / 1024 / 1024,
                pct
            ));
        }

        // Pools
        for p in &license.pools {
            let quota_mb = p
                .quota
                .parse::<u64>()
                .ok()
                .map(|q| (q / 1024 / 1024).to_string())
                .unwrap_or_else(|| p.quota.clone());
            output.push_str(&format!(
                "Pool,{},{},{},{},,,{}\n",
                escape_csv(&p.name),
                escape_csv(&p.stack_id),
                p.used_bytes / 1024 / 1024,
                escape_csv(&quota_mb),
                escape_csv(p.description.as_deref().unwrap_or("N/A"))
            ));
        }

        // Stacks
        for s in &license.stacks {
            output.push_str(&format!(
                "Stack,{},,0,{},,{},{}\n",
                escape_csv(&s.name),
                s.quota / 1024 / 1024,
                escape_csv(&s.label),
                escape_csv(&s.type_name)
            ));
        }

        Ok(output)
    }

    fn format_logs(&self, logs: &[LogEntry]) -> Result<String> {
        let mut output = String::new();

        if logs.is_empty() {
            return Ok(String::new());
        }

        // Header
        output.push_str("Time,Level,Component,Message\n");

        for log in logs {
            output.push_str(&format!(
                "{},{},{},{}\n",
                escape_csv(&log.time),
                escape_csv(&log.level),
                escape_csv(&log.component),
                escape_csv(&log.message)
            ));
        }

        Ok(output)
    }

    fn format_logs_streaming(&self, logs: &[LogEntry], is_first: bool) -> Result<String> {
        let mut output = String::new();

        if logs.is_empty() {
            return Ok(output);
        }

        if is_first {
            output.push_str("Time,Level,Component,Message\n");
        }

        for log in logs {
            output.push_str(&format!(
                "{},{},{},{}\n",
                escape_csv(&log.time),
                escape_csv(&log.level),
                escape_csv(&log.component),
                escape_csv(&log.message)
            ));
        }

        Ok(output)
    }

    fn format_users(&self, users: &[User]) -> Result<String> {
        let mut output = String::new();

        // Header
        output.push_str("name,realname,user_type,default_app,roles,last_successful_login\n");

        for user in users {
            let realname = user.realname.as_deref().unwrap_or("");
            let user_type = user.user_type.as_deref().unwrap_or("");
            let default_app = user.default_app.as_deref().unwrap_or("");
            let roles = user.roles.join(";");
            let last_login = user.last_successful_login.unwrap_or(0);

            output.push_str(&format!(
                "{},{},{},{},{},{}\n",
                escape_csv(&user.name),
                escape_csv(realname),
                escape_csv(user_type),
                escape_csv(default_app),
                roles,
                last_login
            ));
        }

        Ok(output)
    }

    fn format_list_all(&self, output: &ListAllOutput) -> Result<String> {
        let mut csv = String::new();

        csv.push_str("timestamp,resource_type,count,status,error\n");

        for resource in &output.resources {
            let error = resource.error.as_deref().unwrap_or("");
            csv.push_str(&format!(
                "{},{},{},{},{}\n",
                escape_csv(&output.timestamp),
                escape_csv(&resource.resource_type),
                resource.count,
                escape_csv(&resource.status),
                escape_csv(error)
            ));
        }

        Ok(csv)
    }

    fn format_saved_searches(&self, searches: &[SavedSearch]) -> Result<String> {
        let mut output = String::new();

        output.push_str("name,disabled,description\n");

        for search in searches {
            let description = search.description.as_deref().unwrap_or("");
            output.push_str(&format!(
                "{},{},{}\n",
                escape_csv(&search.name),
                search.disabled,
                escape_csv(description)
            ));
        }

        Ok(output)
    }

    fn format_saved_search_info(&self, search: &SavedSearch) -> Result<String> {
        let mut output = String::new();

        output.push_str("name,disabled,search,description\n");
        let description = search.description.as_deref().unwrap_or("");
        output.push_str(&format!(
            "{},{},{},{}\n",
            escape_csv(&search.name),
            search.disabled,
            escape_csv(&search.search),
            escape_csv(description)
        ));

        Ok(output)
    }

    fn format_job_details(&self, job: &SearchJobStatus) -> Result<String> {
        let mut csv = String::new();

        // Header
        csv.push_str("sid,is_done,is_finalized,done_progress,run_duration,cursor_time,scan_count,event_count,result_count,disk_usage,priority,label\n");

        // Data row
        let priority = job.priority.map_or("".to_string(), |p| p.to_string());
        let cursor_time = job.cursor_time.as_deref().unwrap_or("");
        let label = job.label.as_deref().unwrap_or("");

        csv.push_str(&format!(
            "{},{},{},{},{},{},{},{},{},{},{},{}\n",
            escape_csv(&job.sid),
            job.is_done,
            job.is_finalized,
            job.done_progress,
            job.run_duration,
            escape_csv(cursor_time),
            job.scan_count,
            job.event_count,
            job.result_count,
            job.disk_usage,
            escape_csv(&priority),
            escape_csv(label)
        ));

        Ok(csv)
    }

    fn format_profile(&self, profile_name: &str, profile: &ProfileConfig) -> Result<String> {
        let mut csv = String::new();

        csv.push_str("field,value\n");

        csv.push_str(&format!(
            "{},{}\n",
            escape_csv("Profile Name"),
            escape_csv(profile_name)
        ));

        let base_url = profile.base_url.as_deref().unwrap_or("(not set)");
        csv.push_str(&format!(
            "{},{}\n",
            escape_csv("Base URL"),
            escape_csv(base_url)
        ));

        let username = profile.username.as_deref().unwrap_or("(not set)");
        csv.push_str(&format!(
            "{},{}\n",
            escape_csv("Username"),
            escape_csv(username)
        ));

        let password_display = match &profile.password {
            Some(_) => "****",
            None => "(not set)",
        };
        csv.push_str(&format!(
            "{},{}\n",
            escape_csv("Password"),
            escape_csv(password_display)
        ));

        let token_display = match &profile.api_token {
            Some(_) => "****",
            None => "(not set)",
        };
        csv.push_str(&format!(
            "{},{}\n",
            escape_csv("API Token"),
            escape_csv(token_display)
        ));

        let skip_verify = profile
            .skip_verify
            .map_or("(not set)".to_string(), |b| b.to_string());
        csv.push_str(&format!(
            "{},{}\n",
            escape_csv("Skip TLS Verify"),
            escape_csv(&skip_verify)
        ));

        let timeout = profile
            .timeout_seconds
            .map_or("(not set)".to_string(), |t| t.to_string());
        csv.push_str(&format!(
            "{},{}\n",
            escape_csv("Timeout (sec)"),
            escape_csv(&timeout)
        ));

        let max_retries = profile
            .max_retries
            .map_or("(not set)".to_string(), |r| r.to_string());
        csv.push_str(&format!(
            "{},{}",
            escape_csv("Max Retries"),
            escape_csv(&max_retries)
        ));

        Ok(csv)
    }

    fn format_profiles(&self, profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
        if profiles.is_empty() {
            return Ok(String::new());
        }

        let mut csv =
            String::from("profile,base_url,username,skip_verify,timeout_seconds,max_retries\n");
        for (name, profile) in profiles {
            let fields = [
                escape_csv(name),
                escape_csv(profile.base_url.as_deref().unwrap_or("")),
                escape_csv(profile.username.as_deref().unwrap_or("")),
                escape_csv(
                    &profile
                        .skip_verify
                        .map_or("".to_string(), |b| b.to_string()),
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
            ];
            csv.push_str(&fields.join(","));
            csv.push('\n');
        }
        Ok(csv)
    }

    fn format_apps(&self, apps: &[App]) -> Result<String> {
        let mut output = String::new();

        output.push_str("name,label,version,disabled,author\n");

        for app in apps {
            let label = app.label.as_deref().unwrap_or("");
            let version = app.version.as_deref().unwrap_or("");
            let author = app.author.as_deref().unwrap_or("");

            output.push_str(&format!(
                "{},{},{},{},{}\n",
                escape_csv(&app.name),
                escape_csv(label),
                escape_csv(version),
                app.disabled,
                escape_csv(author)
            ));
        }

        Ok(output)
    }

    fn format_app_info(&self, app: &App) -> Result<String> {
        let mut output = String::new();

        output.push_str("name,label,version,disabled,author,description\n");
        output.push_str(&format!(
            "{},{},{},{},{},{}\n",
            escape_csv(&app.name),
            escape_csv(app.label.as_deref().unwrap_or("")),
            escape_csv(app.version.as_deref().unwrap_or("")),
            app.disabled,
            escape_csv(app.author.as_deref().unwrap_or("")),
            escape_csv(app.description.as_deref().unwrap_or(""))
        ));

        Ok(output)
    }

    fn format_forwarders(&self, forwarders: &[Forwarder], detailed: bool) -> Result<String> {
        let mut output = String::new();

        // Header
        output.push_str("name,hostname,client_name,ip_address,version,last_phone");
        if detailed {
            output.push_str(",utsname,repository_location,server_classes");
        }
        output.push('\n');

        for forwarder in forwarders {
            let hostname = forwarder.hostname.as_deref().unwrap_or("");
            let client_name = forwarder.client_name.as_deref().unwrap_or("");
            let ip = forwarder.ip_address.as_deref().unwrap_or("");
            let version = forwarder.version.as_deref().unwrap_or("");
            let last_phone = forwarder.last_phone.as_deref().unwrap_or("");

            output.push_str(&format!(
                "{},{},{},{},{},{}",
                escape_csv(&forwarder.name),
                escape_csv(hostname),
                escape_csv(client_name),
                escape_csv(ip),
                escape_csv(version),
                escape_csv(last_phone)
            ));

            if detailed {
                let utsname = forwarder.utsname.as_deref().unwrap_or("");
                let repo_loc = forwarder.repository_location.as_deref().unwrap_or("");
                let server_classes = forwarder
                    .server_classes
                    .as_ref()
                    .map(|sc| sc.join(";"))
                    .unwrap_or_default();
                output.push_str(&format!(
                    ",{},{},{}",
                    escape_csv(utsname),
                    escape_csv(repo_loc),
                    escape_csv(&server_classes)
                ));
            }

            output.push('\n');
        }

        Ok(output)
    }

    fn format_search_peers(&self, peers: &[SearchPeer], detailed: bool) -> Result<String> {
        let mut output = String::new();

        // Header
        output.push_str("name,host,port,status,version");
        if detailed {
            output.push_str(",guid,last_connected,disabled");
        }
        output.push('\n');

        for peer in peers {
            let version = peer.version.as_deref().unwrap_or("");

            output.push_str(&format!(
                "{},{},{},{},{}",
                escape_csv(&peer.name),
                escape_csv(&peer.host),
                peer.port,
                escape_csv(&peer.status),
                escape_csv(version)
            ));

            if detailed {
                let guid = peer.guid.as_deref().unwrap_or("");
                let last_connected = peer.last_connected.as_deref().unwrap_or("");
                let disabled = peer
                    .disabled
                    .map(|d| if d { "true" } else { "false" })
                    .unwrap_or("");
                output.push_str(&format!(
                    ",{},{},{}",
                    escape_csv(guid),
                    escape_csv(last_connected),
                    escape_csv(disabled)
                ));
            }

            output.push('\n');
        }

        Ok(output)
    }
}
