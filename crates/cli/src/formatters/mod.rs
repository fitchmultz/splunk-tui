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
//! Invariants:
//! - Tables use tab-separation for consistent alignment in standard terminals.
//! - XML output includes a standard version/encoding declaration.
//!
//! ## Empty-State Handling
//!
//! Different formatters handle empty result sets differently based on their use case:
//!
//! | Format | Empty State Behavior | Example | Rationale |
//! |--------|---------------------|---------|-----------|
//! | JSON | Valid empty structure | `[]` | Machine parseable - valid JSON |
//! | XML | Valid empty container | `<items></items>` | Valid XML structure |
//! | CSV | Headers only, no data | `Name,Status\n` | Valid CSV - pipelines can parse headers |
//! | Table | Human message | `No items found.` | Interactive format needs human feedback |
//!
//! This ensures:
//! - Machine-readable formats (JSON, XML, CSV) produce valid, parseable output
//! - Human-facing formats (Table) provide clear feedback
//!
//! See RQ-0359 for the standardization effort.
//!
//! ## Missing/Null Value Handling
//!
//! All formatters use a consistent representation for missing, null, or empty values:
//!
//! | Format | Missing Value Representation | Example |
//! |--------|------------------------------|---------|
//! | JSON | `null` (via serde) | `"field": null` |
//! | XML | `"N/A"` string | `<field>N/A</field>` |
//! | CSV | `N/A` | `field_name,N/A` |
//! | Table | `N/A` | `Field: N/A` |
//!
//! This ensures predictable behavior when parsing output programmatically.
//! Use `common::DEFAULT_MISSING_VALUE` constant when implementing new formatters.
//!
//! See RQ-0399 for the standardization effort.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use splunk_client::models::{
    AuditEvent, ConfigFile, ConfigStanza, Dashboard, DataModel, FiredAlert, Input,
    KvStoreCollection, KvStoreRecord, LogEntry, SearchPeer, WorkloadPool, WorkloadRule,
};
use splunk_client::{
    App, ClusterPeer, Forwarder, Index, KvStoreStatus, LicensePool, LicenseStack, LicenseUsage,
    SavedSearch, SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;

pub use splunk_client::HealthCheckOutput;

// Re-export doctor types for formatters
pub use crate::commands::doctor::{CheckStatus, DiagnosticReport};

mod common;
mod csv;
mod json;
mod macros;
mod ndjson;
mod resource_impls;
mod table;
mod xml;

// Macros are exported at crate root via #[macro_export]

pub use common::{escape_xml, output_result, write_to_file};
pub use csv::CsvFormatter;
pub use json::JsonFormatter;
pub use ndjson::NdjsonFormatter;
pub use table::{Pagination, TableFormatter};
pub use xml::XmlFormatter;

/// Supported output formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum OutputFormat {
    Json,
    Table,
    Csv,
    Xml,
    Ndjson,
}

impl OutputFormat {
    /// Parse from string.
    pub fn from_str(s: &str) -> Result<Self> {
        match s.to_lowercase().as_str() {
            "json" => Ok(OutputFormat::Json),
            "table" => Ok(OutputFormat::Table),
            "csv" => Ok(OutputFormat::Csv),
            "xml" => Ok(OutputFormat::Xml),
            "ndjson" | "jsonl" => Ok(OutputFormat::Ndjson),
            _ => anyhow::bail!(
                "Invalid output format: {}. Valid options: json, table, csv, xml, ndjson",
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

    /// Format diagnostic report from doctor command.
    fn format_health_check_report(&self, report: &DiagnosticReport) -> Result<String>;

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

    /// Format data models list.
    fn format_datamodels(&self, datamodels: &[DataModel], detailed: bool) -> Result<String>;

    /// Format detailed data model information.
    fn format_datamodel(&self, datamodel: &DataModel) -> Result<String>;

    /// Format workload pools list.
    fn format_workload_pools(&self, pools: &[WorkloadPool], detailed: bool) -> Result<String>;

    /// Format workload rules list.
    fn format_workload_rules(&self, rules: &[WorkloadRule], detailed: bool) -> Result<String>;

    /// Format SHC status.
    fn format_shc_status(&self, status: &ShcStatusOutput) -> Result<String>;

    /// Format SHC members list.
    fn format_shc_members(
        &self,
        members: &[ShcMemberOutput],
        pagination: &Pagination,
    ) -> Result<String>;

    /// Format SHC captain.
    fn format_shc_captain(&self, captain: &ShcCaptainOutput) -> Result<String>;

    /// Format SHC config.
    fn format_shc_config(&self, config: &ShcConfigOutput) -> Result<String>;

    /// Format SHC management operation result.
    fn format_shc_management(&self, output: &ShcManagementOutput) -> Result<String>;
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
            status: peer.status.to_string(),
            peer_state: peer.peer_state.to_string(),
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

/// SHC member output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShcMemberOutput {
    pub id: String,
    pub host: String,
    pub port: u32,
    pub status: String,
    pub is_captain: bool,
    pub guid: String,
    pub site: Option<String>,
}

impl From<splunk_client::ShcMember> for ShcMemberOutput {
    fn from(member: splunk_client::ShcMember) -> Self {
        Self {
            id: member.id,
            host: member.host,
            port: member.port,
            status: member.status.to_string(),
            is_captain: member.is_captain,
            guid: member.guid,
            site: member.site,
        }
    }
}

/// SHC captain output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShcCaptainOutput {
    pub id: String,
    pub host: String,
    pub port: u32,
    pub guid: String,
    pub is_dynamic_captain: bool,
    pub site: Option<String>,
}

impl From<splunk_client::ShcCaptain> for ShcCaptainOutput {
    fn from(captain: splunk_client::ShcCaptain) -> Self {
        Self {
            id: captain.id,
            host: captain.host,
            port: captain.port,
            guid: captain.guid,
            is_dynamic_captain: captain.is_dynamic_captain,
            site: captain.site,
        }
    }
}

/// SHC status output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShcStatusOutput {
    pub is_captain: bool,
    pub is_searchable: bool,
    pub captain_uri: Option<String>,
    pub member_count: u32,
    pub minimum_member_count: Option<u32>,
    pub rolling_restart_flag: Option<bool>,
    pub service_ready_flag: Option<bool>,
}

impl From<splunk_client::ShcStatus> for ShcStatusOutput {
    fn from(status: splunk_client::ShcStatus) -> Self {
        Self {
            is_captain: status.is_captain,
            is_searchable: status.is_searchable,
            captain_uri: status.captain_uri,
            member_count: status.member_count,
            minimum_member_count: status.minimum_member_count,
            rolling_restart_flag: status.rolling_restart_flag,
            service_ready_flag: status.service_ready_flag,
        }
    }
}

/// SHC config output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShcConfigOutput {
    pub id: String,
    pub replication_factor: Option<u32>,
    pub captain_uri: Option<String>,
    pub shcluster_label: Option<String>,
}

impl From<splunk_client::ShcConfig> for ShcConfigOutput {
    fn from(config: splunk_client::ShcConfig) -> Self {
        Self {
            id: config.id,
            replication_factor: config.replication_factor,
            captain_uri: config.captain_uri,
            shcluster_label: config.shcluster_label,
        }
    }
}

/// SHC management operation output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShcManagementOutput {
    pub operation: String,
    pub target: String,
    pub success: bool,
    pub message: String,
}

/// Trait for types that can be displayed in tabular formats (CSV, Table, XML).
///
/// Resources implement this trait to define their display schema once,
/// and all formatters automatically support them.
///
/// This trait supports format-specific customizations through separate methods
/// for CSV, Table, and XML output formats. Each method has a default implementation
/// that delegates to the standard methods, so types only need to override
/// the formats where they differ.
///
/// # Example
/// ```ignore
/// impl ResourceDisplay for User {
///     fn headers(_detailed: bool) -> Vec<&'static str> {
///         vec!["Name", "Real Name", "Type", "Roles"]
///     }
///
///     fn headers_csv(detailed: bool) -> Vec<&'static str> {
///         // CSV uses lowercase headers
///         Self::headers(detailed).into_iter()
///             .map(|h| h.to_lowercase().leak())
///             .collect()
///     }
///
///     fn headers_table(detailed: bool) -> Vec<&'static str> {
///         // Table uses UPPERCASE headers
///         Self::headers(detailed).into_iter()
///             .map(|h| h.to_uppercase().leak())
///             .collect()
///     }
///
///     fn row_data(&self, _detailed: bool) -> Vec<Vec<String>> {
///         vec![vec![
///             self.name.clone(),
///             self.realname.clone().unwrap_or_default(),
///             // ... more fields
///         ]]
///     }
///
///     fn xml_element_name() -> &'static str {
///         "user"
///     }
///
///     fn xml_fields(&self) -> Vec<(&'static str, Option<String>)> {
///         vec![
///             ("name", Some(self.name.clone())),
///             ("email", self.email.clone()),
///         ]
///     }
/// }
/// ```
pub trait ResourceDisplay {
    /// Returns the column headers for this resource type (default/standard format).
    fn headers(detailed: bool) -> Vec<&'static str>;

    /// Returns the column headers for CSV format.
    /// Default implementation uses the standard headers.
    fn headers_csv(detailed: bool) -> Vec<&'static str> {
        Self::headers(detailed)
    }

    /// Returns the column headers for Table format.
    /// Default implementation uses the standard headers.
    fn headers_table(detailed: bool) -> Vec<&'static str> {
        Self::headers(detailed)
    }

    /// Returns the row data for this instance (default/standard format).
    /// Each inner Vec contains the cell values for one row.
    fn row_data(&self, detailed: bool) -> Vec<Vec<String>>;

    /// Returns the row data for CSV format.
    /// Default implementation delegates to `row_data`.
    fn row_data_csv(&self, detailed: bool) -> Vec<Vec<String>> {
        self.row_data(detailed)
    }

    /// Returns the row data for Table format.
    /// Default implementation delegates to `row_data`.
    fn row_data_table(&self, detailed: bool) -> Vec<Vec<String>> {
        self.row_data(detailed)
    }

    /// Returns the XML element name for this resource type (for XML formatter).
    fn xml_element_name() -> &'static str;

    /// Returns individual fields as (name, value) pairs for XML output.
    /// The Option allows XML to omit empty/unset fields.
    fn xml_fields(&self) -> Vec<(&'static str, Option<String>)>;
}

/// Get a formatter for the specified output format.
pub fn get_formatter(format: OutputFormat) -> Box<dyn Formatter> {
    match format {
        OutputFormat::Json => Box::new(JsonFormatter),
        OutputFormat::Table => Box::new(TableFormatter),
        OutputFormat::Csv => Box::new(CsvFormatter),
        OutputFormat::Xml => Box::new(XmlFormatter),
        OutputFormat::Ndjson => Box::new(NdjsonFormatter),
    }
}

#[cfg(test)]
mod tests;
