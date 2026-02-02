//! Table formatter implementation.
//!
//! Responsibilities:
//! - Format resources as tab-separated tables.
//! - Provide paginated variants for interactive use.
//!
//! Does NOT handle:
//! - Other output formats.
//! - File I/O.

use crate::formatters::table::workload;
use crate::formatters::{
    ClusterInfoOutput, ClusterManagementOutput, ClusterPeerOutput, Formatter, LicenseInfoOutput,
    LicenseInstallOutput, LicensePoolOperationOutput,
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
use super::list_all;
use super::logs;
use super::lookups;
use super::macros;
use super::profiles;
use super::roles;
use super::saved_searches;
use super::search;
use super::search_peers;
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

    fn format_apps(&self, apps: &[App]) -> Result<String> {
        apps::format_apps(apps)
    }

    fn format_app_info(&self, app: &App) -> Result<String> {
        apps::format_app_info(app)
    }

    fn format_list_all(&self, output: &crate::commands::list_all::ListAllOutput) -> Result<String> {
        list_all::format_list_all(output)
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

        // Tab-separated table format
        let mut lines = Vec::new();

        // Header
        lines.push("Time\tUser\tAction\tTarget\tResult".to_string());

        // Rows
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
}

impl TableFormatter {
    /// Table-only formatter for indexes with pagination footer.
    ///
    /// NOTE: This does not attempt to discover a server-side total for indexes (not exposed by the
    /// current client API return type). Footer omits total/page-count when `total` is None.
    pub fn format_indexes_paginated(
        &self,
        indexes: &[Index],
        detailed: bool,
        pagination: Pagination,
    ) -> Result<String> {
        if indexes.is_empty() {
            if pagination.offset > 0 {
                return Ok(format!(
                    "No indexes found for offset {}.",
                    pagination.offset
                ));
            }
            return Ok("No indexes found.".to_string());
        }

        // Reuse existing table rendering, then append footer.
        let mut output = self.format_indexes(indexes, detailed)?;

        if let Some(footer) = build_pagination_footer(pagination, indexes.len()) {
            output.push('\n');
            output.push_str(&footer);
            output.push('\n');
        }

        Ok(output)
    }

    /// Table-only formatter for forwarders with pagination footer.
    ///
    /// NOTE: This does not attempt to discover a server-side total for forwarders (not exposed by the
    /// current client API return type). Footer omits total/page-count when `total` is None.
    pub fn format_forwarders_paginated(
        &self,
        forwarders_list: &[Forwarder],
        detailed: bool,
        pagination: Pagination,
    ) -> Result<String> {
        if forwarders_list.is_empty() {
            if pagination.offset > 0 {
                return Ok(format!(
                    "No forwarders found for offset {}.",
                    pagination.offset
                ));
            }
            return Ok("No forwarders found.".to_string());
        }

        // Reuse existing table rendering, then append footer.
        let mut output = self.format_forwarders(forwarders_list, detailed)?;

        if let Some(footer) = build_pagination_footer(pagination, forwarders_list.len()) {
            output.push('\n');
            output.push_str(&footer);
            output.push('\n');
        }

        Ok(output)
    }

    /// Table-only formatter for search peers with pagination footer.
    ///
    /// NOTE: This does not attempt to discover a server-side total for search peers (not exposed by the
    /// current client API return type). Footer omits total/page-count when `total` is None.
    pub fn format_search_peers_paginated(
        &self,
        peers: &[SearchPeer],
        detailed: bool,
        pagination: Pagination,
    ) -> Result<String> {
        if peers.is_empty() {
            if pagination.offset > 0 {
                return Ok(format!(
                    "No search peers found for offset {}.",
                    pagination.offset
                ));
            }
            return Ok("No search peers found.".to_string());
        }

        // Reuse existing table rendering, then append footer.
        let mut output = self.format_search_peers(peers, detailed)?;

        if let Some(footer) = build_pagination_footer(pagination, peers.len()) {
            output.push('\n');
            output.push_str(&footer);
            output.push('\n');
        }

        Ok(output)
    }

    /// Table-only formatter for inputs with pagination footer.
    ///
    /// NOTE: This does not attempt to discover a server-side total for inputs (not exposed by the
    /// current client API return type). Footer omits total/page-count when `total` is None.
    pub fn format_inputs_paginated(
        &self,
        inputs: &[Input],
        detailed: bool,
        pagination: Pagination,
    ) -> Result<String> {
        if inputs.is_empty() {
            if pagination.offset > 0 {
                return Ok(format!("No inputs found for offset {}.", pagination.offset));
            }
            return Ok("No inputs found.".to_string());
        }

        // Reuse existing table rendering, then append footer.
        let mut output = self.format_inputs(inputs, detailed)?;

        if let Some(footer) = build_pagination_footer(pagination, inputs.len()) {
            output.push('\n');
            output.push_str(&footer);
            output.push('\n');
        }

        Ok(output)
    }

    /// Table-only formatter for config stanzas with pagination footer.
    ///
    /// NOTE: This does not attempt to discover a server-side total for config stanzas (not exposed by the
    /// current client API return type). Footer omits total/page-count when `total` is None.
    pub fn format_config_stanzas_paginated(
        &self,
        stanzas: &[ConfigStanza],
        pagination: Pagination,
    ) -> Result<String> {
        if stanzas.is_empty() {
            if pagination.offset > 0 {
                return Ok(format!(
                    "No config stanzas found for offset {}.",
                    pagination.offset
                ));
            }
            return Ok("No config stanzas found.".to_string());
        }

        // Reuse existing table rendering, then append footer.
        let mut output = self.format_config_stanzas(stanzas)?;

        if let Some(footer) = build_pagination_footer(pagination, stanzas.len()) {
            output.push('\n');
            output.push_str(&footer);
            output.push('\n');
        }

        Ok(output)
    }

    /// Table-only formatter for KVStore collections with pagination footer.
    ///
    /// NOTE: This does not attempt to discover a server-side total for collections (not exposed by the
    /// current client API return type). Footer omits total/page-count when `total` is None.
    pub fn format_kvstore_collections_paginated(
        &self,
        collections: &[KvStoreCollection],
        pagination: Pagination,
    ) -> Result<String> {
        if collections.is_empty() {
            if pagination.offset > 0 {
                return Ok(format!(
                    "No KVStore collections found for offset {}.",
                    pagination.offset
                ));
            }
            return Ok("No KVStore collections found.".to_string());
        }

        // Reuse existing table rendering, then append footer.
        let mut output = self.format_kvstore_collections(collections)?;

        if let Some(footer) = build_pagination_footer(pagination, collections.len()) {
            output.push('\n');
            output.push_str(&footer);
            output.push('\n');
        }

        Ok(output)
    }

    /// Table-only formatter for dashboards with pagination footer.
    ///
    /// NOTE: This does not attempt to discover a server-side total for dashboards (not exposed by the
    /// current client API return type). Footer omits total/page-count when `total` is None.
    pub fn format_dashboards_paginated(
        &self,
        dashboards: &[Dashboard],
        detailed: bool,
        pagination: Pagination,
    ) -> Result<String> {
        if dashboards.is_empty() {
            if pagination.offset > 0 {
                return Ok(format!(
                    "No dashboards found for offset {}.",
                    pagination.offset
                ));
            }
            return Ok("No dashboards found.".to_string());
        }

        // Reuse existing table rendering, then append footer.
        let mut output = self.format_dashboards(dashboards, detailed)?;

        if let Some(footer) = build_pagination_footer(pagination, dashboards.len()) {
            output.push('\n');
            output.push_str(&footer);
            output.push('\n');
        }

        Ok(output)
    }

    /// Table-only formatter for data models with pagination footer.
    ///
    /// NOTE: This does not attempt to discover a server-side total for data models (not exposed by the
    /// current client API return type). Footer omits total/page-count when `total` is None.
    pub fn format_datamodels_paginated(
        &self,
        datamodels: &[DataModel],
        detailed: bool,
        pagination: Pagination,
    ) -> Result<String> {
        if datamodels.is_empty() {
            if pagination.offset > 0 {
                return Ok(format!(
                    "No data models found for offset {}.",
                    pagination.offset
                ));
            }
            return Ok("No data models found.".to_string());
        }

        // Reuse existing table rendering, then append footer.
        let mut output = self.format_datamodels(datamodels, detailed)?;

        if let Some(footer) = build_pagination_footer(pagination, datamodels.len()) {
            output.push('\n');
            output.push_str(&footer);
            output.push('\n');
        }

        Ok(output)
    }

    /// Table-only formatter for audit events with pagination footer.
    ///
    /// NOTE: This does not attempt to discover a server-side total for audit events (not exposed by the
    /// current client API return type). Footer omits total/page-count when `total` is None.
    pub fn format_audit_events_paginated(
        &self,
        events: &[AuditEvent],
        _detailed: bool,
        pagination: Pagination,
    ) -> Result<String> {
        if events.is_empty() {
            if pagination.offset > 0 {
                return Ok(format!(
                    "No audit events found for offset {}.",
                    pagination.offset
                ));
            }
            return Ok("No audit events found.".to_string());
        }

        // Reuse existing table rendering, then append footer.
        let mut output = Formatter::format_audit_events(self, events, _detailed)?;

        if let Some(footer) = build_pagination_footer(pagination, events.len()) {
            output.push('\n');
            output.push_str(&footer);
            output.push('\n');
        }

        Ok(output)
    }

    /// Table-only formatter for workload pools with pagination footer.
    ///
    /// NOTE: This does not attempt to discover a server-side total for workload pools (not exposed by the
    /// current client API return type). Footer omits total/page-count when `total` is None.
    pub fn format_workload_pools_paginated(
        &self,
        pools: &[splunk_client::WorkloadPool],
        detailed: bool,
        pagination: Pagination,
    ) -> Result<String> {
        if pools.is_empty() {
            if pagination.offset > 0 {
                return Ok(format!(
                    "No workload pools found for offset {}.",
                    pagination.offset
                ));
            }
            return Ok("No workload pools found.".to_string());
        }

        // Reuse existing table rendering, then append footer.
        let mut output = self.format_workload_pools(pools, detailed)?;

        if let Some(footer) = build_pagination_footer(pagination, pools.len()) {
            output.push('\n');
            output.push_str(&footer);
            output.push('\n');
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
                    // Offset out of range is especially important to explain in table output.
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

/// Build a pagination footer string.
///
/// - `offset` is zero-based
/// - `page_size` is the requested page size
/// - `total` is optional; when absent, footer omits total/page-count
pub fn build_pagination_footer(p: Pagination, shown: usize) -> Option<String> {
    if p.page_size == 0 {
        // Avoid division by zero; caller should validate for client-side pagination.
        return None;
    }

    // If nothing is shown, caller should usually emit a friendlier message.
    if shown == 0 {
        if let Some(total) = p.total {
            if total == 0 {
                return Some("No results.".to_string());
            }
            if p.offset >= total {
                return Some(format!(
                    "Showing 0 of {} (offset {} out of range)",
                    total, p.offset
                ));
            }
            return Some(format!("Showing 0 of {}", total));
        }
        return Some("No results.".to_string());
    }

    let start = p.offset.saturating_add(1);
    let end = p.offset.saturating_add(shown);
    let page = (p.offset / p.page_size).saturating_add(1);

    if let Some(total) = p.total {
        let total_pages: usize = if total == 0 {
            0
        } else {
            (total.saturating_add(p.page_size).saturating_sub(1)) / p.page_size
        };
        Some(format!(
            "Showing {}-{} of {} (page {} of {})",
            start, end, total, page, total_pages
        ))
    } else {
        Some(format!("Showing {}-{} (page {})", start, end, page))
    }
}
