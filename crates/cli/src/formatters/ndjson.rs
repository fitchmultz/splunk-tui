//! NDJSON formatter implementation.
//!
//! Responsibilities:
//! - Format resources as NDJSON (Newline Delimited JSON).
//! - Each record is output as a single line of JSON.
//!
//! Does NOT handle:
//! - Pretty-printed JSON (use JsonFormatter for that).
//! - Other output formats.
//!
//! Invariants:
//! - Each line is a valid JSON object
//! - Records are separated by newlines (no trailing comma)
//! - Suitable for streaming and log processing pipelines

use crate::formatters::{
    ClusterInfoOutput, ClusterManagementOutput, ClusterPeerOutput, Formatter, LicenseInfoOutput,
    LicenseInstallOutput, LicensePoolOperationOutput, Pagination, ShcCaptainOutput,
    ShcConfigOutput, ShcManagementOutput, ShcMemberOutput, ShcStatusOutput,
};
use anyhow::Result;
use serde::Serialize;
use splunk_client::models::{
    AuditEvent, ConfigFile, ConfigStanza, Dashboard, DataModel, FiredAlert, Input,
    KvStoreCollection, KvStoreRecord, LogEntry, SearchPeer,
};
use splunk_client::{
    App, Forwarder, HealthCheckOutput, Index, KvStoreStatus, LicensePool, SavedSearch,
    SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// NDJSON formatter.
pub struct NdjsonFormatter;

/// Helper function to format a slice as NDJSON.
fn to_ndjson<T: Serialize>(items: &[T]) -> Result<String> {
    let mut output = String::new();
    for item in items {
        let line = serde_json::to_string(item)?;
        output.push_str(&line);
        output.push('\n');
    }
    Ok(output)
}

/// Helper function to format a single item as NDJSON.
fn to_ndjson_single<T: Serialize>(item: &T) -> Result<String> {
    let line = serde_json::to_string(item)?;
    Ok(line + "\n")
}

impl Formatter for NdjsonFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        to_ndjson(results)
    }

    fn format_indexes(&self, indexes: &[Index], _detailed: bool) -> Result<String> {
        to_ndjson(indexes)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        to_ndjson(jobs)
    }

    fn format_cluster_info(
        &self,
        cluster_info: &ClusterInfoOutput,
        _detailed: bool,
    ) -> Result<String> {
        to_ndjson_single(cluster_info)
    }

    fn format_cluster_peers(
        &self,
        peers: &[ClusterPeerOutput],
        _pagination: &Pagination,
    ) -> Result<String> {
        to_ndjson(peers)
    }

    fn format_cluster_management(&self, output: &ClusterManagementOutput) -> Result<String> {
        to_ndjson_single(output)
    }

    fn format_health(&self, health: &HealthCheckOutput) -> Result<String> {
        to_ndjson_single(health)
    }

    fn format_health_check_report(
        &self,
        report: &crate::formatters::DiagnosticReport,
    ) -> Result<String> {
        to_ndjson_single(report)
    }

    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String> {
        to_ndjson_single(status)
    }

    fn format_kvstore_collections(&self, collections: &[KvStoreCollection]) -> Result<String> {
        to_ndjson(collections)
    }

    fn format_kvstore_records(&self, records: &[KvStoreRecord]) -> Result<String> {
        to_ndjson(records)
    }

    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String> {
        to_ndjson_single(license)
    }

    fn format_logs(&self, logs: &[LogEntry]) -> Result<String> {
        to_ndjson(logs)
    }

    fn format_logs_streaming(&self, logs: &[LogEntry], _is_first: bool) -> Result<String> {
        // NDJSON is the natural format for streaming logs
        to_ndjson(logs)
    }

    fn format_users(&self, users: &[User]) -> Result<String> {
        to_ndjson(users)
    }

    fn format_apps(&self, apps: &[App]) -> Result<String> {
        to_ndjson(apps)
    }

    fn format_app_info(&self, app: &App) -> Result<String> {
        to_ndjson_single(app)
    }

    fn format_saved_searches(&self, searches: &[SavedSearch]) -> Result<String> {
        to_ndjson(searches)
    }

    fn format_saved_search_info(&self, search: &SavedSearch) -> Result<String> {
        to_ndjson_single(search)
    }

    fn format_job_details(&self, job: &SearchJobStatus) -> Result<String> {
        to_ndjson_single(job)
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
            password: Option<String>,
            api_token: Option<String>,
        }

        let display = ProfileDisplay {
            name: profile_name.to_string(),
            base_url: profile.base_url.clone(),
            username: profile.username.clone(),
            skip_verify: profile.skip_verify,
            timeout_seconds: profile.timeout_seconds,
            max_retries: profile.max_retries,
            password: profile.password.as_ref().map(|_| "****".to_string()),
            api_token: profile.api_token.as_ref().map(|_| "****".to_string()),
        };

        to_ndjson_single(&display)
    }

    fn format_profiles(&self, profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
        #[derive(Serialize)]
        struct ProfileDisplay {
            base_url: Option<String>,
            username: Option<String>,
            skip_verify: Option<bool>,
            timeout_seconds: Option<u64>,
            max_retries: Option<usize>,
            password: Option<String>,
            api_token: Option<String>,
        }

        let display_profiles: BTreeMap<String, ProfileDisplay> = profiles
            .iter()
            .map(|(name, profile)| {
                (
                    name.clone(),
                    ProfileDisplay {
                        base_url: profile.base_url.clone(),
                        username: profile.username.clone(),
                        skip_verify: profile.skip_verify,
                        timeout_seconds: profile.timeout_seconds,
                        max_retries: profile.max_retries,
                        password: profile.password.as_ref().map(|_| "****".to_string()),
                        api_token: profile.api_token.as_ref().map(|_| "****".to_string()),
                    },
                )
            })
            .collect();

        to_ndjson_single(&serde_json::json!({ "profiles": display_profiles }))
    }

    fn format_forwarders(&self, forwarders: &[Forwarder], _detailed: bool) -> Result<String> {
        to_ndjson(forwarders)
    }

    fn format_search_peers(&self, peers: &[SearchPeer], _detailed: bool) -> Result<String> {
        to_ndjson(peers)
    }

    fn format_inputs(&self, inputs: &[Input], _detailed: bool) -> Result<String> {
        to_ndjson(inputs)
    }

    fn format_config_files(&self, files: &[ConfigFile]) -> Result<String> {
        to_ndjson(files)
    }

    fn format_config_stanzas(&self, stanzas: &[ConfigStanza]) -> Result<String> {
        to_ndjson(stanzas)
    }

    fn format_config_stanza(&self, stanza: &ConfigStanza) -> Result<String> {
        to_ndjson_single(stanza)
    }

    fn format_fired_alerts(&self, alerts: &[FiredAlert]) -> Result<String> {
        to_ndjson(alerts)
    }

    fn format_fired_alert_info(&self, alert: &FiredAlert) -> Result<String> {
        to_ndjson_single(alert)
    }

    fn format_lookups(&self, lookups: &[splunk_client::LookupTable]) -> Result<String> {
        to_ndjson(lookups)
    }

    fn format_roles(&self, roles: &[splunk_client::Role]) -> Result<String> {
        to_ndjson(roles)
    }

    fn format_capabilities(&self, capabilities: &[splunk_client::Capability]) -> Result<String> {
        to_ndjson(capabilities)
    }

    fn format_installed_licenses(
        &self,
        licenses: &[splunk_client::InstalledLicense],
    ) -> Result<String> {
        to_ndjson(licenses)
    }

    fn format_license_install(&self, result: &LicenseInstallOutput) -> Result<String> {
        to_ndjson_single(result)
    }

    fn format_license_pools(&self, pools: &[LicensePool]) -> Result<String> {
        to_ndjson(pools)
    }

    fn format_license_pool_operation(&self, result: &LicensePoolOperationOutput) -> Result<String> {
        to_ndjson_single(result)
    }

    fn format_hec_response(&self, response: &splunk_client::HecResponse) -> Result<String> {
        to_ndjson_single(response)
    }

    fn format_hec_batch_response(
        &self,
        response: &splunk_client::HecBatchResponse,
    ) -> Result<String> {
        to_ndjson_single(response)
    }

    fn format_hec_health(&self, health: &splunk_client::HecHealth) -> Result<String> {
        to_ndjson_single(health)
    }

    fn format_hec_ack_status(&self, status: &splunk_client::HecAckStatus) -> Result<String> {
        to_ndjson_single(status)
    }

    fn format_macros(&self, macros: &[splunk_client::Macro]) -> Result<String> {
        to_ndjson(macros)
    }

    fn format_macro_info(&self, macro_info: &splunk_client::Macro) -> Result<String> {
        to_ndjson_single(macro_info)
    }

    fn format_audit_events(&self, events: &[AuditEvent], _detailed: bool) -> Result<String> {
        to_ndjson(events)
    }

    fn format_dashboards(&self, dashboards: &[Dashboard], _detailed: bool) -> Result<String> {
        to_ndjson(dashboards)
    }

    fn format_dashboard(&self, dashboard: &Dashboard) -> Result<String> {
        to_ndjson_single(dashboard)
    }

    fn format_datamodels(&self, datamodels: &[DataModel], _detailed: bool) -> Result<String> {
        to_ndjson(datamodels)
    }

    fn format_datamodel(&self, datamodel: &DataModel) -> Result<String> {
        to_ndjson_single(datamodel)
    }

    fn format_workload_pools(
        &self,
        pools: &[splunk_client::WorkloadPool],
        _detailed: bool,
    ) -> Result<String> {
        to_ndjson(pools)
    }

    fn format_workload_rules(
        &self,
        rules: &[splunk_client::WorkloadRule],
        _detailed: bool,
    ) -> Result<String> {
        to_ndjson(rules)
    }

    fn format_shc_status(&self, status: &ShcStatusOutput) -> Result<String> {
        to_ndjson_single(status)
    }

    fn format_shc_members(
        &self,
        members: &[ShcMemberOutput],
        _pagination: &Pagination,
    ) -> Result<String> {
        to_ndjson(members)
    }

    fn format_shc_captain(&self, captain: &ShcCaptainOutput) -> Result<String> {
        to_ndjson_single(captain)
    }

    fn format_shc_config(&self, config: &ShcConfigOutput) -> Result<String> {
        to_ndjson_single(config)
    }

    fn format_shc_management(&self, output: &ShcManagementOutput) -> Result<String> {
        to_ndjson_single(output)
    }

    fn format_validation_result(
        &self,
        result: &splunk_client::models::ValidateSplResponse,
    ) -> Result<String> {
        let mut output = String::new();

        // Output errors
        for error in &result.errors {
            let line = serde_json::json!({
                "type": "error",
                "valid": false,
                "message": &error.message,
                "line": error.line,
                "column": error.column
            });
            output.push_str(&serde_json::to_string(&line)?);
            output.push('\n');
        }

        // Output warnings
        for warning in &result.warnings {
            let line = serde_json::json!({
                "type": "warning",
                "valid": true,
                "message": &warning.message,
                "line": warning.line,
                "column": warning.column
            });
            output.push_str(&serde_json::to_string(&line)?);
            output.push('\n');
        }

        // If valid with no warnings, output a single success line
        if result.errors.is_empty() && result.warnings.is_empty() {
            let line = serde_json::json!({
                "type": "success",
                "valid": true,
                "message": "SPL is valid",
                "line": null,
                "column": null
            });
            output.push_str(&serde_json::to_string(&line)?);
            output.push('\n');
        }

        Ok(output)
    }
}
