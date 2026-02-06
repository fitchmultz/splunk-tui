//! CSV formatter implementation.
//!
//! Responsibilities:
//! - Format resources as RFC 4180 compliant CSV.
//! - Delegate to submodules for specific resource types.
//!
//! Does NOT handle:
//! - Other output formats.
//! - Table-style pagination.
//!
//! Invariants:
//! - CSV output follows RFC 4180 for compatibility with standard tools
//! - Nested structures are flattened using dot notation

use crate::formatters::{
    ClusterInfoOutput, ClusterManagementOutput, ClusterPeerOutput, Formatter, LicenseInfoOutput,
    LicenseInstallOutput, LicensePoolOperationOutput, Pagination, ShcCaptainOutput,
    ShcConfigOutput, ShcManagementOutput, ShcMemberOutput, ShcStatusOutput,
};
use anyhow::Result;
use splunk_client::models::AuditEvent;
use splunk_client::models::{
    ConfigFile, ConfigStanza, DataModel, Input, KvStoreCollection, KvStoreRecord, LogEntry,
    SearchPeer,
};
use splunk_client::{
    App, Dashboard, Forwarder, HealthCheckOutput, Index, KvStoreStatus, SavedSearch,
    SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

use crate::formatters::csv::workload;

use super::alerts;
use super::apps;
use super::cluster;
use super::configs;
use super::dashboards;
use super::datamodels;
use super::forwarders;
use super::health;
use super::hec;
use super::indexes;
use super::inputs;
use super::jobs;
use super::kvstore;
use super::license;
use super::logs;
use super::lookups;
use super::macros;
use super::profiles;
use super::roles;
use super::saved_searches;
use super::search;
use super::search_peers;
use super::shc;
use super::users;

/// CSV formatter.
pub struct CsvFormatter;

impl Formatter for CsvFormatter {
    // Delegated implementations using macros
    crate::impl_delegated_formatter_slice! {
        format_jobs: &[SearchJobStatus] => jobs::format_jobs,
        format_users: &[User] => users::format_users,
        format_saved_searches: &[SavedSearch] => saved_searches::format_saved_searches,
        format_logs: &[LogEntry] => logs::format_logs,
        format_kvstore_collections: &[KvStoreCollection] => kvstore::format_kvstore_collections,
        format_kvstore_records: &[KvStoreRecord] => search::format_kvstore_records,
        format_lookups: &[splunk_client::LookupTable] => lookups::format_lookups,
        format_config_files: &[ConfigFile] => configs::format_config_files,
        format_config_stanzas: &[ConfigStanza] => configs::format_config_stanzas,
        format_fired_alerts: &[splunk_client::models::FiredAlert] => alerts::format_fired_alerts,
        format_roles: &[splunk_client::Role] => roles::format_roles,
        format_capabilities: &[splunk_client::Capability] => roles::format_capabilities,
        format_installed_licenses: &[splunk_client::InstalledLicense] => license::format_installed_licenses,
        format_license_pools: &[splunk_client::LicensePool] => license::format_license_pools,
        format_macros: &[splunk_client::Macro] => macros::format_macros,
    }

    crate::impl_delegated_formatter_slice_detailed! {
        format_indexes: &[Index] => indexes::format_indexes,
        format_forwarders: &[Forwarder] => forwarders::format_forwarders,
        format_search_peers: &[SearchPeer] => search_peers::format_search_peers,
        format_inputs: &[Input] => inputs::format_inputs,
        format_dashboards: &[Dashboard] => dashboards::format_dashboards,
        format_datamodels: &[DataModel] => datamodels::format_datamodels,
        format_workload_pools: &[splunk_client::WorkloadPool] => workload::format_workload_pools,
        format_workload_rules: &[splunk_client::WorkloadRule] => workload::format_workload_rules,
    }

    crate::impl_delegated_formatter_single! {
        format_job_details: &SearchJobStatus => jobs::format_job_details,
        format_health: &HealthCheckOutput => health::format_health,
        format_kvstore_status: &KvStoreStatus => health::format_kvstore_status,
        format_license: &LicenseInfoOutput => license::format_license,
        format_license_install: &LicenseInstallOutput => license::format_license_install,
        format_license_pool_operation: &LicensePoolOperationOutput => license::format_license_pool_operation,
        format_app_info: &App => apps::format_app_info,
        format_saved_search_info: &SavedSearch => saved_searches::format_saved_search_info,
        format_config_stanza: &ConfigStanza => configs::format_config_stanza_detail,
        format_fired_alert_info: &splunk_client::models::FiredAlert => alerts::format_fired_alert_info,
        format_hec_response: &splunk_client::HecResponse => hec::format_hec_response,
        format_hec_batch_response: &splunk_client::HecBatchResponse => hec::format_hec_batch_response,
        format_hec_health: &splunk_client::HecHealth => hec::format_hec_health,
        format_hec_ack_status: &splunk_client::HecAckStatus => hec::format_hec_ack_status,
        format_macro_info: &splunk_client::Macro => macros::format_macro_info,
        format_dashboard: &Dashboard => dashboards::format_dashboard,
        format_datamodel: &DataModel => datamodels::format_datamodel,
        format_shc_status: &ShcStatusOutput => shc::format_shc_status,
        format_shc_captain: &ShcCaptainOutput => shc::format_shc_captain,
        format_shc_config: &ShcConfigOutput => shc::format_shc_config,
        format_shc_management: &ShcManagementOutput => shc::format_shc_management,
    }

    crate::impl_delegated_formatter_streaming! {
        format_logs_streaming: &[LogEntry] => logs::format_logs_streaming,
    }

    crate::impl_csv_formatter! {
        format_apps: &[App] => apps,
    }

    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        search::format_search_results(results)
    }

    fn format_cluster_info(
        &self,
        cluster_info: &ClusterInfoOutput,
        detailed: bool,
    ) -> Result<String> {
        cluster::format_cluster_info(cluster_info, detailed)
    }

    fn format_cluster_management(&self, output: &ClusterManagementOutput) -> Result<String> {
        cluster::format_cluster_management(output)
    }

    fn format_profile(&self, profile_name: &str, profile: &ProfileConfig) -> Result<String> {
        profiles::format_profile(profile_name, profile)
    }

    fn format_profiles(&self, profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
        profiles::format_profiles(profiles)
    }

    fn format_cluster_peers(
        &self,
        _peers: &[ClusterPeerOutput],
        _pagination: &Pagination,
    ) -> Result<String> {
        anyhow::bail!("Failed to format cluster peers: CSV format not supported. Use JSON format.")
    }

    fn format_shc_members(
        &self,
        _members: &[ShcMemberOutput],
        _pagination: &Pagination,
    ) -> Result<String> {
        anyhow::bail!("Failed to format SHC members: CSV format not supported. Use JSON format.")
    }

    fn format_audit_events(&self, events: &[AuditEvent], _detailed: bool) -> Result<String> {
        use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv};

        let mut output = String::new();

        // Header
        output.push_str(&build_csv_header(&[
            "Time",
            "User",
            "Action",
            "Target",
            "Result",
            "Client IP",
            "Details",
        ]));

        // Rows
        for event in events {
            output.push_str(&build_csv_row(&[
                escape_csv(&event.time),
                escape_csv(&event.user),
                escape_csv(&event.action),
                escape_csv(&event.target),
                escape_csv(&event.result),
                escape_csv(&event.client_ip),
                escape_csv(&event.details),
            ]));
        }

        Ok(output)
    }
}
