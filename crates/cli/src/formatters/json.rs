//! JSON formatter implementation.
//!
//! Responsibilities:
//! - Format all resource types as pretty-printed JSON.
//!
//! Does NOT handle:
//! - Other output formats.
//! - Pagination (JSON output doesn't paginate).

use crate::commands::list_all::ListAllOutput;
use crate::formatters::{ClusterInfoOutput, Formatter, LicenseInfoOutput};
use anyhow::Result;
use serde::Serialize;
use splunk_client::models::{ConfigFile, ConfigStanza, Input, LogEntry, SearchPeer};
use splunk_client::{
    App, Forwarder, HealthCheckOutput, Index, KvStoreStatus, SavedSearch, SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// JSON formatter.
pub struct JsonFormatter;

impl Formatter for JsonFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        Ok(serde_json::to_string_pretty(results)?)
    }

    fn format_indexes(&self, indexes: &[Index], _detailed: bool) -> Result<String> {
        // JSON formatter always outputs full Index struct regardless of detailed flag
        Ok(serde_json::to_string_pretty(indexes)?)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        Ok(serde_json::to_string_pretty(jobs)?)
    }

    fn format_cluster_info(
        &self,
        cluster_info: &ClusterInfoOutput,
        _detailed: bool,
    ) -> Result<String> {
        Ok(serde_json::to_string_pretty(cluster_info)?)
    }

    fn format_health(&self, health: &HealthCheckOutput) -> Result<String> {
        Ok(serde_json::to_string_pretty(health)?)
    }

    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String> {
        Ok(serde_json::to_string_pretty(status)?)
    }

    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String> {
        Ok(serde_json::to_string_pretty(license)?)
    }

    fn format_logs(&self, logs: &[LogEntry]) -> Result<String> {
        Ok(serde_json::to_string_pretty(logs)?)
    }

    fn format_logs_streaming(&self, logs: &[LogEntry], _is_first: bool) -> Result<String> {
        // NDJSON format: one JSON object per line
        let mut output = String::new();
        for log in logs {
            let line = serde_json::to_string(log)?;
            output.push_str(&line);
            output.push('\n');
        }
        Ok(output)
    }

    fn format_users(&self, users: &[User]) -> Result<String> {
        Ok(serde_json::to_string_pretty(users)?)
    }

    fn format_apps(&self, apps: &[App]) -> Result<String> {
        let json = serde_json::to_string_pretty(apps)?;
        Ok(json)
    }

    fn format_app_info(&self, app: &App) -> Result<String> {
        Ok(serde_json::to_string_pretty(app)?)
    }

    fn format_list_all(&self, output: &ListAllOutput) -> Result<String> {
        Ok(serde_json::to_string_pretty(output)?)
    }

    fn format_saved_searches(&self, searches: &[SavedSearch]) -> Result<String> {
        Ok(serde_json::to_string_pretty(searches)?)
    }

    fn format_saved_search_info(&self, search: &SavedSearch) -> Result<String> {
        Ok(serde_json::to_string_pretty(search)?)
    }

    fn format_job_details(&self, job: &SearchJobStatus) -> Result<String> {
        Ok(serde_json::to_string_pretty(job)?)
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

        Ok(serde_json::to_string_pretty(&display)?)
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

        Ok(serde_json::to_string_pretty(&Output {
            profiles: display_profiles,
        })?)
    }

    fn format_forwarders(&self, forwarders: &[Forwarder], _detailed: bool) -> Result<String> {
        // JSON formatter always outputs full Forwarder struct regardless of detailed flag
        Ok(serde_json::to_string_pretty(forwarders)?)
    }

    fn format_search_peers(&self, peers: &[SearchPeer], _detailed: bool) -> Result<String> {
        // JSON formatter always outputs full SearchPeer struct regardless of detailed flag
        Ok(serde_json::to_string_pretty(peers)?)
    }

    fn format_inputs(&self, inputs: &[Input], _detailed: bool) -> Result<String> {
        // JSON formatter always outputs full Input struct regardless of detailed flag
        Ok(serde_json::to_string_pretty(inputs)?)
    }

    fn format_config_files(&self, files: &[ConfigFile]) -> Result<String> {
        Ok(serde_json::to_string_pretty(files)?)
    }

    fn format_config_stanzas(&self, stanzas: &[ConfigStanza]) -> Result<String> {
        Ok(serde_json::to_string_pretty(stanzas)?)
    }

    fn format_config_stanza(&self, stanza: &ConfigStanza) -> Result<String> {
        Ok(serde_json::to_string_pretty(stanza)?)
    }

    fn format_fired_alerts(&self, alerts: &[splunk_client::models::FiredAlert]) -> Result<String> {
        Ok(serde_json::to_string_pretty(alerts)?)
    }

    fn format_fired_alert_info(&self, alert: &splunk_client::models::FiredAlert) -> Result<String> {
        Ok(serde_json::to_string_pretty(alert)?)
    }

    fn format_lookups(&self, lookups: &[splunk_client::LookupTable]) -> Result<String> {
        Ok(serde_json::to_string_pretty(lookups)?)
    }
}
