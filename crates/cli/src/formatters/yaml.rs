//! YAML formatter implementation.
//!
//! Responsibilities:
//! - Format all resource types as YAML.
//! - Human-friendly configuration export format.
//!
//! Does NOT handle:
//! - Other output formats.
//! - Pagination (YAML output doesn't paginate).

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
    App, Forwarder, HealthCheckOutput, Index, KvStoreStatus, LicensePool, SavedSearch,
    SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// YAML formatter.
pub struct YamlFormatter;

impl Formatter for YamlFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        Ok(serde_yaml::to_string(results)?)
    }

    fn format_indexes(&self, indexes: &[Index], _detailed: bool) -> Result<String> {
        Ok(serde_yaml::to_string(indexes)?)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        Ok(serde_yaml::to_string(jobs)?)
    }

    fn format_cluster_info(
        &self,
        cluster_info: &ClusterInfoOutput,
        _detailed: bool,
    ) -> Result<String> {
        Ok(serde_yaml::to_string(cluster_info)?)
    }

    fn format_cluster_peers(
        &self,
        peers: &[ClusterPeerOutput],
        _pagination: &Pagination,
    ) -> Result<String> {
        Ok(serde_yaml::to_string(peers)?)
    }

    fn format_cluster_management(&self, output: &ClusterManagementOutput) -> Result<String> {
        Ok(serde_yaml::to_string(output)?)
    }

    fn format_health(&self, health: &HealthCheckOutput) -> Result<String> {
        Ok(serde_yaml::to_string(health)?)
    }

    fn format_health_check_report(
        &self,
        report: &crate::formatters::DiagnosticReport,
    ) -> Result<String> {
        Ok(serde_yaml::to_string(report)?)
    }

    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String> {
        Ok(serde_yaml::to_string(status)?)
    }

    fn format_kvstore_collections(&self, collections: &[KvStoreCollection]) -> Result<String> {
        Ok(serde_yaml::to_string(collections)?)
    }

    fn format_kvstore_records(&self, records: &[KvStoreRecord]) -> Result<String> {
        Ok(serde_yaml::to_string(records)?)
    }

    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String> {
        Ok(serde_yaml::to_string(license)?)
    }

    fn format_logs(&self, logs: &[LogEntry]) -> Result<String> {
        Ok(serde_yaml::to_string(logs)?)
    }

    fn format_logs_streaming(&self, logs: &[LogEntry], _is_first: bool) -> Result<String> {
        let mut output = String::new();
        for log in logs {
            output.push_str("---\n");
            output.push_str(&serde_yaml::to_string(log)?);
        }
        Ok(output)
    }

    fn format_users(&self, users: &[User]) -> Result<String> {
        Ok(serde_yaml::to_string(users)?)
    }

    fn format_apps(&self, apps: &[App]) -> Result<String> {
        Ok(serde_yaml::to_string(apps)?)
    }

    fn format_app_info(&self, app: &App) -> Result<String> {
        Ok(serde_yaml::to_string(app)?)
    }

    fn format_saved_searches(&self, searches: &[SavedSearch]) -> Result<String> {
        Ok(serde_yaml::to_string(searches)?)
    }

    fn format_saved_search_info(&self, search: &SavedSearch) -> Result<String> {
        Ok(serde_yaml::to_string(search)?)
    }

    fn format_job_details(&self, job: &SearchJobStatus) -> Result<String> {
        Ok(serde_yaml::to_string(job)?)
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

        Ok(serde_yaml::to_string(&display)?)
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

        #[derive(Serialize)]
        struct Output {
            profiles: BTreeMap<String, ProfileDisplay>,
        }

        Ok(serde_yaml::to_string(&Output {
            profiles: display_profiles,
        })?)
    }

    fn format_forwarders(&self, forwarders: &[Forwarder], _detailed: bool) -> Result<String> {
        Ok(serde_yaml::to_string(forwarders)?)
    }

    fn format_search_peers(&self, peers: &[SearchPeer], _detailed: bool) -> Result<String> {
        Ok(serde_yaml::to_string(peers)?)
    }

    fn format_inputs(&self, inputs: &[Input], _detailed: bool) -> Result<String> {
        Ok(serde_yaml::to_string(inputs)?)
    }

    fn format_config_files(&self, files: &[ConfigFile]) -> Result<String> {
        Ok(serde_yaml::to_string(files)?)
    }

    fn format_config_stanzas(&self, stanzas: &[ConfigStanza]) -> Result<String> {
        Ok(serde_yaml::to_string(stanzas)?)
    }

    fn format_config_stanza(&self, stanza: &ConfigStanza) -> Result<String> {
        Ok(serde_yaml::to_string(stanza)?)
    }

    fn format_fired_alerts(&self, alerts: &[splunk_client::models::FiredAlert]) -> Result<String> {
        Ok(serde_yaml::to_string(alerts)?)
    }

    fn format_fired_alert_info(&self, alert: &splunk_client::models::FiredAlert) -> Result<String> {
        Ok(serde_yaml::to_string(alert)?)
    }

    fn format_lookups(&self, lookups: &[splunk_client::LookupTable]) -> Result<String> {
        Ok(serde_yaml::to_string(lookups)?)
    }

    fn format_roles(&self, roles: &[splunk_client::Role]) -> Result<String> {
        Ok(serde_yaml::to_string(roles)?)
    }

    fn format_capabilities(&self, capabilities: &[splunk_client::Capability]) -> Result<String> {
        Ok(serde_yaml::to_string(capabilities)?)
    }

    fn format_installed_licenses(
        &self,
        licenses: &[splunk_client::InstalledLicense],
    ) -> Result<String> {
        Ok(serde_yaml::to_string(licenses)?)
    }

    fn format_license_install(&self, result: &LicenseInstallOutput) -> Result<String> {
        Ok(serde_yaml::to_string(result)?)
    }

    fn format_license_pools(&self, pools: &[LicensePool]) -> Result<String> {
        Ok(serde_yaml::to_string(pools)?)
    }

    fn format_license_pool_operation(&self, result: &LicensePoolOperationOutput) -> Result<String> {
        Ok(serde_yaml::to_string(result)?)
    }

    fn format_hec_response(&self, response: &splunk_client::HecResponse) -> Result<String> {
        Ok(serde_yaml::to_string(response)?)
    }

    fn format_hec_batch_response(
        &self,
        response: &splunk_client::HecBatchResponse,
    ) -> Result<String> {
        Ok(serde_yaml::to_string(response)?)
    }

    fn format_hec_health(&self, health: &splunk_client::HecHealth) -> Result<String> {
        Ok(serde_yaml::to_string(health)?)
    }

    fn format_hec_ack_status(&self, status: &splunk_client::HecAckStatus) -> Result<String> {
        Ok(serde_yaml::to_string(status)?)
    }

    fn format_macros(&self, macros: &[splunk_client::Macro]) -> Result<String> {
        Ok(serde_yaml::to_string(macros)?)
    }

    fn format_macro_info(&self, macro_info: &splunk_client::Macro) -> Result<String> {
        Ok(serde_yaml::to_string(macro_info)?)
    }

    fn format_audit_events(&self, events: &[AuditEvent], _detailed: bool) -> Result<String> {
        Ok(serde_yaml::to_string(events)?)
    }

    fn format_dashboards(&self, dashboards: &[Dashboard], _detailed: bool) -> Result<String> {
        Ok(serde_yaml::to_string(dashboards)?)
    }

    fn format_dashboard(&self, dashboard: &Dashboard) -> Result<String> {
        Ok(serde_yaml::to_string(dashboard)?)
    }

    fn format_datamodels(&self, datamodels: &[DataModel], _detailed: bool) -> Result<String> {
        Ok(serde_yaml::to_string(datamodels)?)
    }

    fn format_datamodel(&self, datamodel: &DataModel) -> Result<String> {
        Ok(serde_yaml::to_string(datamodel)?)
    }

    fn format_workload_pools(
        &self,
        pools: &[splunk_client::WorkloadPool],
        _detailed: bool,
    ) -> Result<String> {
        Ok(serde_yaml::to_string(pools)?)
    }

    fn format_workload_rules(
        &self,
        rules: &[splunk_client::WorkloadRule],
        _detailed: bool,
    ) -> Result<String> {
        Ok(serde_yaml::to_string(rules)?)
    }

    fn format_shc_status(&self, status: &ShcStatusOutput) -> Result<String> {
        Ok(serde_yaml::to_string(status)?)
    }

    fn format_shc_members(
        &self,
        members: &[ShcMemberOutput],
        _pagination: &Pagination,
    ) -> Result<String> {
        Ok(serde_yaml::to_string(members)?)
    }

    fn format_shc_captain(&self, captain: &ShcCaptainOutput) -> Result<String> {
        Ok(serde_yaml::to_string(captain)?)
    }

    fn format_shc_config(&self, config: &ShcConfigOutput) -> Result<String> {
        Ok(serde_yaml::to_string(config)?)
    }

    fn format_shc_management(&self, output: &ShcManagementOutput) -> Result<String> {
        Ok(serde_yaml::to_string(output)?)
    }

    fn format_validation_result(
        &self,
        result: &splunk_client::models::ValidateSplResponse,
    ) -> Result<String> {
        Ok(serde_yaml::to_string(result)?)
    }
}
