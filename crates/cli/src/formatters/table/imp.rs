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
use super::pagination::build_pagination_footer;
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
    // Delegated implementations using macros
    crate::impl_delegated_formatter_slice! {
        format_jobs: &[SearchJobStatus] => jobs::format_jobs,
        format_users: &[User] => users::format_users,
        format_saved_searches: &[SavedSearch] => saved_searches::format_saved_searches,
        format_logs: &[splunk_client::models::LogEntry] => logs::format_logs,
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
        format_dashboard: &Dashboard => dashboards::format_dashboard,
        format_datamodel: &DataModel => datamodels::format_datamodel,
        format_shc_status: &ShcStatusOutput => shc::format_shc_status,
        format_shc_captain: &ShcCaptainOutput => shc::format_shc_captain,
        format_shc_config: &ShcConfigOutput => shc::format_shc_config,
        format_shc_management: &ShcManagementOutput => shc::format_shc_management,
        format_macro_info: &splunk_client::Macro => macros::format_macro_info,
    }

    crate::impl_delegated_formatter_streaming! {
        format_logs_streaming: &[splunk_client::models::LogEntry] => logs::format_logs_streaming,
    }

    crate::impl_table_formatter! {
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

    fn format_profile(&self, profile_name: &str, profile: &ProfileConfig) -> Result<String> {
        profiles::format_profile(profile_name, profile)
    }

    fn format_profiles(&self, profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
        profiles::format_profiles(profiles)
    }

    fn format_shc_members(
        &self,
        members: &[ShcMemberOutput],
        pagination: &Pagination,
    ) -> Result<String> {
        shc::format_shc_members(members, pagination)
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

    fn format_kvstore_collections(&self, collections: &[KvStoreCollection]) -> Result<String> {
        super::kvstore::format_kvstore_collections(collections)
    }

    fn format_kvstore_records(&self, records: &[KvStoreRecord]) -> Result<String> {
        super::kvstore::format_kvstore_records(records)
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
}

impl TableFormatter {
    // Paginated formatters using macros
    crate::impl_table_paginated_detailed! {
        format_audit_events_paginated: &[AuditEvent] => events, base: format_audit_events, resource_name: "audit events",
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
