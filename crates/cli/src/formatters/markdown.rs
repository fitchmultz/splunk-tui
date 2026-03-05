//! Markdown formatter implementation.
//!
//! Responsibilities:
//! - Format resources as Markdown tables and sections.
//! - Human-readable documentation format.
//!
//! Does NOT handle:
//! - Other output formats.
//! - Complex nested structures (flattens to tables).

use crate::formatters::{
    ClusterInfoOutput, ClusterManagementOutput, ClusterPeerOutput, Formatter, LicenseInfoOutput,
    LicenseInstallOutput, LicensePoolOperationOutput, Pagination, ShcCaptainOutput,
    ShcConfigOutput, ShcManagementOutput, ShcMemberOutput, ShcStatusOutput,
};
use anyhow::Result;
use serde::Serialize;
use splunk_client::models::{
    AuditEvent, ConfigFile, ConfigStanza, Dashboard, DataModel, Input, KvStoreCollection,
    KvStoreRecord, LogEntry, SearchPeer,
};
use splunk_client::{
    App, Forwarder, HealthCheckOutput, Index, KvStoreStatus, SavedSearch, SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// Markdown formatter.
pub struct MarkdownFormatter;

/// Helper to escape markdown special characters in cell content
fn escape_markdown(text: &str) -> String {
    text.replace('|', "\\|")
}

/// Build a markdown table from headers and rows
fn build_markdown_table(headers: &[String], rows: &[Vec<String>]) -> String {
    if rows.is_empty() {
        return "_No data available._\n".to_string();
    }

    let mut output = String::new();

    // Header row
    output.push('|');
    for header in headers {
        output.push(' ');
        output.push_str(header);
        output.push(' ');
        output.push('|');
    }
    output.push('\n');

    // Separator row
    output.push('|');
    for _ in headers {
        output.push(' ');
        output.push_str("---");
        output.push(' ');
        output.push('|');
    }
    output.push('\n');

    // Data rows
    for row in rows {
        output.push('|');
        for cell in row {
            output.push(' ');
            output.push_str(&escape_markdown(cell));
            output.push(' ');
            output.push('|');
        }
        output.push('\n');
    }

    output
}

/// Convert a JSON value to a string representation for markdown
fn value_to_string(value: &serde_json::Value) -> String {
    match value {
        serde_json::Value::Null => "_".to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Array(arr) => format!("[{} items]", arr.len()),
        serde_json::Value::Object(obj) => format!("[{} fields]", obj.len()),
    }
}

/// Convert a slice of serializable items to markdown table
fn to_markdown_table<T: Serialize>(items: &[T], title: &str) -> Result<String> {
    let mut output = format!("# {}\n\n", title);

    // Serialize to JSON Value first
    let value = serde_json::to_value(items)?;

    let array = match value {
        serde_json::Value::Array(arr) => arr,
        _ => {
            output.push_str(&format!(
                "```json\n{}\n```\n",
                serde_json::to_string_pretty(items)?
            ));
            return Ok(output);
        }
    };

    if array.is_empty() {
        output.push_str("_No items found._\n");
        return Ok(output);
    }

    // Collect all unique keys from all objects
    let mut all_keys: Vec<String> = Vec::new();
    for item in &array {
        if let serde_json::Value::Object(obj) = item {
            for key in obj.keys() {
                if !all_keys.contains(key) {
                    all_keys.push(key.clone());
                }
            }
        }
    }

    if all_keys.is_empty() {
        output.push_str("_No data available._\n");
        return Ok(output);
    }

    // Build rows
    let rows: Vec<Vec<String>> = array
        .iter()
        .map(|item| {
            if let serde_json::Value::Object(obj) = item {
                all_keys
                    .iter()
                    .map(|key| value_to_string(obj.get(key).unwrap_or(&serde_json::Value::Null)))
                    .collect()
            } else {
                vec![value_to_string(item)]
            }
        })
        .collect();

    output.push_str(&build_markdown_table(&all_keys, &rows));
    Ok(output)
}

/// Convert a single serializable item to markdown section
fn to_markdown_section<T: Serialize>(item: &T, title: &str) -> Result<String> {
    let mut output = format!("# {}\n\n", title);

    let value = serde_json::to_value(item)?;

    if let serde_json::Value::Object(obj) = value {
        for (key, val) in obj {
            output.push_str(&format!("- **{}**: {}\n", key, value_to_string(&val)));
        }
    } else {
        output.push_str(&format!(
            "```json\n{}\n```\n",
            serde_json::to_string_pretty(item)?
        ));
    }

    output.push('\n');
    Ok(output)
}

impl Formatter for MarkdownFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        let mut output = "# Search Results\n\n".to_string();

        if results.is_empty() {
            output.push_str("_No search results found._\n");
            return Ok(output);
        }

        // Collect all unique keys
        let mut all_keys: Vec<String> = Vec::new();
        for result in results {
            if let serde_json::Value::Object(obj) = result {
                for key in obj.keys() {
                    if !all_keys.contains(key) {
                        all_keys.push(key.clone());
                    }
                }
            }
        }

        if all_keys.is_empty() {
            output.push_str("_No data available._\n");
            return Ok(output);
        }

        // Build rows
        let rows: Vec<Vec<String>> = results
            .iter()
            .map(|result| {
                if let serde_json::Value::Object(obj) = result {
                    all_keys
                        .iter()
                        .map(|key| {
                            value_to_string(obj.get(key).unwrap_or(&serde_json::Value::Null))
                        })
                        .collect()
                } else {
                    vec![value_to_string(result)]
                }
            })
            .collect();

        output.push_str(&build_markdown_table(&all_keys, &rows));
        Ok(output)
    }

    fn format_indexes(&self, indexes: &[Index], _detailed: bool) -> Result<String> {
        to_markdown_table(indexes, "Indexes")
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        to_markdown_table(jobs, "Search Jobs")
    }

    fn format_cluster_info(
        &self,
        cluster_info: &ClusterInfoOutput,
        _detailed: bool,
    ) -> Result<String> {
        to_markdown_section(cluster_info, "Cluster Information")
    }

    fn format_cluster_peers(
        &self,
        peers: &[ClusterPeerOutput],
        _pagination: &Pagination,
    ) -> Result<String> {
        to_markdown_table(peers, "Cluster Peers")
    }

    fn format_cluster_management(&self, output: &ClusterManagementOutput) -> Result<String> {
        to_markdown_section(output, "Cluster Management Operation")
    }

    fn format_health(&self, health: &HealthCheckOutput) -> Result<String> {
        to_markdown_section(health, "Health Check Results")
    }

    fn format_health_check_report(
        &self,
        report: &crate::formatters::DiagnosticReport,
    ) -> Result<String> {
        let mut output = "# Diagnostic Report\n\n".to_string();
        output.push_str(&format!("**CLI Version**: {}\n\n", report.cli_version));
        output.push_str(&format!("**Timestamp**: {}\n\n", report.timestamp));

        if !report.checks.is_empty() {
            output.push_str("## Checks\n\n");
            let headers = vec![
                "Name".to_string(),
                "Status".to_string(),
                "Message".to_string(),
            ];
            let rows: Vec<Vec<String>> = report
                .checks
                .iter()
                .map(|c| {
                    let status = match c.status {
                        crate::commands::doctor::CheckStatus::Pass => "✅ Pass",
                        crate::commands::doctor::CheckStatus::Warning => "⚠️ Warning",
                        crate::commands::doctor::CheckStatus::Fail => "❌ Fail",
                        crate::commands::doctor::CheckStatus::Skipped => "⏭️ Skipped",
                    };
                    vec![c.name.clone(), status.to_string(), c.message.clone()]
                })
                .collect();
            output.push_str(&build_markdown_table(&headers, &rows));
        }

        Ok(output)
    }

    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String> {
        to_markdown_section(status, "KVStore Status")
    }

    fn format_kvstore_collections(&self, collections: &[KvStoreCollection]) -> Result<String> {
        to_markdown_table(collections, "KVStore Collections")
    }

    fn format_kvstore_records(&self, records: &[KvStoreRecord]) -> Result<String> {
        to_markdown_table(records, "KVStore Records")
    }

    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String> {
        let mut output = "# License Information\n\n".to_string();

        if !license.usage.is_empty() {
            output.push_str("## Usage\n\n");
            let headers = vec!["Name".to_string(), "Quota".to_string()];
            let rows: Vec<Vec<String>> = license
                .usage
                .iter()
                .map(|u| vec![u.name.clone(), u.quota.to_string()])
                .collect();
            output.push_str(&build_markdown_table(&headers, &rows));
        }

        if !license.pools.is_empty() {
            output.push_str("\n## Pools\n\n");
            let headers = vec!["Name".to_string(), "Quota".to_string()];
            let rows: Vec<Vec<String>> = license
                .pools
                .iter()
                .map(|p| vec![p.name.clone(), p.quota.to_string()])
                .collect();
            output.push_str(&build_markdown_table(&headers, &rows));
        }

        Ok(output)
    }

    fn format_logs(&self, logs: &[LogEntry]) -> Result<String> {
        to_markdown_table(logs, "Logs")
    }

    fn format_logs_streaming(&self, logs: &[LogEntry], _is_first: bool) -> Result<String> {
        let mut output = String::new();
        for log in logs {
            let level_str = format!("{:?}", log.level);
            output.push_str(&format!(
                "- **{}** [{}] {}: {}\n",
                log.time, level_str, log.component, log.message
            ));
        }
        Ok(output)
    }

    fn format_users(&self, users: &[User]) -> Result<String> {
        to_markdown_table(users, "Users")
    }

    fn format_apps(&self, apps: &[App]) -> Result<String> {
        to_markdown_table(apps, "Apps")
    }

    fn format_app_info(&self, app: &App) -> Result<String> {
        to_markdown_section(app, &format!("App: {}", app.name))
    }

    fn format_saved_searches(&self, searches: &[SavedSearch]) -> Result<String> {
        to_markdown_table(searches, "Saved Searches")
    }

    fn format_saved_search_info(&self, search: &SavedSearch) -> Result<String> {
        to_markdown_section(search, &format!("Saved Search: {}", search.name))
    }

    fn format_job_details(&self, job: &SearchJobStatus) -> Result<String> {
        to_markdown_section(job, &format!("Job Details: {}", job.sid))
    }

    fn format_profile(&self, profile_name: &str, profile: &ProfileConfig) -> Result<String> {
        #[derive(Serialize)]
        struct ProfileDisplay {
            name: String,
            base_url: Option<String>,
            username: Option<String>,
            skip_verify: Option<bool>,
            timeout_seconds: Option<u64>,
            max_retries: Option<usize>,
        }

        let display = ProfileDisplay {
            name: profile_name.to_string(),
            base_url: profile.base_url.clone(),
            username: profile.username.clone(),
            skip_verify: profile.skip_verify,
            timeout_seconds: profile.timeout_seconds,
            max_retries: profile.max_retries,
        };

        to_markdown_section(&display, &format!("Profile: {}", profile_name))
    }

    fn format_profiles(&self, profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
        let mut output = "# Profiles\n\n".to_string();

        if profiles.is_empty() {
            output.push_str("_No profiles configured._\n");
            return Ok(output);
        }

        let headers = vec![
            "Name".to_string(),
            "Base URL".to_string(),
            "Username".to_string(),
        ];
        let rows: Vec<Vec<String>> = profiles
            .iter()
            .map(|(name, p)| {
                vec![
                    name.clone(),
                    p.base_url.clone().unwrap_or_default(),
                    p.username.clone().unwrap_or_default(),
                ]
            })
            .collect();

        output.push_str(&build_markdown_table(&headers, &rows));
        Ok(output)
    }

    fn format_forwarders(&self, forwarders: &[Forwarder], _detailed: bool) -> Result<String> {
        to_markdown_table(forwarders, "Forwarders")
    }

    fn format_search_peers(&self, peers: &[SearchPeer], _detailed: bool) -> Result<String> {
        to_markdown_table(peers, "Search Peers")
    }

    fn format_inputs(&self, inputs: &[Input], _detailed: bool) -> Result<String> {
        to_markdown_table(inputs, "Inputs")
    }

    fn format_config_files(&self, files: &[ConfigFile]) -> Result<String> {
        to_markdown_table(files, "Config Files")
    }

    fn format_config_stanzas(&self, stanzas: &[ConfigStanza]) -> Result<String> {
        to_markdown_table(stanzas, "Config Stanzas")
    }

    fn format_config_stanza(&self, stanza: &ConfigStanza) -> Result<String> {
        to_markdown_section(stanza, &format!("Config Stanza: {}", stanza.name))
    }

    fn format_fired_alerts(&self, alerts: &[splunk_client::models::FiredAlert]) -> Result<String> {
        to_markdown_table(alerts, "Fired Alerts")
    }

    fn format_fired_alert_info(&self, alert: &splunk_client::models::FiredAlert) -> Result<String> {
        to_markdown_section(alert, &format!("Fired Alert: {}", alert.name))
    }

    fn format_lookups(&self, lookups: &[splunk_client::LookupTable]) -> Result<String> {
        to_markdown_table(lookups, "Lookup Tables")
    }

    fn format_roles(&self, roles: &[splunk_client::Role]) -> Result<String> {
        to_markdown_table(roles, "Roles")
    }

    fn format_capabilities(&self, capabilities: &[splunk_client::Capability]) -> Result<String> {
        let mut output = "# Capabilities\n\n".to_string();

        if capabilities.is_empty() {
            output.push_str("_No capabilities found._\n");
            return Ok(output);
        }

        for cap in capabilities {
            output.push_str(&format!("- {}\n", cap.name));
        }

        Ok(output)
    }

    fn format_installed_licenses(
        &self,
        licenses: &[splunk_client::InstalledLicense],
    ) -> Result<String> {
        to_markdown_table(licenses, "Installed Licenses")
    }

    fn format_license_install(&self, result: &LicenseInstallOutput) -> Result<String> {
        to_markdown_section(result, "License Installation")
    }

    fn format_license_pools(&self, pools: &[splunk_client::LicensePool]) -> Result<String> {
        to_markdown_table(pools, "License Pools")
    }

    fn format_license_pool_operation(&self, result: &LicensePoolOperationOutput) -> Result<String> {
        to_markdown_section(result, "License Pool Operation")
    }

    fn format_hec_response(&self, response: &splunk_client::HecResponse) -> Result<String> {
        to_markdown_section(response, "HEC Response")
    }

    fn format_hec_batch_response(
        &self,
        response: &splunk_client::HecBatchResponse,
    ) -> Result<String> {
        to_markdown_section(response, "HEC Batch Response")
    }

    fn format_hec_health(&self, health: &splunk_client::HecHealth) -> Result<String> {
        to_markdown_section(health, "HEC Health")
    }

    fn format_hec_ack_status(&self, status: &splunk_client::HecAckStatus) -> Result<String> {
        to_markdown_section(status, "HEC Acknowledgment Status")
    }

    fn format_macros(&self, macros: &[splunk_client::Macro]) -> Result<String> {
        to_markdown_table(macros, "Macros")
    }

    fn format_macro_info(&self, macro_info: &splunk_client::Macro) -> Result<String> {
        to_markdown_section(macro_info, &format!("Macro: {}", macro_info.name))
    }

    fn format_audit_events(&self, events: &[AuditEvent], _detailed: bool) -> Result<String> {
        to_markdown_table(events, "Audit Events")
    }

    fn format_dashboards(&self, dashboards: &[Dashboard], _detailed: bool) -> Result<String> {
        to_markdown_table(dashboards, "Dashboards")
    }

    fn format_dashboard(&self, dashboard: &Dashboard) -> Result<String> {
        to_markdown_section(dashboard, &format!("Dashboard: {}", dashboard.name))
    }

    fn format_datamodels(&self, datamodels: &[DataModel], _detailed: bool) -> Result<String> {
        to_markdown_table(datamodels, "Data Models")
    }

    fn format_datamodel(&self, datamodel: &DataModel) -> Result<String> {
        to_markdown_section(datamodel, &format!("Data Model: {}", datamodel.name))
    }

    fn format_workload_pools(
        &self,
        pools: &[splunk_client::WorkloadPool],
        _detailed: bool,
    ) -> Result<String> {
        to_markdown_table(pools, "Workload Pools")
    }

    fn format_workload_rules(
        &self,
        rules: &[splunk_client::WorkloadRule],
        _detailed: bool,
    ) -> Result<String> {
        to_markdown_table(rules, "Workload Rules")
    }

    fn format_shc_status(&self, status: &ShcStatusOutput) -> Result<String> {
        to_markdown_section(status, "SHC Status")
    }

    fn format_shc_members(
        &self,
        members: &[ShcMemberOutput],
        _pagination: &Pagination,
    ) -> Result<String> {
        to_markdown_table(members, "SHC Members")
    }

    fn format_shc_captain(&self, captain: &ShcCaptainOutput) -> Result<String> {
        to_markdown_section(captain, "SHC Captain")
    }

    fn format_shc_config(&self, config: &ShcConfigOutput) -> Result<String> {
        to_markdown_section(config, "SHC Config")
    }

    fn format_shc_management(&self, output: &ShcManagementOutput) -> Result<String> {
        to_markdown_section(output, "SHC Management Operation")
    }

    fn format_validation_result(
        &self,
        result: &splunk_client::models::ValidateSplResponse,
    ) -> Result<String> {
        to_markdown_section(result, "SPL Validation Result")
    }
}
