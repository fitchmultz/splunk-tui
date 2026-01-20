//! Output formatters for CLI commands.
//!
//! Provides multiple output formats: JSON, Table, CSV, and XML.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use splunk_client::{
    ClusterPeer, Index, KvStoreStatus, LicenseUsage, LogParsingHealth, SearchJobStatus, ServerInfo,
    SplunkHealth,
};

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
}

/// Health check output aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_info: Option<ServerInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splunkd_health: Option<SplunkHealth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_usage: Option<Vec<LicenseUsage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kvstore_status: Option<KvStoreStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_parsing_health: Option<LogParsingHealth>,
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
}

/// CSV formatter.
pub struct CsvFormatter;

impl Formatter for CsvFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        if results.is_empty() {
            return Ok(String::new());
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

        // Print header (escaped)
        let header: Vec<String> = all_keys.iter().map(|k| escape_csv(k)).collect();
        output.push_str(&header.join(","));
        output.push('\n');

        // Print rows
        for result in results {
            if let Some(obj) = result.as_object() {
                let row: Vec<String> = all_keys
                    .iter()
                    .map(|key| {
                        let value = obj.get(key).map(format_json_value).unwrap_or_default();
                        escape_csv(&value)
                    })
                    .collect();
                output.push_str(&row.join(","));
                output.push('\n');
            }
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
}

/// XML formatter.
pub struct XmlFormatter;

impl Formatter for XmlFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<results>\n");

        for (i, result) in results.iter().enumerate() {
            xml.push_str(&format!("  <result index=\"{}\">\n", i));
            if let Some(obj) = result.as_object() {
                for (key, value) in obj {
                    let value_str = format_json_value(value);
                    xml.push_str(&format!(
                        "    <field name=\"{}\">{}</field>\n",
                        escape_xml(key),
                        escape_xml(&value_str)
                    ));
                }
            }
            xml.push_str("  </result>\n");
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
        assert!(output.contains("<field name=\"name\">test</field>"));
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
        // Numbers should be rendered
        assert!(output.contains("<field name=\"count\">42</field>"));
        // Booleans should be rendered
        assert!(output.contains("<field name=\"active\">true</field>"));
        // Objects should be rendered as compact JSON (with XML-escaped quotes)
        assert!(
            output.contains("<field name=\"nested\">{&quot;key&quot;:&quot;value&quot;}</field>")
        );
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
    fn test_cluster_peers_not_shown_when_not_detailed() {
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
        let output = formatter.format_cluster_info(&cluster_info, false).unwrap();
        // Verify basic cluster info is shown
        assert!(output.contains("Cluster Information:"));
        assert!(output.contains("ID: cluster-1"));
        // Verify peers are NOT shown when detailed=false
        assert!(!output.contains("Cluster Peers"));
        assert!(!output.contains("Host: peer1"));
    }
}
