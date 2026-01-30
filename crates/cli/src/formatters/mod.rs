//! Output formatters for CLI commands.
//!
//! Responsibilities:
//! - Provide multiple output formats: JSON, Table, CSV, and XML.
//! - Implement the `Formatter` trait for various Splunk resource types.
//! - Handle nested JSON flattening for CSV and hierarchical mapping for XML.
//!
//! Does NOT handle:
//! - Direct printing to stdout (returns formatted strings).
//! - Terminal UI rendering (see `crates/tui`).
//!
//! Invariants / Assumptions:
//! - Tables use tab-separation for consistent alignment in standard terminals.
//! - XML output includes a standard version/encoding declaration.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use splunk_client::models::{Input, LogEntry, SearchPeer};
use splunk_client::{
    App, ClusterPeer, Forwarder, Index, KvStoreStatus, LicensePool, LicenseStack, LicenseUsage,
    SavedSearch, SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;

pub use splunk_client::HealthCheckOutput;

mod common;
mod csv;
mod json;
mod table;
mod xml;

pub use common::write_to_file;
pub use csv::CsvFormatter;
pub use json::JsonFormatter;
pub use table::{Pagination, TableFormatter};
pub use xml::XmlFormatter;

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

    /// Format logs for streaming/tail mode.
    ///
    /// Default implementation delegates to `format_logs`. Individual formatters
    /// can override this to provide streaming-appropriate output (e.g., headers
    /// only on first call, NDJSON for JSON format).
    ///
    /// # Arguments
    /// * `logs` - The log entries to format
    /// * `is_first` - True if this is the first batch in the stream
    fn format_logs_streaming(&self, logs: &[LogEntry], is_first: bool) -> Result<String> {
        let _ = is_first; // unused in default impl
        self.format_logs(logs)
    }

    /// Format users list.
    fn format_users(&self, users: &[User]) -> Result<String>;

    /// Format apps list.
    fn format_apps(&self, apps: &[App]) -> Result<String>;

    /// Format detailed app information for info subcommand.
    fn format_app_info(&self, app: &App) -> Result<String>;

    /// Format list-all unified resource overview.
    #[allow(dead_code)]
    fn format_list_all(&self, output: &crate::commands::list_all::ListAllOutput) -> Result<String>;

    /// Format saved searches list.
    fn format_saved_searches(&self, searches: &[SavedSearch]) -> Result<String>;

    /// Format detailed saved search information.
    fn format_saved_search_info(&self, search: &SavedSearch) -> Result<String>;

    /// Format detailed job information.
    fn format_job_details(&self, job: &SearchJobStatus) -> Result<String>;

    /// Format profile configuration.
    fn format_profile(&self, profile_name: &str, profile: &ProfileConfig) -> Result<String>;

    /// Format all configured profiles.
    fn format_profiles(
        &self,
        profiles: &std::collections::BTreeMap<String, ProfileConfig>,
    ) -> Result<String>;

    /// Format forwarders list.
    fn format_forwarders(&self, forwarders: &[Forwarder], detailed: bool) -> Result<String>;

    /// Format search peers list.
    fn format_search_peers(&self, peers: &[SearchPeer], detailed: bool) -> Result<String>;

    /// Format data inputs list.
    fn format_inputs(&self, inputs: &[Input], detailed: bool) -> Result<String>;
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
mod tests;
