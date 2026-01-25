//! Output formatters for CLI commands.
//!
//! Provides multiple output formats: JSON, Table, CSV, and XML.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use splunk_client::models::LogEntry;
use splunk_client::{
    ClusterPeer, Index, KvStoreStatus, LicensePool, LicenseStack, LicenseUsage, SearchJobStatus,
    User,
};

pub use splunk_client::HealthCheckOutput;

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Table,
    Csv,
    Xml,
}

impl OutputFormat {
    /// Parse from string.
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "table" => Ok(OutputFormat::Table),
            "csv" => Ok(OutputFormat::Csv),
            "xml" => Ok(OutputFormat::Xml),
            _ => anyhow::bail!(
                "Invalid output format: {}. Valid options: json, table, csv, xml",
                s
            ),
        }
    }
}

/// License information output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInfoOutput {
    pub usage: Vec<LicenseUsage>,
    pub pools: Vec<LicensePool>,
    pub stacks: Vec<LicenseStack>,
}

/// Formatter trait for different output types.
pub trait Formatter {
    /// Format search results.
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String>;

    /// Format indexes list.
    fn format_indexes(&self, indexes: &[Index], detailed: bool) -> Result<String>;

    /// Format jobs list.
    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String>;

    /// Format cluster info.
    fn format_cluster_info(
        &self,
        cluster_info: &ClusterInfoOutput,
        detailed: bool,
    ) -> Result<String>;

    /// Format health check results.
    fn format_health(&self, health: &HealthCheckOutput) -> Result<String>;

    /// Format KVStore status.
    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String>;

    /// Format license information.
    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String>;

    /// Format internal logs.
    fn format_logs(&self, logs: &[LogEntry]) -> Result<String>;

    /// Format users list.
    fn format_users(&self, users: &[User]) -> Result<String>;

    /// Format list-all unified resource overview.
    fn format_list_all(&self, output: &crate::commands::list_all::ListAllOutput) -> Result<String>;
}

/// Cluster peer output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterPeerOutput {
    pub host: String,
    pub port: u32,
    pub id: String,
    pub status: String,
    pub peer_state: String,
    pub label: Option<String>,
    pub site: Option<String>,
    pub is_captain: bool,
}

impl From<ClusterPeer> for ClusterPeerOutput {
    fn from(peer: ClusterPeer) -> Self {
        Self {
            host: peer.host,
            port: peer.port,
            id: peer.id,
            status: peer.status,
            peer_state: peer.peer_state,
            label: peer.label,
            site: peer.site,
            is_captain: peer.is_captain.unwrap_or(false),
        }
    }
}

/// Cluster info output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterInfoOutput {
    pub id: String,
    pub label: Option<String>,
    pub mode: String,
    pub manager_uri: Option<String>,
    pub replication_factor: Option<u32>,
    pub search_factor: Option<u32>,
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peers: Option<Vec<ClusterPeerOutput>>,
}

/// JSON formatter.
pub struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        Ok(serde_json::to_string_pretty(results)?)
    }

    fn format_indexes(&self, indexes: &[Index], _detailed: bool) -> Result<String> {
        // JSON formatter always outputs full Index struct regardless of detailed flag
        Ok(serde_json::to_string_pretty(indexes)?)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        Ok(serde_json::to_string_pretty(jobs)?)
    }

    fn format_cluster_info(
        &self,
        cluster_info: &ClusterInfoOutput,
        _detailed: bool,
    ) -> Result<String> {
        Ok(serde_json::to_string_pretty(cluster_info)?)
    }

    fn format_health(&self, health: &HealthCheckOutput) -> Result<String> {
        Ok(serde_json::to_string_pretty(health)?)
    }

    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String> {
        Ok(serde_json::to_string_pretty(status)?)
    }

    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String> {
        Ok(serde_json::to_string_pretty(license)?)
    }

    fn format_logs(&self, logs: &[LogEntry]) -> Result<String> {
        Ok(serde_json::to_string_pretty(logs)?)
    }

    fn format_users(&self, users: &[User]) -> Result<String> {
        Ok(serde_json::to_string_pretty(users)?)
    }

    fn format_list_all(&self, output: &crate::commands::list_all::ListAllOutput) -> Result<String> {
        Ok(serde_json::to_string_pretty(output)?)
    }
}

/// Flatten a JSON object into a map of dot-notation keys to string values.
///
/// # Arguments
/// * `value` - The JSON value to flatten
/// * `prefix` - The current key prefix (for nested recursion)
/// * `output` - The output map to populate
///
/// # Flattening Rules
/// - Primitive values (string, number, bool, null): stored as-is with string conversion
/// - Nested objects: keys are prefixed with parent key and dot (e.g., `user.name`)
/// - Arrays: each element gets indexed key (e.g., `tags.0`, `tags.1`)
/// - Nested arrays within objects: combined notation (e.g., `users.0.name`)
fn flatten_json_object(
    value: &serde_json::Value,
    prefix: &str,
    output: &mut std::collections::BTreeMap<String, String>,
) {
    match value {
        serde_json::Value::Null => {
            output.insert(prefix.to_string(), String::new());
        }
        serde_json::Value::Bool(b) => {
            output.insert(prefix.to_string(), b.to_string());
        }
        serde_json::Value::Number(n) => {
            output.insert(prefix.to_string(), n.to_string());
        }
        serde_json::Value::String(s) => {
            output.insert(prefix.to_string(), s.clone());
        }
        serde_json::Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                let new_key = format!("{}.{}", prefix, i);
                flatten_json_object(item, &new_key, output);
            }
        }
        serde_json::Value::Object(obj) => {
            for (key, val) in obj {
                let new_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_json_object(val, &new_key, output);
            }
        }
    }
}

/// Extract all flattened keys from a slice of JSON results.
///
/// Returns a sorted list of all unique dot-notation keys across all results.
fn get_all_flattened_keys(results: &[serde_json::Value]) -> Vec<String> {
    let mut all_keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for result in results {
        let mut flat = std::collections::BTreeMap::new();
        flatten_json_object(result, "", &mut flat);
        all_keys.extend(flat.into_keys());
    }
    all_keys.into_iter().collect()
}

/// Convert a JSON value to nested XML element(s).
///
/// Returns a vector of XML element strings. For primitive values, returns
/// a single element. For arrays and objects, returns multiple nested elements.
fn value_to_xml_elements(name: &str, value: &serde_json::Value, indent: &str) -> Vec<String> {
    match value {
        serde_json::Value::Null => {
            vec![format!(
                "{}<{}></{}>",
                indent,
                escape_xml(name),
                escape_xml(name)
            )]
        }
        serde_json::Value::Bool(b) => {
            vec![format!(
                "{}<{}>{}</{}>",
                indent,
                escape_xml(name),
                b,
                escape_xml(name)
            )]
        }
        serde_json::Value::Number(n) => {
            vec![format!(
                "{}<{}>{}</{}>",
                indent,
                escape_xml(name),
                n,
                escape_xml(name)
            )]
        }
        serde_json::Value::String(s) => {
            vec![format!(
                "{}<{}>{}</{}>",
                indent,
                escape_xml(name),
                escape_xml(s),
                escape_xml(name)
            )]
        }
        serde_json::Value::Array(arr) => {
            let mut elems = vec![format!("{}<{}>", indent, escape_xml(name))];
            for item in arr.iter() {
                let item_name = "item";
                elems.extend(value_to_xml_elements(
                    item_name,
                    item,
                    &format!("{}  ", indent),
                ));
            }
            elems.push(format!("{}</{}>", indent, escape_xml(name)));
            elems
        }
        serde_json::Value::Object(obj) => {
            let mut elems = vec![format!("{}<{}>", indent, escape_xml(name))];
            for (key, val) in obj {
                elems.extend(value_to_xml_elements(key, val, &format!("{}  ", indent)));
            }
            elems.push(format!("{}</{}>", indent, escape_xml(name)));
            elems
        }
    }
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
                    output.push_str(&format!(
                        "  Used: {} MB / Quota: {} MB\n",
                        u.used_bytes / 1024 / 1024,
                        u.quota / 1024 / 1024
                    ));
                    if let Some(slaves) = &u.slaves_usage_bytes
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
                "Member: {}:{} ({})\n",
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
                let used_mb = u.used_bytes / 1024 / 1024;
                let quota_mb = u.quota / 1024 / 1024;
                let pct = if u.quota > 0 {
                    (u.used_bytes as f64 / u.quota as f64) * 100.0
                } else {
                    0.0
                };
                let alert = if pct > 90.0 { "!" } else { "" };
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
                output.push_str(&format!(
                    "{}\t{}\t{}\t{}\t{}\n",
                    p.name,
                    p.stack_id,
                    p.used_bytes / 1024 / 1024,
                    p.quota / 1024 / 1024,
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

    fn format_list_all(&self, output: &crate::commands::list_all::ListAllOutput) -> Result<String> {
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
}

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
            let used: u64 = usage.iter().map(|u| u.used_bytes).sum();
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
            let pct = if u.quota > 0 {
                (u.used_bytes as f64 / u.quota as f64) * 100.0
            } else {
                0.0
            };
            output.push_str(&format!(
                "Usage,{},{},{},{},{:.2},,, \n",
                escape_csv(&u.name),
                escape_csv(u.stack_id.as_deref().unwrap_or("N/A")),
                u.used_bytes / 1024 / 1024,
                u.quota / 1024 / 1024,
                pct
            ));
        }

        // Pools
        for p in &license.pools {
            output.push_str(&format!(
                "Pool,{},{},{},{},,,,{}\n",
                escape_csv(&p.name),
                escape_csv(&p.stack_id),
                p.used_bytes / 1024 / 1024,
                p.quota / 1024 / 1024,
                escape_csv(p.description.as_deref().unwrap_or("N/A"))
            ));
        }

        // Stacks
        for s in &license.stacks {
            output.push_str(&format!(
                "Stack,{},,0,{},,{},{}, \n",
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

    fn format_list_all(&self, output: &crate::commands::list_all::ListAllOutput) -> Result<String> {
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
}

/// XML formatter.
pub struct XmlFormatter;

impl Formatter for XmlFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<results>\n");

        for result in results {
            // Use nested XML structure instead of flat fields
            let nested = value_to_xml_elements("result", result, "  ");
            xml.push_str(&nested.join("\n"));
            xml.push('\n');
        }

        xml.push_str("</results>");
        Ok(xml)
    }

    fn format_indexes(&self, indexes: &[Index], detailed: bool) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<indexes>\n");

        for index in indexes {
            xml.push_str("  <index>\n");
            xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&index.name)));
            xml.push_str(&format!(
                "    <sizeMB>{}</sizeMB>\n",
                index.current_db_size_mb
            ));
            xml.push_str(&format!(
                "    <events>{}</events>\n",
                index.total_event_count
            ));
            if let Some(max_size) = index.max_total_data_size_mb {
                xml.push_str(&format!("    <maxSizeMB>{}</maxSizeMB>\n", max_size));
            }
            // When detailed, include additional path and retention fields
            if detailed {
                if let Some(frozen_time) = index.frozen_time_period_in_secs {
                    xml.push_str(&format!(
                        "    <retentionSecs>{}</retentionSecs>\n",
                        frozen_time
                    ));
                }
                if let Some(home_path) = &index.home_path {
                    xml.push_str(&format!(
                        "    <homePath>{}</homePath>\n",
                        escape_xml(home_path)
                    ));
                }
                if let Some(cold_path) = &index.cold_db_path {
                    xml.push_str(&format!(
                        "    <coldPath>{}</coldPath>\n",
                        escape_xml(cold_path)
                    ));
                }
                if let Some(thawed_path) = &index.thawed_path {
                    xml.push_str(&format!(
                        "    <thawedPath>{}</thawedPath>\n",
                        escape_xml(thawed_path)
                    ));
                }
            }
            xml.push_str("  </index>\n");
        }

        xml.push_str("</indexes>");
        Ok(xml)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<jobs>\n");

        for job in jobs {
            xml.push_str("  <job>\n");
            xml.push_str(&format!("    <sid>{}</sid>\n", escape_xml(&job.sid)));
            xml.push_str(&format!("    <done>{}</done>\n", job.is_done));
            xml.push_str(&format!(
                "    <progress>{:.1}</progress>\n",
                job.done_progress * 100.0
            ));
            xml.push_str(&format!("    <results>{}</results>\n", job.result_count));
            xml.push_str(&format!(
                "    <duration>{:.2}</duration>\n",
                job.run_duration
            ));
            xml.push_str("  </job>\n");
        }

        xml.push_str("</jobs>");
        Ok(xml)
    }

    fn format_cluster_info(
        &self,
        cluster_info: &ClusterInfoOutput,
        detailed: bool,
    ) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<cluster>\n");
        xml.push_str(&format!("  <id>{}</id>\n", escape_xml(&cluster_info.id)));
        if let Some(label) = &cluster_info.label {
            xml.push_str(&format!("  <label>{}</label>\n", escape_xml(label)));
        }
        xml.push_str(&format!(
            "  <mode>{}</mode>\n",
            escape_xml(&cluster_info.mode)
        ));
        if let Some(manager_uri) = &cluster_info.manager_uri {
            xml.push_str(&format!(
                "  <managerUri>{}</managerUri>\n",
                escape_xml(manager_uri)
            ));
        }
        if let Some(replication_factor) = cluster_info.replication_factor {
            xml.push_str(&format!(
                "  <replicationFactor>{}</replicationFactor>\n",
                replication_factor
            ));
        }
        if let Some(search_factor) = cluster_info.search_factor {
            xml.push_str(&format!(
                "  <searchFactor>{}</searchFactor>\n",
                search_factor
            ));
        }
        if let Some(status) = &cluster_info.status {
            xml.push_str(&format!("  <status>{}</status>\n", escape_xml(status)));
        }

        // Add peers if detailed
        if detailed && let Some(peers) = &cluster_info.peers {
            xml.push_str("  <peers>\n");
            for peer in peers {
                xml.push_str("    <peer>\n");
                xml.push_str(&format!("      <host>{}</host>\n", escape_xml(&peer.host)));
                xml.push_str(&format!("      <port>{}</port>\n", peer.port));
                xml.push_str(&format!("      <id>{}</id>\n", escape_xml(&peer.id)));
                xml.push_str(&format!(
                    "      <status>{}</status>\n",
                    escape_xml(&peer.status)
                ));
                xml.push_str(&format!(
                    "      <peerState>{}</peerState>\n",
                    escape_xml(&peer.peer_state)
                ));
                if let Some(label) = &peer.label {
                    xml.push_str(&format!("      <label>{}</label>\n", escape_xml(label)));
                }
                if let Some(site) = &peer.site {
                    xml.push_str(&format!("      <site>{}</site>\n", escape_xml(site)));
                }
                xml.push_str(&format!(
                    "      <isCaptain>{}</isCaptain>\n",
                    peer.is_captain
                ));
                xml.push_str("    </peer>\n");
            }
            xml.push_str("  </peers>\n");
        }

        xml.push_str("</cluster>");
        Ok(xml)
    }

    fn format_health(&self, health: &HealthCheckOutput) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<health>\n");

        if let Some(info) = &health.server_info {
            xml.push_str("  <serverInfo>\n");
            xml.push_str(&format!(
                "    <serverName>{}</serverName>\n",
                escape_xml(&info.server_name)
            ));
            xml.push_str(&format!(
                "    <version>{}</version>\n",
                escape_xml(&info.version)
            ));
            xml.push_str(&format!("    <build>{}</build>\n", escape_xml(&info.build)));
            if let Some(os) = &info.os_name {
                xml.push_str(&format!("    <osName>{}</osName>\n", escape_xml(os)));
            }
            xml.push_str("    <roles>\n");
            for role in &info.server_roles {
                xml.push_str(&format!("      <role>{}</role>\n", escape_xml(role)));
            }
            xml.push_str("    </roles>\n");
            xml.push_str("  </serverInfo>\n");
        }

        if let Some(sh) = &health.splunkd_health {
            xml.push_str("  <splunkdHealth>\n");
            xml.push_str(&format!(
                "    <status>{}</status>\n",
                escape_xml(&sh.health)
            ));
            xml.push_str("    <features>\n");
            for (name, feature) in &sh.features {
                xml.push_str(&format!("      <feature name=\"{}\">\n", escape_xml(name)));
                xml.push_str(&format!(
                    "        <health>{}</health>\n",
                    escape_xml(&feature.health)
                ));
                xml.push_str(&format!(
                    "        <status>{}</status>\n",
                    escape_xml(&feature.status)
                ));
                xml.push_str("        <reasons>\n");
                for reason in &feature.reasons {
                    xml.push_str(&format!(
                        "          <reason>{}</reason>\n",
                        escape_xml(reason)
                    ));
                }
                xml.push_str("        </reasons>\n");
                xml.push_str("      </feature>\n");
            }
            xml.push_str("    </features>\n");
            xml.push_str("  </splunkdHealth>\n");
        }

        if let Some(usage) = &health.license_usage {
            xml.push_str("  <licenseUsage>\n");
            for u in usage {
                xml.push_str("    <stack>\n");
                if let Some(stack_id) = &u.stack_id {
                    xml.push_str(&format!(
                        "      <stackId>{}</stackId>\n",
                        escape_xml(stack_id)
                    ));
                }
                xml.push_str(&format!("      <usedBytes>{}</usedBytes>\n", u.used_bytes));
                xml.push_str(&format!("      <quotaBytes>{}</quotaBytes>\n", u.quota));
                if let Some(slaves) = &u.slaves_usage_bytes {
                    xml.push_str("      <slaves>\n");
                    for (name, bytes) in slaves {
                        xml.push_str(&format!(
                            "        <slave name=\"{}\">{}</slave>\n",
                            escape_xml(name),
                            bytes
                        ));
                    }
                    xml.push_str("      </slaves>\n");
                }
                xml.push_str("    </stack>\n");
            }
            xml.push_str("  </licenseUsage>\n");
        }

        if let Some(kv) = &health.kvstore_status {
            xml.push_str("  <kvstoreStatus>\n");
            xml.push_str("    <currentMember>\n");
            xml.push_str(&format!(
                "      <host>{}</host>\n",
                escape_xml(&kv.current_member.host)
            ));
            xml.push_str(&format!("      <port>{}</port>\n", kv.current_member.port));
            xml.push_str(&format!(
                "      <status>{}</status>\n",
                escape_xml(&kv.current_member.status)
            ));
            xml.push_str(&format!(
                "      <replicaSet>{}</replicaSet>\n",
                escape_xml(&kv.current_member.replica_set)
            ));
            xml.push_str("    </currentMember>\n");
            xml.push_str("    <replicationStatus>\n");
            xml.push_str(&format!(
                "      <oplogSize>{}</oplogSize>\n",
                kv.replication_status.oplog_size
            ));
            xml.push_str(&format!(
                "      <oplogUsed>{:.2}</oplogUsed>\n",
                kv.replication_status.oplog_used
            ));
            xml.push_str("    </replicationStatus>\n");
            xml.push_str("  </kvstoreStatus>\n");
        }

        if let Some(lp) = &health.log_parsing_health {
            xml.push_str("  <logParsingHealth>\n");
            xml.push_str(&format!("    <isHealthy>{}</isHealthy>\n", lp.is_healthy));
            xml.push_str(&format!(
                "    <totalErrors>{}</totalErrors>\n",
                lp.total_errors
            ));
            xml.push_str(&format!(
                "    <timeWindow>{}</timeWindow>\n",
                escape_xml(&lp.time_window)
            ));
            xml.push_str("    <errors>\n");
            for err in &lp.errors {
                xml.push_str("      <error>\n");
                xml.push_str(&format!("        <time>{}</time>\n", escape_xml(&err.time)));
                xml.push_str(&format!(
                    "        <sourcetype>{}</sourcetype>\n",
                    escape_xml(&err.sourcetype)
                ));
                xml.push_str(&format!(
                    "        <logLevel>{}</logLevel>\n",
                    escape_xml(&err.log_level)
                ));
                xml.push_str(&format!(
                    "        <message>{}</message>\n",
                    escape_xml(&err.message)
                ));
                xml.push_str("      </error>\n");
            }
            xml.push_str("    </errors>\n");
            xml.push_str("  </logParsingHealth>\n");
        }

        xml.push_str("</health>");
        Ok(xml)
    }

    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kvstoreStatus>\n");
        xml.push_str("  <currentMember>\n");
        xml.push_str(&format!(
            "    <host>{}</host>\n",
            escape_xml(&status.current_member.host)
        ));
        xml.push_str(&format!(
            "    <port>{}</port>\n",
            status.current_member.port
        ));
        xml.push_str(&format!(
            "    <status>{}</status>\n",
            escape_xml(&status.current_member.status)
        ));
        xml.push_str(&format!(
            "    <replicaSet>{}</replicaSet>\n",
            escape_xml(&status.current_member.replica_set)
        ));
        xml.push_str("  </currentMember>\n");
        xml.push_str("  <replicationStatus>\n");
        xml.push_str(&format!(
            "    <oplogSize>{}</oplogSize>\n",
            status.replication_status.oplog_size
        ));
        xml.push_str(&format!(
            "    <oplogUsed>{:.2}</oplogUsed>\n",
            status.replication_status.oplog_used
        ));
        xml.push_str("  </replicationStatus>\n");
        xml.push_str("</kvstoreStatus>");
        Ok(xml)
    }

    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<licenseInfo>\n");

        xml.push_str("  <usage>\n");
        for u in &license.usage {
            xml.push_str("    <entry>\n");
            xml.push_str(&format!("      <name>{}</name>\n", escape_xml(&u.name)));
            if let Some(stack_id) = &u.stack_id {
                xml.push_str(&format!(
                    "      <stackId>{}</stackId>\n",
                    escape_xml(stack_id)
                ));
            }
            xml.push_str(&format!("      <usedBytes>{}</usedBytes>\n", u.used_bytes));
            xml.push_str(&format!("      <quotaBytes>{}</quotaBytes>\n", u.quota));
            xml.push_str("    </entry>\n");
        }
        xml.push_str("  </usage>\n");

        xml.push_str("  <pools>\n");
        for p in &license.pools {
            xml.push_str("    <pool>\n");
            xml.push_str(&format!("      <name>{}</name>\n", escape_xml(&p.name)));
            xml.push_str(&format!(
                "      <stackId>{}</stackId>\n",
                escape_xml(&p.stack_id)
            ));
            xml.push_str(&format!("      <usedBytes>{}</usedBytes>\n", p.used_bytes));
            xml.push_str(&format!("      <quotaBytes>{}</quotaBytes>\n", p.quota));
            if let Some(desc) = &p.description {
                xml.push_str(&format!(
                    "      <description>{}</description>\n",
                    escape_xml(desc)
                ));
            }
            xml.push_str("    </pool>\n");
        }
        xml.push_str("  </pools>\n");

        xml.push_str("  <stacks>\n");
        for s in &license.stacks {
            xml.push_str("    <stack>\n");
            xml.push_str(&format!("      <name>{}</name>\n", escape_xml(&s.name)));
            xml.push_str(&format!("      <label>{}</label>\n", escape_xml(&s.label)));
            xml.push_str(&format!(
                "      <type>{}</type>\n",
                escape_xml(&s.type_name)
            ));
            xml.push_str(&format!("      <quotaBytes>{}</quotaBytes>\n", s.quota));
            xml.push_str("    </stack>\n");
        }
        xml.push_str("  </stacks>\n");

        xml.push_str("</licenseInfo>");
        Ok(xml)
    }

    fn format_logs(&self, logs: &[LogEntry]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<logs>\n");

        for log in logs {
            xml.push_str("  <log>\n");
            xml.push_str(&format!("    <time>{}</time>\n", escape_xml(&log.time)));
            xml.push_str(&format!("    <level>{}</level>\n", escape_xml(&log.level)));
            xml.push_str(&format!(
                "    <component>{}</component>\n",
                escape_xml(&log.component)
            ));
            xml.push_str(&format!(
                "    <message>{}</message>\n",
                escape_xml(&log.message)
            ));
            xml.push_str("  </log>\n");
        }

        xml.push_str("</logs>");
        Ok(xml)
    }

    fn format_users(&self, users: &[User]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<users>\n");

        for user in users {
            xml.push_str("  <user>\n");
            xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&user.name)));

            if let Some(ref realname) = user.realname {
                xml.push_str(&format!(
                    "    <realname>{}</realname>\n",
                    escape_xml(realname)
                ));
            }

            if let Some(ref user_type) = user.user_type {
                xml.push_str(&format!("    <type>{}</type>\n", escape_xml(user_type)));
            }

            if let Some(ref default_app) = user.default_app {
                xml.push_str(&format!(
                    "    <defaultApp>{}</defaultApp>\n",
                    escape_xml(default_app)
                ));
            }

            if !user.roles.is_empty() {
                xml.push_str("    <roles>\n");
                for role in &user.roles {
                    xml.push_str(&format!("      <role>{}</role>\n", escape_xml(role)));
                }
                xml.push_str("    </roles>\n");
            }

            if let Some(last_login) = user.last_successful_login {
                xml.push_str(&format!(
                    "    <lastSuccessfulLogin>{}</lastSuccessfulLogin>\n",
                    last_login
                ));
            }

            xml.push_str("  </user>\n");
        }

        xml.push_str("</users>\n");
        Ok(xml)
    }

    fn format_list_all(&self, output: &crate::commands::list_all::ListAllOutput) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<list_all>\n");
        xml.push_str(&format!(
            "  <timestamp>{}</timestamp>\n",
            escape_xml(&output.timestamp)
        ));
        xml.push_str("  <resources>\n");

        for resource in &output.resources {
            xml.push_str("    <resource>\n");
            xml.push_str(&format!(
                "      <type>{}</type>\n",
                escape_xml(&resource.resource_type)
            ));
            xml.push_str(&format!("      <count>{}</count>\n", resource.count));
            xml.push_str(&format!(
                "      <status>{}</status>\n",
                escape_xml(&resource.status)
            ));
            if let Some(error) = &resource.error {
                xml.push_str(&format!("      <error>{}</error>\n", escape_xml(error)));
            }
            xml.push_str("    </resource>\n");
        }

        xml.push_str("  </resources>\n");
        xml.push_str("</list_all>");
        Ok(xml)
    }
}

/// Format a JSON value as a string for display.
///
/// Converts any JSON value to its string representation:
/// - Strings are returned as-is
/// - Numbers and booleans are converted to their string representation
/// - Null values become empty strings
/// - Arrays and objects are serialized as compact JSON
fn format_json_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            // Serialize arrays/objects as compact JSON
            serde_json::to_string(v).unwrap_or_default()
        }
    }
}

/// Escape a string value for CSV output according to RFC 4180.
///
/// Rules:
/// - Wrap in double quotes if the field contains comma, double quote, or newline
/// - Double any internal double quotes (e.g., `"hello"` -> `""hello""`)
fn escape_csv(s: &str) -> String {
    let needs_quoting = s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r');
    if !needs_quoting {
        return s.to_string();
    }
    // Double all quotes and wrap in quotes
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// Escape special XML characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Get a formatter for the specified output format.
pub fn get_formatter(format: OutputFormat) -> Box<dyn Formatter> {
    match format {
        OutputFormat::Json => Box::new(JsonFormatter),
        OutputFormat::Table => Box::new(TableFormatter),
        OutputFormat::Csv => Box::new(CsvFormatter),
        OutputFormat::Xml => Box::new(XmlFormatter),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;
    use splunk_client::{KvStoreMember, KvStoreReplicationStatus};

    #[test]
    fn test_output_format_from_str() {
        assert_eq!(OutputFormat::from_str("json").unwrap(), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("JSON").unwrap(), OutputFormat::Json);
        assert_eq!(OutputFormat::from_str("csv").unwrap(), OutputFormat::Csv);
        assert_eq!(OutputFormat::from_str("xml").unwrap(), OutputFormat::Xml);
        assert_eq!(
            OutputFormat::from_str("table").unwrap(),
            OutputFormat::Table
        );
        assert!(OutputFormat::from_str("invalid").is_err());
    }

    #[test]
    fn test_xml_escaping() {
        assert_eq!(escape_xml("test&<>'\""), "test&amp;&lt;&gt;&apos;&quot;");
    }

    #[test]
    fn test_csv_escaping() {
        // No escaping needed for simple strings
        assert_eq!(escape_csv("simple"), "simple");
        // Comma requires quoting
        assert_eq!(escape_csv("hello,world"), "\"hello,world\"");
        // Quote requires doubling and wrapping
        assert_eq!(escape_csv("say \"hi\""), "\"say \"\"hi\"\"\"");
        // Newline requires quoting
        assert_eq!(escape_csv("line1\nline2"), "\"line1\nline2\"");
        // Mixed special chars
        assert_eq!(
            escape_csv("value, with \"quotes\"\nand newline"),
            "\"value, with \"\"quotes\"\"\nand newline\""
        );
    }

    #[test]
    fn test_format_json_value() {
        // String values
        assert_eq!(format_json_value(&json!("hello")), "hello");
        // Number values
        assert_eq!(format_json_value(&json!(42)), "42");
        assert_eq!(
            format_json_value(&json!(std::f64::consts::PI)),
            format!("{}", std::f64::consts::PI)
        );
        // Boolean values
        assert_eq!(format_json_value(&json!(true)), "true");
        assert_eq!(format_json_value(&json!(false)), "false");
        // Null values
        assert_eq!(format_json_value(&json!(null)), "");
        // Array values (compact JSON)
        assert_eq!(format_json_value(&json!([1, 2, 3])), "[1,2,3]");
        // Object values (compact JSON)
        assert_eq!(
            format_json_value(&json!({"key": "value"})),
            "{\"key\":\"value\"}"
        );
    }

    #[test]
    fn test_json_formatter() {
        let formatter = JsonFormatter;
        let results = vec![json!({"name": "test", "value": "123"})];
        let output = formatter.format_search_results(&results).unwrap();
        assert!(output.contains("test"));
        assert!(output.contains("123"));
    }

    #[test]
    fn test_csv_formatter() {
        let formatter = CsvFormatter;
        let results = vec![json!({"name": "test", "value": "123"})];
        let output = formatter.format_search_results(&results).unwrap();
        assert!(output.contains("name,value"));
        assert!(output.contains("test,123"));
    }

    #[test]
    fn test_csv_formatter_with_special_chars() {
        let formatter = CsvFormatter;
        let results = vec![json!({"name": "test,with,commas", "value": "say \"hello\""})];
        let output = formatter.format_search_results(&results).unwrap();
        // Headers should be properly escaped
        assert!(output.contains("name,value"));
        // Values with commas should be quoted
        assert!(output.contains("\"test,with,commas\""));
        // Values with quotes should have doubled quotes
        assert!(output.contains("\"say \"\"hello\"\"\""));
    }

    #[test]
    fn test_xml_formatter() {
        let formatter = XmlFormatter;
        let results = vec![json!({"name": "test", "value": "123"})];
        let output = formatter.format_search_results(&results).unwrap();
        assert!(output.contains("<?xml"));
        assert!(output.contains("<results>"));
        // New format uses nested elements instead of field attributes
        assert!(output.contains("<name>test</name>"));
        assert!(output.contains("<value>123</value>"));
        assert!(output.contains("</results>"));
    }

    #[test]
    fn test_table_formatter_with_non_string_values() {
        let formatter = TableFormatter;
        let results = vec![json!({"name": "test", "count": 42, "active": true, "data": null})];
        let output = formatter.format_search_results(&results).unwrap();
        // Numbers should be rendered
        assert!(output.contains("42"));
        // Booleans should be rendered
        assert!(output.contains("true"));
        // Null should be empty string (not "null")
        assert!(!output.contains("null"));
    }

    #[test]
    fn test_csv_formatter_with_non_string_values() {
        let formatter = CsvFormatter;
        let results = vec![json!({"name": "test", "count": 42, "active": true})];
        let output = formatter.format_search_results(&results).unwrap();
        // Numbers should be rendered
        assert!(output.contains("42"));
        // Booleans should be rendered
        assert!(output.contains("true"));
    }

    #[test]
    fn test_xml_formatter_with_non_string_values() {
        let formatter = XmlFormatter;
        let results =
            vec![json!({"name": "test", "count": 42, "active": true, "nested": {"key": "value"}})];
        let output = formatter.format_search_results(&results).unwrap();
        // Numbers should be rendered in nested elements
        assert!(output.contains("<count>42</count>"));
        // Booleans should be rendered
        assert!(output.contains("<active>true</active>"));
        // Nested objects should be properly nested, not JSON-escaped
        assert!(output.contains("<nested>"));
        assert!(output.contains("<key>value</key>"));
        assert!(output.contains("</nested>"));
        // Should NOT contain JSON serialization
        assert!(!output.contains("{&quot;"));
    }

    #[test]
    fn test_value_rendering() {
        // Test that numeric and boolean values appear in all formatters
        let results = vec![json!({"name": "test", "count": 123, "enabled": false})];

        // Table formatter
        let table_output = TableFormatter.format_search_results(&results).unwrap();
        assert!(table_output.contains("123"));
        assert!(table_output.contains("false"));

        // CSV formatter
        let csv_output = CsvFormatter.format_search_results(&results).unwrap();
        assert!(csv_output.contains("123"));
        assert!(csv_output.contains("false"));

        // XML formatter
        let xml_output = XmlFormatter.format_search_results(&results).unwrap();
        assert!(xml_output.contains("123"));
        assert!(xml_output.contains("false"));
    }

    #[test]
    fn test_table_formatter_indexes_basic() {
        let formatter = TableFormatter;
        let indexes = vec![Index {
            name: "main".to_string(),
            max_total_data_size_mb: Some(500),
            current_db_size_mb: 100,
            total_event_count: 1000,
            max_warm_db_count: Some(300),
            max_hot_buckets: Some(10),
            frozen_time_period_in_secs: Some(2592000),
            cold_db_path: Some("/opt/splunk/var/lib/splunk/main/colddb".to_string()),
            home_path: Some("/opt/splunk/var/lib/splunk/main/db".to_string()),
            thawed_path: Some("/opt/splunk/var/lib/splunk/main/thaweddb".to_string()),
            cold_to_frozen_dir: None,
            primary_index: Some(true),
        }];
        let output = formatter.format_indexes(&indexes, false).unwrap();
        assert!(output.contains("Name"));
        assert!(output.contains("Size (MB)"));
        assert!(output.contains("main"));
        assert!(!output.contains("Home Path"));
        assert!(!output.contains("Cold Path"));
        assert!(!output.contains("Retention"));
    }

    #[test]
    fn test_table_formatter_indexes_detailed() {
        let formatter = TableFormatter;
        let indexes = vec![Index {
            name: "main".to_string(),
            max_total_data_size_mb: Some(500),
            current_db_size_mb: 100,
            total_event_count: 1000,
            max_warm_db_count: Some(300),
            max_hot_buckets: Some(10),
            frozen_time_period_in_secs: Some(2592000),
            cold_db_path: Some("/opt/splunk/var/lib/splunk/main/colddb".to_string()),
            home_path: Some("/opt/splunk/var/lib/splunk/main/db".to_string()),
            thawed_path: Some("/opt/splunk/var/lib/splunk/main/thaweddb".to_string()),
            cold_to_frozen_dir: None,
            primary_index: Some(true),
        }];
        let output = formatter.format_indexes(&indexes, true).unwrap();
        assert!(output.contains("Name"));
        assert!(output.contains("Size (MB)"));
        assert!(output.contains("main"));
        assert!(output.contains("Home Path"));
        assert!(output.contains("Cold Path"));
        assert!(output.contains("Thawed Path"));
        assert!(output.contains("Retention (s)"));
        assert!(output.contains("2592000"));
        assert!(output.contains("/opt/splunk/var/lib/splunk/main/db"));
    }

    #[test]
    fn test_csv_formatter_indexes_basic() {
        let formatter = CsvFormatter;
        let indexes = vec![Index {
            name: "main".to_string(),
            max_total_data_size_mb: Some(500),
            current_db_size_mb: 100,
            total_event_count: 1000,
            max_warm_db_count: Some(300),
            max_hot_buckets: Some(10),
            frozen_time_period_in_secs: Some(2592000),
            cold_db_path: Some("/opt/splunk/var/lib/splunk/main/colddb".to_string()),
            home_path: Some("/opt/splunk/var/lib/splunk/main/db".to_string()),
            thawed_path: Some("/opt/splunk/var/lib/splunk/main/thaweddb".to_string()),
            cold_to_frozen_dir: None,
            primary_index: Some(true),
        }];
        let output = formatter.format_indexes(&indexes, false).unwrap();
        assert!(output.contains("Name,SizeMB,Events,MaxSizeMB"));
        assert!(!output.contains("HomePath"));
        assert!(!output.contains("ColdPath"));
    }

    #[test]
    fn test_csv_formatter_indexes_detailed() {
        let formatter = CsvFormatter;
        let indexes = vec![Index {
            name: "main".to_string(),
            max_total_data_size_mb: Some(500),
            current_db_size_mb: 100,
            total_event_count: 1000,
            max_warm_db_count: Some(300),
            max_hot_buckets: Some(10),
            frozen_time_period_in_secs: Some(2592000),
            cold_db_path: Some("/opt/splunk/var/lib/splunk/main/colddb".to_string()),
            home_path: Some("/opt/splunk/var/lib/splunk/main/db".to_string()),
            thawed_path: Some("/opt/splunk/var/lib/splunk/main/thaweddb".to_string()),
            cold_to_frozen_dir: None,
            primary_index: Some(true),
        }];
        let output = formatter.format_indexes(&indexes, true).unwrap();
        assert!(
            output.contains(
                "Name,SizeMB,Events,MaxSizeMB,RetentionSecs,HomePath,ColdPath,ThawedPath"
            )
        );
        assert!(output.contains("2592000"));
        assert!(output.contains("/opt/splunk/var/lib/splunk/main/db"));
    }

    #[test]
    fn test_xml_formatter_indexes_basic() {
        let formatter = XmlFormatter;
        let indexes = vec![Index {
            name: "main".to_string(),
            max_total_data_size_mb: Some(500),
            current_db_size_mb: 100,
            total_event_count: 1000,
            max_warm_db_count: Some(300),
            max_hot_buckets: Some(10),
            frozen_time_period_in_secs: Some(2592000),
            cold_db_path: Some("/opt/splunk/var/lib/splunk/main/colddb".to_string()),
            home_path: Some("/opt/splunk/var/lib/splunk/main/db".to_string()),
            thawed_path: Some("/opt/splunk/var/lib/splunk/main/thaweddb".to_string()),
            cold_to_frozen_dir: None,
            primary_index: Some(true),
        }];
        let output = formatter.format_indexes(&indexes, false).unwrap();
        assert!(output.contains("<?xml"));
        assert!(output.contains("<indexes>"));
        assert!(output.contains("<name>main</name>"));
        assert!(output.contains("<sizeMB>100</sizeMB>"));
        assert!(output.contains("<maxSizeMB>500</maxSizeMB>"));
        // Detailed fields should NOT be present
        assert!(!output.contains("<homePath>"));
        assert!(!output.contains("<coldPath>"));
        assert!(!output.contains("<retentionSecs>"));
    }

    #[test]
    fn test_xml_formatter_indexes_detailed() {
        let formatter = XmlFormatter;
        let indexes = vec![Index {
            name: "main".to_string(),
            max_total_data_size_mb: Some(500),
            current_db_size_mb: 100,
            total_event_count: 1000,
            max_warm_db_count: Some(300),
            max_hot_buckets: Some(10),
            frozen_time_period_in_secs: Some(2592000),
            cold_db_path: Some("/opt/splunk/var/lib/splunk/main/colddb".to_string()),
            home_path: Some("/opt/splunk/var/lib/splunk/main/db".to_string()),
            thawed_path: Some("/opt/splunk/var/lib/splunk/main/thaweddb".to_string()),
            cold_to_frozen_dir: None,
            primary_index: Some(true),
        }];
        let output = formatter.format_indexes(&indexes, true).unwrap();
        assert!(output.contains("<?xml"));
        assert!(output.contains("<indexes>"));
        assert!(output.contains("<name>main</name>"));
        assert!(output.contains("<sizeMB>100</sizeMB>"));
        // Detailed fields SHOULD be present
        assert!(output.contains("<homePath>/opt/splunk/var/lib/splunk/main/db</homePath>"));
        assert!(output.contains("<coldPath>/opt/splunk/var/lib/splunk/main/colddb</coldPath>"));
        assert!(
            output.contains("<thawedPath>/opt/splunk/var/lib/splunk/main/thaweddb</thawedPath>")
        );
        assert!(output.contains("<retentionSecs>2592000</retentionSecs>"));
    }

    #[test]
    fn test_json_formatter_indexes_always_detailed() {
        let formatter = JsonFormatter;
        let indexes = vec![Index {
            name: "main".to_string(),
            max_total_data_size_mb: Some(500),
            current_db_size_mb: 100,
            total_event_count: 1000,
            max_warm_db_count: Some(300),
            max_hot_buckets: Some(10),
            frozen_time_period_in_secs: Some(2592000),
            cold_db_path: Some("/opt/splunk/var/lib/splunk/main/colddb".to_string()),
            home_path: Some("/opt/splunk/var/lib/splunk/main/db".to_string()),
            thawed_path: Some("/opt/splunk/var/lib/splunk/main/thaweddb".to_string()),
            cold_to_frozen_dir: None,
            primary_index: Some(true),
        }];
        // JSON always outputs all fields regardless of detailed flag
        let output_basic = formatter.format_indexes(&indexes, false).unwrap();
        let output_detailed = formatter.format_indexes(&indexes, true).unwrap();
        // Check that the JSON contains the expected fields (using serde rename names)
        assert!(output_basic.contains("\"name\""));
        assert!(output_basic.contains("\"homePath\""));
        assert!(output_basic.contains("\"coldDBPath\""));
        // Both should be identical since JSON ignores the detailed flag
        assert_eq!(output_basic, output_detailed);
    }

    #[test]
    fn test_cluster_peers_json_formatting() {
        let formatter = JsonFormatter;
        let cluster_info = ClusterInfoOutput {
            id: "cluster-1".to_string(),
            label: Some("test-cluster".to_string()),
            mode: "master".to_string(),
            manager_uri: Some("https://master:8089".to_string()),
            replication_factor: Some(3),
            search_factor: Some(2),
            status: Some("Enabled".to_string()),
            peers: Some(vec![
                ClusterPeerOutput {
                    host: "peer1".to_string(),
                    port: 8089,
                    id: "peer-1".to_string(),
                    status: "Up".to_string(),
                    peer_state: "Ready".to_string(),
                    label: Some("Peer 1".to_string()),
                    site: Some("site1".to_string()),
                    is_captain: true,
                },
                ClusterPeerOutput {
                    host: "peer2".to_string(),
                    port: 8089,
                    id: "peer-2".to_string(),
                    status: "Up".to_string(),
                    peer_state: "Ready".to_string(),
                    label: None,
                    site: None,
                    is_captain: false,
                },
            ]),
        };
        let output = formatter.format_cluster_info(&cluster_info, true).unwrap();
        // Verify JSON structure includes peers array
        assert!(output.contains("\"peers\""));
        assert!(output.contains("\"host\""));
        assert!(output.contains("\"peer1\""));
        assert!(output.contains("\"peer2\""));
        assert!(output.contains("\"is_captain\""));
        assert!(output.contains("true"));
        assert!(output.contains("false"));
    }

    #[test]
    fn test_cluster_peers_csv_formatting() {
        let formatter = CsvFormatter;
        let cluster_info = ClusterInfoOutput {
            id: "cluster-1".to_string(),
            label: Some("test-cluster".to_string()),
            mode: "master".to_string(),
            manager_uri: Some("https://master:8089".to_string()),
            replication_factor: Some(3),
            search_factor: Some(2),
            status: Some("Enabled".to_string()),
            peers: Some(vec![ClusterPeerOutput {
                host: "peer1".to_string(),
                port: 8089,
                id: "peer-1".to_string(),
                status: "Up".to_string(),
                peer_state: "Ready".to_string(),
                label: Some("Peer,1".to_string()),
                site: Some("site1".to_string()),
                is_captain: true,
            }]),
        };
        let output = formatter.format_cluster_info(&cluster_info, true).unwrap();
        // Verify CSV has cluster info row and peer row
        assert!(output.contains("ClusterInfo"));
        assert!(output.contains("cluster-1"));
        assert!(output.contains("Peer"));
        assert!(output.contains("peer1:8089"));
        // Verify CSV escaping for label with comma
        assert!(output.contains("\"Peer,1\""));
        assert!(output.contains("Yes"));
    }

    #[test]
    fn test_cluster_peers_xml_formatting() {
        let formatter = XmlFormatter;
        let cluster_info = ClusterInfoOutput {
            id: "cluster-1".to_string(),
            label: Some("test-cluster".to_string()),
            mode: "master".to_string(),
            manager_uri: Some("https://master:8089".to_string()),
            replication_factor: Some(3),
            search_factor: Some(2),
            status: Some("Enabled".to_string()),
            peers: Some(vec![ClusterPeerOutput {
                host: "peer1".to_string(),
                port: 8089,
                id: "peer-1".to_string(),
                status: "Up".to_string(),
                peer_state: "Ready".to_string(),
                label: Some("Peer 1".to_string()),
                site: Some("site1".to_string()),
                is_captain: true,
            }]),
        };
        let output = formatter.format_cluster_info(&cluster_info, true).unwrap();
        // Verify XML structure
        assert!(output.contains("<cluster>"));
        assert!(output.contains("<id>cluster-1</id>"));
        assert!(output.contains("<peers>"));
        assert!(output.contains("<peer>"));
        assert!(output.contains("<host>peer1</host>"));
        assert!(output.contains("<port>8089</port>"));
        assert!(output.contains("<isCaptain>true</isCaptain>"));
        assert!(output.contains("</peers>"));
        assert!(output.contains("</cluster>"));
    }

    #[test]
    fn test_cluster_peers_table_formatting() {
        let formatter = TableFormatter;
        let cluster_info = ClusterInfoOutput {
            id: "cluster-1".to_string(),
            label: Some("test-cluster".to_string()),
            mode: "master".to_string(),
            manager_uri: Some("https://master:8089".to_string()),
            replication_factor: Some(3),
            search_factor: Some(2),
            status: Some("Enabled".to_string()),
            peers: Some(vec![ClusterPeerOutput {
                host: "peer1".to_string(),
                port: 8089,
                id: "peer-1".to_string(),
                status: "Up".to_string(),
                peer_state: "Ready".to_string(),
                label: Some("Peer 1".to_string()),
                site: Some("site1".to_string()),
                is_captain: true,
            }]),
        };
        let output = formatter.format_cluster_info(&cluster_info, true).unwrap();
        // Verify table structure includes peers
        assert!(output.contains("Cluster Information:"));
        assert!(output.contains("ID: cluster-1"));
        assert!(output.contains("Cluster Peers (1)"));
        assert!(output.contains("Host: peer1:8089"));
        assert!(output.contains("Captain: Yes"));
    }

    #[test]
    fn test_kvstore_peers_table_formatting() {
        let status = KvStoreStatus {
            current_member: KvStoreMember {
                guid: "guid".to_string(),
                host: "localhost".to_string(),
                port: 8089,
                replica_set: "rs0".to_string(),
                status: "Ready".to_string(),
            },
            replication_status: KvStoreReplicationStatus {
                oplog_size: 100,
                oplog_used: 1.5,
            },
        };
        let output = TableFormatter.format_kvstore_status(&status).unwrap();
        assert!(output.contains("KVStore Status:"));
        assert!(output.contains("localhost:8089"));
        assert!(output.contains("Status: Ready"));
        assert!(output.contains("Replica Set: rs0"));
        assert!(output.contains("Oplog Size: 100 MB"));
        assert!(output.contains("Oplog Used: 1.50%"));
    }

    #[test]
    fn test_kvstore_peers_json_formatting() {
        let status = KvStoreStatus {
            current_member: KvStoreMember {
                guid: "guid".to_string(),
                host: "localhost".to_string(),
                port: 8089,
                replica_set: "rs0".to_string(),
                status: "Ready".to_string(),
            },
            replication_status: KvStoreReplicationStatus {
                oplog_size: 100,
                oplog_used: 1.5,
            },
        };
        let output = JsonFormatter.format_kvstore_status(&status).unwrap();
        assert!(output.contains("\"currentMember\""));
        assert!(output.contains("\"replicationStatus\""));
        assert!(output.contains("\"localhost\""));
        assert!(output.contains("\"rs0\""));
    }

    #[test]
    fn test_kvstore_peers_csv_formatting() {
        let status = KvStoreStatus {
            current_member: KvStoreMember {
                guid: "guid".to_string(),
                host: "localhost".to_string(),
                port: 8089,
                replica_set: "rs0".to_string(),
                status: "Ready".to_string(),
            },
            replication_status: KvStoreReplicationStatus {
                oplog_size: 100,
                oplog_used: 1.5,
            },
        };
        let output = CsvFormatter.format_kvstore_status(&status).unwrap();
        assert!(output.contains("host,port,status,replica_set,oplog_size_mb,oplog_used_percent"));
        assert!(output.contains("localhost,8089,Ready,rs0,100,1.5"));
    }

    #[test]
    fn test_kvstore_peers_xml_formatting() {
        let status = KvStoreStatus {
            current_member: KvStoreMember {
                guid: "guid".to_string(),
                host: "localhost".to_string(),
                port: 8089,
                replica_set: "rs0".to_string(),
                status: "Ready".to_string(),
            },
            replication_status: KvStoreReplicationStatus {
                oplog_size: 100,
                oplog_used: 1.5,
            },
        };
        let output = XmlFormatter.format_kvstore_status(&status).unwrap();
        assert!(output.contains("<kvstoreStatus>"));
        assert!(output.contains("<host>localhost</host>"));
        assert!(output.contains("<port>8089</port>"));
        assert!(output.contains("<oplogUsed>1.50</oplogUsed>"));
    }

    #[test]
    fn test_format_license_table() {
        let formatter = TableFormatter;
        let license = LicenseInfoOutput {
            usage: vec![LicenseUsage {
                name: "daily_usage".to_string(),
                quota: 100 * 1024 * 1024,
                used_bytes: 50 * 1024 * 1024,
                slaves_usage_bytes: None,
                stack_id: Some("enterprise".to_string()),
            }],
            pools: vec![LicensePool {
                name: "pool1".to_string(),
                quota: 50 * 1024 * 1024,
                used_bytes: 25 * 1024 * 1024,
                stack_id: "enterprise".to_string(),
                description: Some("Test pool".to_string()),
            }],
            stacks: vec![LicenseStack {
                name: "enterprise".to_string(),
                quota: 100 * 1024 * 1024,
                type_name: "enterprise".to_string(),
                label: "Enterprise".to_string(),
            }],
        };

        let output = formatter.format_license(&license).unwrap();
        assert!(output.contains("daily_usage"));
        assert!(output.contains("50.0%"));
        assert!(output.contains("pool1"));
        assert!(output.contains("Test pool"));
        assert!(output.contains("Enterprise"));
    }

    #[test]
    fn test_format_license_csv() {
        let formatter = CsvFormatter;
        let license = LicenseInfoOutput {
            usage: vec![LicenseUsage {
                name: "daily_usage".to_string(),
                quota: 100 * 1024 * 1024,
                used_bytes: 50 * 1024 * 1024,
                slaves_usage_bytes: None,
                stack_id: Some("enterprise".to_string()),
            }],
            pools: vec![],
            stacks: vec![],
        };

        let output = formatter.format_license(&license).unwrap();
        assert!(output.contains("Type,Name,StackID,UsedMB,QuotaMB,PctUsed"));
        assert!(output.contains("Usage,daily_usage,enterprise,50,100,50.00"));
    }

    // === RQ-0056: Tests for flattening nested JSON structures ===

    #[test]
    fn test_flatten_simple_object() {
        let value = json!({"name": "Alice", "age": 30});
        let mut flat = std::collections::BTreeMap::new();
        flatten_json_object(&value, "", &mut flat);
        assert_eq!(flat.get("name"), Some(&"Alice".to_string()));
        assert_eq!(flat.get("age"), Some(&"30".to_string()));
    }

    #[test]
    fn test_flatten_nested_object() {
        let value = json!({"user": {"name": "Bob", "address": {"city": "NYC"}}});
        let mut flat = std::collections::BTreeMap::new();
        flatten_json_object(&value, "", &mut flat);
        assert_eq!(flat.get("user.name"), Some(&"Bob".to_string()));
        assert_eq!(flat.get("user.address.city"), Some(&"NYC".to_string()));
    }

    #[test]
    fn test_flatten_array() {
        let value = json!({"tags": ["foo", "bar", "baz"]});
        let mut flat = std::collections::BTreeMap::new();
        flatten_json_object(&value, "", &mut flat);
        assert_eq!(flat.get("tags.0"), Some(&"foo".to_string()));
        assert_eq!(flat.get("tags.1"), Some(&"bar".to_string()));
        assert_eq!(flat.get("tags.2"), Some(&"baz".to_string()));
    }

    #[test]
    fn test_flatten_array_of_objects() {
        let value = json!({"users": [{"name": "Alice"}, {"name": "Bob"}]});
        let mut flat = std::collections::BTreeMap::new();
        flatten_json_object(&value, "", &mut flat);
        assert_eq!(flat.get("users.0.name"), Some(&"Alice".to_string()));
        assert_eq!(flat.get("users.1.name"), Some(&"Bob".to_string()));
    }

    #[test]
    fn test_flatten_null_values() {
        let value = json!({"name": "Test", "optional": null});
        let mut flat = std::collections::BTreeMap::new();
        flatten_json_object(&value, "", &mut flat);
        assert_eq!(flat.get("name"), Some(&"Test".to_string()));
        assert_eq!(flat.get("optional"), Some(&"".to_string())); // null becomes empty string
    }

    #[test]
    fn test_get_all_flattened_keys() {
        let results = vec![
            json!({"user": {"name": "Alice"}}),
            json!({"user": {"age": 30}, "status": "active"}),
        ];
        let keys = get_all_flattened_keys(&results);
        // Should include all unique keys in sorted order
        assert!(keys.contains(&"status".to_string()));
        assert!(keys.contains(&"user.age".to_string()));
        assert!(keys.contains(&"user.name".to_string()));
    }

    #[test]
    fn test_csv_formatter_nested_objects() {
        let formatter = CsvFormatter;
        let results = vec![
            json!({"user": {"name": "Alice", "age": 30}, "status": "active"}),
            json!({"user": {"name": "Bob"}, "status": "inactive"}),
        ];
        let output = formatter.format_search_results(&results).unwrap();

        // Headers should include dot-notation keys
        assert!(output.contains("status"));
        assert!(output.contains("user.age"));
        assert!(output.contains("user.name"));

        // First row - Alice has all fields
        assert!(output.contains("active"));
        assert!(output.contains("30"));
        assert!(output.contains("Alice"));

        // Second row - Bob is missing age field - should be empty
        assert!(output.contains("inactive"));
        assert!(output.contains("Bob"));
    }

    #[test]
    fn test_csv_formatter_deeply_nested() {
        let formatter = CsvFormatter;
        let results = vec![json!({
            "location": {
                "address": {
                    "city": "NYC",
                    "zip": "10001"
                }
            }
        })];
        let output = formatter.format_search_results(&results).unwrap();
        assert!(output.contains("location.address.city"));
        assert!(output.contains("location.address.zip"));
        assert!(output.contains("NYC"));
        assert!(output.contains("10001"));
    }

    #[test]
    fn test_csv_formatter_arrays() {
        let formatter = CsvFormatter;
        let results = vec![json!({"tags": ["foo", "bar"], "count": 2})];
        let output = formatter.format_search_results(&results).unwrap();
        assert!(output.contains("count"));
        assert!(output.contains("tags.0"));
        assert!(output.contains("tags.1"));
        assert!(output.contains("foo"));
        assert!(output.contains("bar"));
    }

    #[test]
    fn test_xml_formatter_nested_structure() {
        let formatter = XmlFormatter;
        let results = vec![json!({"user": {"name": "Alice", "age": 30}})];
        let output = formatter.format_search_results(&results).unwrap();

        // Should have proper nesting, not escaped JSON
        assert!(output.contains("<user>"));
        assert!(output.contains("<name>Alice</name>"));
        assert!(output.contains("<age>30</age>"));
        assert!(output.contains("</user>"));

        // Should NOT contain JSON serialization
        assert!(!output.contains("{&quot;"));
    }

    #[test]
    fn test_xml_formatter_arrays() {
        let formatter = XmlFormatter;
        let results = vec![json!({"tags": ["foo", "bar"]})];
        let output = formatter.format_search_results(&results).unwrap();

        assert!(output.contains("<tags>"));
        assert!(output.contains("<item>foo</item>"));
        assert!(output.contains("<item>bar</item>"));
        assert!(output.contains("</tags>"));
    }

    #[test]
    fn test_xml_formatter_complex_nesting() {
        let formatter = XmlFormatter;
        let results = vec![json!({
            "user": {
                "name": "Bob",
                "roles": ["admin", "user"]
            }
        })];
        let output = formatter.format_search_results(&results).unwrap();

        assert!(output.contains("<user>"));
        assert!(output.contains("<name>Bob</name>"));
        assert!(output.contains("<roles>"));
        assert!(output.contains("<item>admin</item>"));
        assert!(output.contains("<item>user</item>"));
        assert!(output.contains("</roles>"));
        assert!(output.contains("</user>"));
    }

    #[test]
    fn test_xml_formatter_null_values() {
        let formatter = XmlFormatter;
        let results = vec![json!({"name": "test", "optional": null})];
        let output = formatter.format_search_results(&results).unwrap();

        // Null values should produce empty elements
        assert!(output.contains("<name>test</name>"));
        assert!(output.contains("<optional></optional>"));
    }

    #[test]
    fn test_xml_formatter_deep_nesting() {
        let formatter = XmlFormatter;
        let results = vec![json!({
            "location": {
                "address": {
                    "city": "NYC",
                    "zip": "10001"
                }
            }
        })];
        let output = formatter.format_search_results(&results).unwrap();

        // Should have deeply nested structure
        assert!(output.contains("<location>"));
        assert!(output.contains("<address>"));
        assert!(output.contains("<city>NYC</city>"));
        assert!(output.contains("<zip>10001</zip>"));
        assert!(output.contains("</address>"));
        assert!(output.contains("</location>"));
    }

    #[test]
    fn test_users_json_formatting() {
        let formatter = JsonFormatter;
        let users = vec![User {
            name: "admin".to_string(),
            realname: Some("Administrator".to_string()),
            email: Some("admin@example.com".to_string()),
            user_type: Some("Splunk".to_string()),
            default_app: Some("launcher".to_string()),
            roles: vec!["admin".to_string(), "power".to_string()],
            last_successful_login: Some(1704067200),
        }];
        let output = formatter.format_users(&users).unwrap();
        assert!(output.contains("\"name\""));
        assert!(output.contains("\"admin\""));
        assert!(output.contains("\"realname\""));
        assert!(output.contains("\"Administrator\""));
        assert!(output.contains("\"type\""));
        assert!(output.contains("\"Splunk\""));
        assert!(output.contains("\"defaultApp\""));
        assert!(output.contains("\"launcher\""));
        assert!(output.contains("\"roles\""));
        assert!(output.contains("\"admin\""));
        assert!(output.contains("\"power\""));
        assert!(output.contains("\"lastSuccessfulLogin\""));
        assert!(output.contains("1704067200"));
    }

    #[test]
    fn test_users_table_formatting() {
        let formatter = TableFormatter;
        let users = vec![
            User {
                name: "admin".to_string(),
                realname: Some("Administrator".to_string()),
                email: Some("admin@example.com".to_string()),
                user_type: Some("Splunk".to_string()),
                default_app: Some("launcher".to_string()),
                roles: vec!["admin".to_string(), "power".to_string()],
                last_successful_login: Some(1704067200),
            },
            User {
                name: "user1".to_string(),
                realname: None,
                email: None,
                user_type: None,
                default_app: None,
                roles: vec![],
                last_successful_login: None,
            },
        ];
        let output = formatter.format_users(&users).unwrap();
        assert!(output.contains("NAME"));
        assert!(output.contains("REAL NAME"));
        assert!(output.contains("TYPE"));
        assert!(output.contains("ROLES"));
        assert!(output.contains("admin"));
        assert!(output.contains("Administrator"));
        assert!(output.contains("Splunk"));
        assert!(output.contains("admin, power"));
        assert!(output.contains("user1"));
    }

    #[test]
    fn test_users_table_empty() {
        let formatter = TableFormatter;
        let users: Vec<User> = vec![];
        let output = formatter.format_users(&users).unwrap();
        assert!(output.contains("No users found"));
    }

    #[test]
    fn test_users_csv_formatting() {
        let formatter = CsvFormatter;
        let users = vec![User {
            name: "admin".to_string(),
            realname: Some("Administrator".to_string()),
            email: Some("admin@example.com".to_string()),
            user_type: Some("Splunk".to_string()),
            default_app: Some("launcher".to_string()),
            roles: vec!["admin".to_string(), "power".to_string()],
            last_successful_login: Some(1704067200),
        }];
        let output = formatter.format_users(&users).unwrap();
        assert!(output.contains("name,realname,user_type,default_app,roles,last_successful_login"));
        assert!(output.contains("admin,Administrator,Splunk,launcher,admin;power,1704067200"));
    }

    #[test]
    fn test_users_csv_special_characters() {
        let formatter = CsvFormatter;
        let users = vec![User {
            name: "user,name".to_string(),
            realname: Some("User, Name".to_string()),
            email: None,
            user_type: None,
            default_app: None,
            roles: vec![],
            last_successful_login: None,
        }];
        let output = formatter.format_users(&users).unwrap();
        assert!(output.contains("\"user,name\""));
        assert!(output.contains("\"User, Name\""));
    }

    #[test]
    fn test_users_xml_formatting() {
        let formatter = XmlFormatter;
        let users = vec![
            User {
                name: "admin".to_string(),
                realname: Some("Administrator".to_string()),
                email: Some("admin@example.com".to_string()),
                user_type: Some("Splunk".to_string()),
                default_app: Some("launcher".to_string()),
                roles: vec!["admin".to_string(), "power".to_string()],
                last_successful_login: Some(1704067200),
            },
            User {
                name: "user1".to_string(),
                realname: None,
                email: None,
                user_type: None,
                default_app: None,
                roles: vec![],
                last_successful_login: None,
            },
        ];
        let output = formatter.format_users(&users).unwrap();
        assert!(output.contains("<?xml"));
        assert!(output.contains("<users>"));
        assert!(output.contains("<user>"));
        assert!(output.contains("<name>admin</name>"));
        assert!(output.contains("<realname>Administrator</realname>"));
        assert!(output.contains("<type>Splunk</type>"));
        assert!(output.contains("<defaultApp>launcher</defaultApp>"));
        assert!(output.contains("<roles>"));
        assert!(output.contains("<role>admin</role>"));
        assert!(output.contains("<role>power</role>"));
        assert!(output.contains("</roles>"));
        assert!(output.contains("<lastSuccessfulLogin>1704067200</lastSuccessfulLogin>"));
        assert!(output.contains("<name>user1</name>"));
        assert!(output.contains("</users>"));
    }
}
