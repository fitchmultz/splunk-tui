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
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        search::format_search_results(results)
    }

    fn format_indexes(&self, indexes: &[Index], detailed: bool) -> Result<String> {
        indexes::format_indexes(indexes, detailed)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        jobs::format_jobs(jobs)
    }

    fn format_cluster_info(
        &self,
        cluster_info: &ClusterInfoOutput,
        detailed: bool,
    ) -> Result<String> {
        cluster::format_cluster_info(cluster_info, detailed)
    }

    fn format_cluster_peers(
        &self,
        _peers: &[ClusterPeerOutput],
        _pagination: &Pagination,
    ) -> Result<String> {
        // CSV doesn't support paginated peer lists; use JSON for programmatic access
        anyhow::bail!("CSV format not supported for cluster peers. Use JSON format.")
    }

    fn format_cluster_management(&self, output: &ClusterManagementOutput) -> Result<String> {
        cluster::format_cluster_management(output)
    }

    fn format_health(&self, health: &HealthCheckOutput) -> Result<String> {
        health::format_health(health)
    }

    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String> {
        health::format_kvstore_status(status)
    }

    fn format_kvstore_collections(&self, collections: &[KvStoreCollection]) -> Result<String> {
        kvstore::format_kvstore_collections(collections)
    }

    fn format_kvstore_records(&self, records: &[KvStoreRecord]) -> Result<String> {
        search::format_kvstore_records(records)
    }

    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String> {
        license::format_license(license)
    }

    fn format_logs(&self, logs: &[LogEntry]) -> Result<String> {
        logs::format_logs(logs)
    }

    fn format_logs_streaming(&self, logs: &[LogEntry], is_first: bool) -> Result<String> {
        logs::format_logs_streaming(logs, is_first)
    }

    fn format_users(&self, users: &[User]) -> Result<String> {
        users::format_users(users)
    }

    crate::impl_csv_formatter! {
        format_apps: &[App] => apps,
    }

    fn format_app_info(&self, app: &App) -> Result<String> {
        apps::format_app_info(app)
    }

    fn format_saved_searches(&self, searches: &[SavedSearch]) -> Result<String> {
        saved_searches::format_saved_searches(searches)
    }

    fn format_saved_search_info(&self, search: &SavedSearch) -> Result<String> {
        saved_searches::format_saved_search_info(search)
    }

    fn format_job_details(&self, job: &SearchJobStatus) -> Result<String> {
        jobs::format_job_details(job)
    }

    fn format_lookups(&self, lookups: &[splunk_client::LookupTable]) -> Result<String> {
        lookups::format_lookups(lookups)
    }

    fn format_profile(&self, profile_name: &str, profile: &ProfileConfig) -> Result<String> {
        profiles::format_profile(profile_name, profile)
    }

    fn format_profiles(&self, profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
        profiles::format_profiles(profiles)
    }

    fn format_forwarders(&self, forwarders_list: &[Forwarder], detailed: bool) -> Result<String> {
        forwarders::format_forwarders(forwarders_list, detailed)
    }

    fn format_search_peers(&self, peers: &[SearchPeer], detailed: bool) -> Result<String> {
        search_peers::format_search_peers(peers, detailed)
    }

    fn format_inputs(&self, inputs: &[Input], detailed: bool) -> Result<String> {
        inputs::format_inputs(inputs, detailed)
    }

    fn format_config_files(&self, files: &[ConfigFile]) -> Result<String> {
        configs::format_config_files(files)
    }

    fn format_config_stanzas(&self, stanzas: &[ConfigStanza]) -> Result<String> {
        configs::format_config_stanzas(stanzas)
    }

    fn format_config_stanza(&self, stanza: &ConfigStanza) -> Result<String> {
        configs::format_config_stanza_detail(stanza)
    }

    fn format_fired_alerts(&self, alerts: &[splunk_client::models::FiredAlert]) -> Result<String> {
        alerts::format_fired_alerts(alerts)
    }

    fn format_fired_alert_info(&self, alert: &splunk_client::models::FiredAlert) -> Result<String> {
        alerts::format_fired_alert_info(alert)
    }

    fn format_roles(&self, roles: &[splunk_client::Role]) -> Result<String> {
        roles::format_roles(roles)
    }

    fn format_capabilities(&self, capabilities: &[splunk_client::Capability]) -> Result<String> {
        roles::format_capabilities(capabilities)
    }

    fn format_installed_licenses(
        &self,
        licenses: &[splunk_client::InstalledLicense],
    ) -> Result<String> {
        license::format_installed_licenses(licenses)
    }

    fn format_license_install(&self, result: &LicenseInstallOutput) -> Result<String> {
        license::format_license_install(result)
    }

    fn format_license_pools(&self, pools: &[splunk_client::LicensePool]) -> Result<String> {
        license::format_license_pools(pools)
    }

    fn format_license_pool_operation(&self, result: &LicensePoolOperationOutput) -> Result<String> {
        license::format_license_pool_operation(result)
    }

    fn format_hec_response(&self, response: &splunk_client::HecResponse) -> Result<String> {
        hec::format_hec_response(response)
    }

    fn format_hec_batch_response(
        &self,
        response: &splunk_client::HecBatchResponse,
    ) -> Result<String> {
        hec::format_hec_batch_response(response)
    }

    fn format_hec_health(&self, health: &splunk_client::HecHealth) -> Result<String> {
        hec::format_hec_health(health)
    }

    fn format_hec_ack_status(&self, status: &splunk_client::HecAckStatus) -> Result<String> {
        hec::format_hec_ack_status(status)
    }

    fn format_macros(&self, macros: &[splunk_client::Macro]) -> Result<String> {
        macros::format_macros(macros)
    }

    fn format_macro_info(&self, macro_info: &splunk_client::Macro) -> Result<String> {
        macros::format_macro_info(macro_info)
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

    fn format_dashboards(&self, dashboards_list: &[Dashboard], detailed: bool) -> Result<String> {
        dashboards::format_dashboards(dashboards_list, detailed)
    }

    fn format_dashboard(&self, dashboard: &Dashboard) -> Result<String> {
        dashboards::format_dashboard(dashboard)
    }

    fn format_datamodels(&self, datamodels_list: &[DataModel], detailed: bool) -> Result<String> {
        datamodels::format_datamodels(datamodels_list, detailed)
    }

    fn format_datamodel(&self, datamodel: &DataModel) -> Result<String> {
        datamodels::format_datamodel(datamodel)
    }

    fn format_workload_pools(
        &self,
        pools: &[splunk_client::WorkloadPool],
        detailed: bool,
    ) -> Result<String> {
        workload::format_workload_pools(pools, detailed)
    }

    fn format_workload_rules(
        &self,
        rules: &[splunk_client::WorkloadRule],
        detailed: bool,
    ) -> Result<String> {
        workload::format_workload_rules(rules, detailed)
    }

    fn format_shc_status(&self, status: &ShcStatusOutput) -> Result<String> {
        shc::format_shc_status(status)
    }

    fn format_shc_members(
        &self,
        _members: &[ShcMemberOutput],
        _pagination: &Pagination,
    ) -> Result<String> {
        anyhow::bail!("CSV format not supported for SHC members. Use JSON format.")
    }

    fn format_shc_captain(&self, captain: &ShcCaptainOutput) -> Result<String> {
        shc::format_shc_captain(captain)
    }

    fn format_shc_config(&self, config: &ShcConfigOutput) -> Result<String> {
        shc::format_shc_config(config)
    }

    fn format_shc_management(&self, output: &ShcManagementOutput) -> Result<String> {
        shc::format_shc_management(output)
    }
}
