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
use splunk_client::models::{
    AuditEvent, ConfigFile, ConfigStanza, Dashboard, FiredAlert, Input, KvStoreCollection,
    KvStoreRecord, LogEntry, SearchPeer,
};
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

/// License installation result output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseInstallOutput {
    pub success: bool,
    pub message: String,
    pub license_name: Option<String>,
}

/// License pool operation output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicensePoolOperationOutput {
    pub operation: String,
    pub pool_name: String,
    pub success: bool,
    pub message: String,
}

/// License activation/deactivation output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LicenseActivationOutput {
    pub operation: String,
    pub license_name: String,
    pub success: bool,
    pub message: String,
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

    /// Format cluster peers list.
    fn format_cluster_peers(
        &self,
        peers: &[ClusterPeerOutput],
        pagination: &Pagination,
    ) -> Result<String>;

    /// Format cluster management operation result.
    fn format_cluster_management(&self, output: &ClusterManagementOutput) -> Result<String>;

    /// Format health check results.
    fn format_health(&self, health: &HealthCheckOutput) -> Result<String>;

    /// Format KVStore status.
    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String>;

    /// Format KVStore collections list.
    fn format_kvstore_collections(&self, collections: &[KvStoreCollection]) -> Result<String>;

    /// Format KVStore collection records.
    fn format_kvstore_records(&self, records: &[KvStoreRecord]) -> Result<String>;

    /// Format license information.
    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String>;

    /// Format installed licenses list.
    fn format_installed_licenses(
        &self,
        licenses: &[splunk_client::InstalledLicense],
    ) -> Result<String>;

    /// Format license installation result.
    fn format_license_install(&self, result: &LicenseInstallOutput) -> Result<String>;

    /// Format license pools list.
    fn format_license_pools(&self, pools: &[LicensePool]) -> Result<String>;

    /// Format license pool operation result.
    fn format_license_pool_operation(&self, result: &LicensePoolOperationOutput) -> Result<String>;

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

    /// Format lookup tables list.
    fn format_lookups(&self, lookups: &[splunk_client::LookupTable]) -> Result<String>;

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

    /// Format config files list.
    fn format_config_files(&self, files: &[ConfigFile]) -> Result<String>;

    /// Format config stanzas list.
    fn format_config_stanzas(&self, stanzas: &[ConfigStanza]) -> Result<String>;

    /// Format a single config stanza in detail.
    fn format_config_stanza(&self, stanza: &ConfigStanza) -> Result<String>;

    /// Format fired alerts list.
    fn format_fired_alerts(&self, alerts: &[FiredAlert]) -> Result<String>;

    /// Format detailed fired alert information.
    fn format_fired_alert_info(&self, alert: &FiredAlert) -> Result<String>;

    /// Format roles list.
    fn format_roles(&self, roles: &[splunk_client::Role]) -> Result<String>;

    /// Format capabilities list.
    fn format_capabilities(&self, capabilities: &[splunk_client::Capability]) -> Result<String>;

    /// Format HEC response.
    fn format_hec_response(&self, response: &splunk_client::HecResponse) -> Result<String>;

    /// Format HEC batch response.
    fn format_hec_batch_response(
        &self,
        response: &splunk_client::HecBatchResponse,
    ) -> Result<String>;

    /// Format HEC health status.
    fn format_hec_health(&self, health: &splunk_client::HecHealth) -> Result<String>;

    /// Format HEC acknowledgment status.
    fn format_hec_ack_status(&self, status: &splunk_client::HecAckStatus) -> Result<String>;

    /// Format macros list.
    fn format_macros(&self, macros: &[splunk_client::Macro]) -> Result<String>;

    /// Format detailed macro information.
    fn format_macro_info(&self, macro_info: &splunk_client::Macro) -> Result<String>;

    /// Format audit events list.
    fn format_audit_events(&self, events: &[AuditEvent], detailed: bool) -> Result<String>;

    /// Format dashboards list.
    fn format_dashboards(&self, dashboards: &[Dashboard], detailed: bool) -> Result<String>;

    /// Format detailed dashboard information.
    fn format_dashboard(&self, dashboard: &Dashboard) -> Result<String>;
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
    pub maintenance_mode: Option<bool>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub peers: Option<Vec<ClusterPeerOutput>>,
}

/// Cluster management operation output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClusterManagementOutput {
    pub operation: String,
    pub target: String,
    pub success: bool,
    pub message: String,
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
