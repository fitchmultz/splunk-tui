//! Table formatter implementation.
//!
//! Responsibilities:
//! - Format resources as tab-separated tables.
//! - Provide paginated variants for interactive use.
//!
//! Does NOT handle:
//! - Other output formats.
//! - File I/O.
//!
//! Invariants:
//! - Tables use tab-separation for consistent terminal alignment
//! - Column widths are calculated for readable output

use crate::formatters::table::workload;
use crate::formatters::{
    ClusterInfoOutput, ClusterManagementOutput, ClusterPeerOutput, Formatter, LicenseInfoOutput,
    LicenseInstallOutput, LicensePoolOperationOutput, ShcCaptainOutput, ShcConfigOutput,
    ShcManagementOutput, ShcMemberOutput, ShcStatusOutput,
};
use anyhow::Result;
use splunk_client::models::AuditEvent;
use splunk_client::models::{
    ConfigFile, ConfigStanza, DataModel, Input, KvStoreCollection, KvStoreRecord, SearchPeer,
};
use splunk_client::{
    App, Dashboard, Forwarder, HealthCheckOutput, Index, KvStoreStatus, SavedSearch,
    SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

use super::alerts;
use super::apps;
use super::cluster;
use super::configs;
use super::dashboards;
use super::datamodels;
use super::forwarders;
use super::health;
use super::indexes;
use super::inputs;
use super::jobs;
use super::license;
use super::logs;
use super::lookups;
use super::macros;
use super::pagination::{build_pagination_footer, format_empty_message};
use super::profiles;
use super::roles;
use super::saved_searches;
use super::search;
use super::search_peers;
use super::shc;
use super::users;

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
        peers: &[ClusterPeerOutput],
        pagination: &Pagination,
    ) -> Result<String> {
        cluster::format_cluster_peers(peers, pagination)
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

    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String> {
        license::format_license(license)
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

    fn format_logs(&self, logs: &[splunk_client::models::LogEntry]) -> Result<String> {
        logs::format_logs(logs)
    }

    fn format_logs_streaming(
        &self,
        logs: &[splunk_client::models::LogEntry],
        is_first: bool,
    ) -> Result<String> {
        logs::format_logs_streaming(logs, is_first)
    }

    fn format_users(&self, users: &[User]) -> Result<String> {
        users::format_users(users)
    }

    crate::impl_table_formatter! {
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

    fn format_kvstore_collections(&self, collections: &[KvStoreCollection]) -> Result<String> {
        super::kvstore::format_kvstore_collections(collections)
    }

    fn format_kvstore_records(&self, records: &[KvStoreRecord]) -> Result<String> {
        super::kvstore::format_kvstore_records(records)
    }

    fn format_hec_response(&self, response: &splunk_client::HecResponse) -> Result<String> {
        super::hec::format_hec_response(response)
    }

    fn format_hec_batch_response(
        &self,
        response: &splunk_client::HecBatchResponse,
    ) -> Result<String> {
        super::hec::format_hec_batch_response(response)
    }

    fn format_hec_health(&self, health: &splunk_client::HecHealth) -> Result<String> {
        super::hec::format_hec_health(health)
    }

    fn format_hec_ack_status(&self, status: &splunk_client::HecAckStatus) -> Result<String> {
        super::hec::format_hec_ack_status(status)
    }

    fn format_macros(&self, macros: &[splunk_client::Macro]) -> Result<String> {
        macros::format_macros(macros)
    }

    fn format_macro_info(&self, macro_info: &splunk_client::Macro) -> Result<String> {
        macros::format_macro_info(macro_info)
    }

    fn format_audit_events(&self, events: &[AuditEvent], _detailed: bool) -> Result<String> {
        if events.is_empty() {
            return Ok("No audit events found.".to_string());
        }

        let mut lines = Vec::new();
        lines.push("Time\tUser\tAction\tTarget\tResult".to_string());

        for event in events {
            lines.push(format!(
                "{}\t{}\t{}\t{}\t{}",
                event.time, event.user, event.action, event.target, event.result
            ));
        }

        Ok(lines.join("\n"))
    }

    fn format_dashboards(&self, dashboards: &[Dashboard], detailed: bool) -> Result<String> {
        dashboards::format_dashboards(dashboards, detailed)
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
        members: &[ShcMemberOutput],
        pagination: &Pagination,
    ) -> Result<String> {
        shc::format_shc_members(members, pagination)
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

impl TableFormatter {
    // Paginated formatters using macros
    crate::impl_table_paginated_detailed! {
        format_indexes_paginated: &[Index] => indexes, base: format_indexes, resource_name: "indexes",
        format_forwarders_paginated: &[Forwarder] => forwarders_list, base: format_forwarders, resource_name: "forwarders",
        format_search_peers_paginated: &[SearchPeer] => peers, base: format_search_peers, resource_name: "search peers",
        format_inputs_paginated: &[Input] => inputs, base: format_inputs, resource_name: "inputs",
        format_dashboards_paginated: &[Dashboard] => dashboards, base: format_dashboards, resource_name: "dashboards",
        format_datamodels_paginated: &[DataModel] => datamodels, base: format_datamodels, resource_name: "data models",
        format_workload_pools_paginated: &[splunk_client::WorkloadPool] => pools, base: format_workload_pools, resource_name: "workload pools",
    }

    crate::impl_table_paginated! {
        format_config_stanzas_paginated: &[ConfigStanza] => stanzas, base: format_config_stanzas, resource_name: "config stanzas",
        format_kvstore_collections_paginated: &[KvStoreCollection] => collections, base: format_kvstore_collections, resource_name: "KVStore collections",
    }

    /// Table-only formatter for audit events with pagination footer.
    pub fn format_audit_events_paginated(
        &self,
        events: &[AuditEvent],
        _detailed: bool,
        pagination: Pagination,
    ) -> Result<String> {
        if events.is_empty() {
            return Ok(format_empty_message("audit events", pagination.offset));
        }

        let mut output = Formatter::format_audit_events(self, events, _detailed)?;

        if let Some(footer) = build_pagination_footer(pagination, events.len()) {
            output.push('\n');
            output.push_str(&footer);
            output.push('\n');
        }

        Ok(output)
    }

    /// Table-only formatter for SHC status with pagination footer (members only).
    #[allow(clippy::collapsible_if)]
    pub fn format_shc_status_paginated(
        &self,
        shc_status: &ShcStatusOutput,
        detailed: bool,
        members_output: Option<Vec<ShcMemberOutput>>,
        members_pagination: Option<Pagination>,
    ) -> Result<String> {
        let mut output = format!(
            "SHC Status:\n\
             Is Captain: {}\n\
             Is Searchable: {}\n\
             Captain URI: {}\n\
             Member Count: {}\n\
             Minimum Member Count: {}\n\
             Rolling Restart: {}\n\
             Service Ready: {}\n",
            shc_status.is_captain,
            shc_status.is_searchable,
            shc_status.captain_uri.as_deref().unwrap_or("N/A"),
            shc_status.member_count,
            shc_status
                .minimum_member_count
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            shc_status
                .rolling_restart_flag
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
            shc_status
                .service_ready_flag
                .map(|v| v.to_string())
                .unwrap_or_else(|| "N/A".to_string()),
        );

        if detailed {
            if let Some(members) = members_output {
                output.push_str("\nSHC Members:\n");

                if members.is_empty() {
                    if let Some(p) = members_pagination
                        && let Some(total) = p.total
                        && total > 0
                        && p.offset >= total
                    {
                        output.push_str(&format!(
                            "  No members found for offset {} (total {}).\n",
                            p.offset, total
                        ));
                    } else {
                        output.push_str("  No members found.\n");
                    }
                } else {
                    output.push_str("  Host\t\tStatus\tCaptain\tGUID\t\tSite\n");
                    output.push_str("  ----\t\t------\t-------\t----\t\t----\n");
                    for member in &members {
                        let captain_marker = if member.is_captain { "Yes" } else { "" };
                        output.push_str(&format!(
                            "  {}:{}\t{}\t{}\t{}\t{}\n",
                            member.host,
                            member.port,
                            member.status,
                            captain_marker,
                            &member.guid[..member.guid.len().min(8)],
                            member.site.as_deref().unwrap_or("-"),
                        ));
                    }
                }

                if let Some(p) = members_pagination
                    && let Some(footer) = build_pagination_footer(p, members.len())
                {
                    output.push('\n');
                    output.push_str(&footer);
                    output.push('\n');
                }
            }
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
