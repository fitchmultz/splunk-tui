//! XML formatter implementation.
//!
//! Responsibilities:
//! - Format resources as XML with proper escaping.
//! - Handle nested structures via recursive element generation.
//!
//! Does NOT handle:
//! - Other output formats.
//! - Schema validation.

use crate::formatters::{ClusterInfoOutput, Formatter, LicenseInfoOutput};
use anyhow::Result;
use splunk_client::models::{ConfigFile, ConfigStanza, Input, SearchPeer};
use splunk_client::{
    App, Forwarder, HealthCheckOutput, Index, KvStoreStatus, SavedSearch, SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

mod alerts;
mod apps;
mod cluster;
mod configs;
mod forwarders;
mod health;
mod indexes;
mod inputs;
mod jobs;
mod license;
mod list_all;
mod logs;
mod profiles;
mod saved_searches;
mod search;
mod search_peers;
mod users;

/// XML formatter.
pub struct XmlFormatter;

impl Formatter for XmlFormatter {
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

    fn format_health(&self, health: &HealthCheckOutput) -> Result<String> {
        health::format_health(health)
    }

    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String> {
        health::format_kvstore_status(status)
    }

    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String> {
        license::format_license(license)
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
        configs::format_config_stanza(stanza)
    }

    fn format_fired_alerts(&self, alerts: &[splunk_client::models::FiredAlert]) -> Result<String> {
        alerts::format_fired_alerts(alerts)
    }

    fn format_fired_alert_info(&self, alert: &splunk_client::models::FiredAlert) -> Result<String> {
        alerts::format_fired_alert_info(alert)
    }

    fn format_lookups(&self, lookups: &[splunk_client::LookupTable]) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<lookups>\n");
        for lookup in lookups {
            output.push_str("  <lookup>\n");
            output.push_str(&format!("    <name>{}</name>\n", escape_xml(&lookup.name)));
            output.push_str(&format!(
                "    <filename>{}</filename>\n",
                escape_xml(&lookup.filename)
            ));
            output.push_str(&format!(
                "    <owner>{}</owner>\n",
                escape_xml(&lookup.owner)
            ));
            output.push_str(&format!("    <app>{}</app>\n", escape_xml(&lookup.app)));
            output.push_str(&format!(
                "    <sharing>{}</sharing>\n",
                escape_xml(&lookup.sharing)
            ));
            output.push_str(&format!("    <size>{}</size>\n", lookup.size));
            output.push_str("  </lookup>\n");
        }
        output.push_str("</lookups>\n");
        Ok(output)
    }
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
