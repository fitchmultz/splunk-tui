//! XML formatter implementation.
//!
//! Responsibilities:
//! - Format resources as XML with proper escaping.
//! - Handle nested structures via recursive element generation.
//!
//! Does NOT handle:
//! - Other output formats.
//! - Schema validation.
//!
//! Invariants:
//! - XML output includes standard version/encoding declaration
//! - Special characters are properly escaped in XML content

use crate::formatters::{
    ClusterInfoOutput, ClusterManagementOutput, ClusterPeerOutput, Formatter, LicenseInfoOutput,
    LicenseInstallOutput, LicensePoolOperationOutput, Pagination, ShcCaptainOutput,
    ShcConfigOutput, ShcManagementOutput, ShcMemberOutput, ShcStatusOutput, common::escape_xml,
};
use anyhow::Result;
use splunk_client::models::DataModel;
use splunk_client::models::{
    AuditEvent, ConfigFile, ConfigStanza, Input, KvStoreCollection, KvStoreRecord, SearchPeer,
};
use splunk_client::{
    App, Dashboard, Forwarder, HealthCheckOutput, Index, KvStoreStatus, SavedSearch,
    SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

mod alerts;
mod cluster;
mod configs;
mod forwarders;
mod health;
mod hec;
mod indexes;
mod inputs;
mod jobs;
mod kvstore;
mod license;
mod logs;
mod profiles;
mod saved_searches;
mod search;
mod search_peers;
mod users;
mod workload;

/// XML formatter.
pub struct XmlFormatter;

impl Formatter for XmlFormatter {
    // Delegated implementations using macros
    crate::impl_delegated_formatter_slice! {
        format_jobs: &[SearchJobStatus] => jobs::format_jobs,
        format_users: &[User] => users::format_users,
        format_saved_searches: &[SavedSearch] => saved_searches::format_saved_searches,
        format_logs: &[splunk_client::models::LogEntry] => logs::format_logs,
        format_kvstore_collections: &[KvStoreCollection] => kvstore::format_kvstore_collections,
        format_kvstore_records: &[KvStoreRecord] => kvstore::format_kvstore_records,
        format_config_files: &[ConfigFile] => configs::format_config_files,
        format_config_stanzas: &[ConfigStanza] => configs::format_config_stanzas,
        format_fired_alerts: &[splunk_client::models::FiredAlert] => alerts::format_fired_alerts,
        format_installed_licenses: &[splunk_client::InstalledLicense] => license::format_installed_licenses,
        format_license_pools: &[splunk_client::LicensePool] => license::format_license_pools,
    }

    crate::impl_delegated_formatter_slice_detailed! {
        format_indexes: &[Index] => indexes::format_indexes,
        format_forwarders: &[Forwarder] => forwarders::format_forwarders,
        format_search_peers: &[SearchPeer] => search_peers::format_search_peers,
        format_inputs: &[Input] => inputs::format_inputs,
        format_workload_pools: &[splunk_client::WorkloadPool] => workload::format_workload_pools,
        format_workload_rules: &[splunk_client::WorkloadRule] => workload::format_workload_rules,
    }

    crate::impl_delegated_formatter_single! {
        format_job_details: &SearchJobStatus => jobs::format_job_details,
        format_health: &HealthCheckOutput => health::format_health,
        format_health_check_report: &crate::formatters::DiagnosticReport => health::format_health_check_report,
        format_kvstore_status: &KvStoreStatus => health::format_kvstore_status,
        format_license: &LicenseInfoOutput => license::format_license,
        format_license_install: &LicenseInstallOutput => license::format_license_install,
        format_license_pool_operation: &LicensePoolOperationOutput => license::format_license_pool_operation,
        format_config_stanza: &ConfigStanza => configs::format_config_stanza,
        format_fired_alert_info: &splunk_client::models::FiredAlert => alerts::format_fired_alert_info,
        format_hec_response: &splunk_client::HecResponse => hec::format_hec_response,
        format_hec_batch_response: &splunk_client::HecBatchResponse => hec::format_hec_batch_response,
        format_hec_health: &splunk_client::HecHealth => hec::format_hec_health,
        format_hec_ack_status: &splunk_client::HecAckStatus => hec::format_hec_ack_status,
    }

    crate::impl_delegated_formatter_streaming! {
        format_logs_streaming: &[splunk_client::models::LogEntry] => logs::format_logs_streaming,
    }

    crate::impl_xml_list_formatter! {
        format_apps: &[App] => apps,
    }

    crate::impl_xml_detail_formatter! {
        format_app_info: &App => app,
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
        _peers: &[ClusterPeerOutput],
        _pagination: &Pagination,
    ) -> Result<String> {
        anyhow::bail!("Failed to format cluster peers: XML format not supported. Use JSON format.")
    }

    fn format_cluster_management(&self, output: &ClusterManagementOutput) -> Result<String> {
        let mut result = String::new();
        result.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        result.push_str("<cluster_management>\n");
        result.push_str(&format!(
            "  <operation>{}</operation>\n",
            escape_xml(&output.operation)
        ));
        result.push_str(&format!(
            "  <target>{}</target>\n",
            escape_xml(&output.target)
        ));
        result.push_str(&format!("  <success>{}</success>\n", output.success));
        result.push_str(&format!(
            "  <message>{}</message>\n",
            escape_xml(&output.message)
        ));
        result.push_str("</cluster_management>\n");
        Ok(result)
    }

    fn format_profile(&self, profile_name: &str, profile: &ProfileConfig) -> Result<String> {
        profiles::format_profile(profile_name, profile)
    }

    fn format_profiles(&self, profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
        profiles::format_profiles(profiles)
    }

    fn format_saved_search_info(&self, search: &SavedSearch) -> Result<String> {
        saved_searches::format_saved_search_info(search)
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

    fn format_roles(&self, roles: &[splunk_client::Role]) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<roles>\n");
        for role in roles {
            output.push_str("  <role>\n");
            output.push_str(&format!("    <name>{}</name>\n", escape_xml(&role.name)));
            output.push_str("    <capabilities>\n");
            for cap in &role.capabilities {
                output.push_str(&format!(
                    "      <capability>{}</capability>\n",
                    escape_xml(cap)
                ));
            }
            output.push_str("    </capabilities>\n");
            output.push_str("    <searchIndexes>\n");
            for idx in &role.search_indexes {
                output.push_str(&format!("      <index>{}</index>\n", escape_xml(idx)));
            }
            output.push_str("    </searchIndexes>\n");
            if let Some(ref filter) = role.search_filter {
                output.push_str(&format!(
                    "    <searchFilter>{}</searchFilter>\n",
                    escape_xml(filter)
                ));
            }
            output.push_str("    <importedRoles>\n");
            for imported in &role.imported_roles {
                output.push_str(&format!("      <role>{}</role>\n", escape_xml(imported)));
            }
            output.push_str("    </importedRoles>\n");
            if let Some(ref app) = role.default_app {
                output.push_str(&format!(
                    "    <defaultApp>{}</defaultApp>\n",
                    escape_xml(app)
                ));
            }
            output.push_str("  </role>\n");
        }
        output.push_str("</roles>\n");
        Ok(output)
    }

    fn format_capabilities(&self, capabilities: &[splunk_client::Capability]) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<capabilities>\n");
        for cap in capabilities {
            output.push_str(&format!(
                "  <capability>{}</capability>\n",
                escape_xml(&cap.name)
            ));
        }
        output.push_str("</capabilities>\n");
        Ok(output)
    }

    fn format_macros(&self, macros: &[splunk_client::Macro]) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<macros>\n");
        for macro_item in macros {
            output.push_str("  <macro>\n");
            output.push_str(&format!(
                "    <name>{}</name>\n",
                escape_xml(&macro_item.name)
            ));
            output.push_str(&format!(
                "    <definition>{}</definition>\n",
                escape_xml(&macro_item.definition)
            ));
            if let Some(ref args) = macro_item.args {
                output.push_str(&format!("    <args>{}</args>\n", escape_xml(args)));
            }
            if let Some(ref desc) = macro_item.description {
                output.push_str(&format!(
                    "    <description>{}</description>\n",
                    escape_xml(desc)
                ));
            }
            output.push_str(&format!(
                "    <disabled>{}</disabled>\n",
                macro_item.disabled
            ));
            output.push_str(&format!("    <iseval>{}</iseval>\n", macro_item.iseval));
            if let Some(ref validation) = macro_item.validation {
                output.push_str(&format!(
                    "    <validation>{}</validation>\n",
                    escape_xml(validation)
                ));
            }
            if let Some(ref errormsg) = macro_item.errormsg {
                output.push_str(&format!(
                    "    <errormsg>{}</errormsg>\n",
                    escape_xml(errormsg)
                ));
            }
            output.push_str("  </macro>\n");
        }
        output.push_str("</macros>\n");
        Ok(output)
    }

    fn format_macro_info(&self, macro_info: &splunk_client::Macro) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<macro>\n");
        output.push_str(&format!(
            "  <name>{}</name>\n",
            escape_xml(&macro_info.name)
        ));
        output.push_str(&format!(
            "  <definition>{}</definition>\n",
            escape_xml(&macro_info.definition)
        ));
        if let Some(ref args) = macro_info.args {
            output.push_str(&format!("  <args>{}</args>\n", escape_xml(args)));
        }
        if let Some(ref desc) = macro_info.description {
            output.push_str(&format!(
                "  <description>{}</description>\n",
                escape_xml(desc)
            ));
        }
        output.push_str(&format!("  <disabled>{}</disabled>\n", macro_info.disabled));
        output.push_str(&format!("  <iseval>{}</iseval>\n", macro_info.iseval));
        if let Some(ref validation) = macro_info.validation {
            output.push_str(&format!(
                "  <validation>{}</validation>\n",
                escape_xml(validation)
            ));
        }
        if let Some(ref errormsg) = macro_info.errormsg {
            output.push_str(&format!(
                "  <errormsg>{}</errormsg>\n",
                escape_xml(errormsg)
            ));
        }
        output.push_str("</macro>\n");
        Ok(output)
    }

    fn format_audit_events(&self, events: &[AuditEvent], _detailed: bool) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<audit_events>\n");
        for event in events {
            output.push_str("  <audit_event>\n");
            output.push_str(&format!("    <time>{}</time>\n", escape_xml(&event.time)));
            output.push_str(&format!("    <user>{}</user>\n", escape_xml(&event.user)));
            output.push_str(&format!(
                "    <action>{}</action>\n",
                escape_xml(&event.action.to_string())
            ));
            output.push_str(&format!(
                "    <target>{}</target>\n",
                escape_xml(&event.target)
            ));
            output.push_str(&format!(
                "    <result>{}</result>\n",
                escape_xml(&event.result.to_string())
            ));
            output.push_str(&format!(
                "    <client_ip>{}</client_ip>\n",
                escape_xml(&event.client_ip)
            ));
            output.push_str(&format!(
                "    <details>{}</details>\n",
                escape_xml(&event.details)
            ));
            output.push_str("  </audit_event>\n");
        }
        output.push_str("</audit_events>\n");
        Ok(output)
    }

    fn format_dashboards(&self, dashboards: &[Dashboard], _detailed: bool) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<dashboards>\n");
        for dashboard in dashboards {
            output.push_str("  <dashboard>\n");
            output.push_str(&format!(
                "    <name>{}</name>\n",
                escape_xml(&dashboard.name)
            ));
            output.push_str(&format!(
                "    <label>{}</label>\n",
                escape_xml(&dashboard.label)
            ));
            output.push_str(&format!(
                "    <author>{}</author>\n",
                escape_xml(&dashboard.author)
            ));
            output.push_str(&format!(
                "    <isDashboard>{}</isDashboard>\n",
                dashboard.is_dashboard
            ));
            output.push_str(&format!(
                "    <isVisible>{}</isVisible>\n",
                dashboard.is_visible
            ));
            if let Some(ref desc) = dashboard.description {
                output.push_str(&format!(
                    "    <description>{}</description>\n",
                    escape_xml(desc)
                ));
            }
            if let Some(ref version) = dashboard.version {
                output.push_str(&format!("    <version>{}</version>\n", escape_xml(version)));
            }
            if let Some(ref updated) = dashboard.updated {
                output.push_str(&format!("    <updated>{}</updated>\n", escape_xml(updated)));
            }
            output.push_str("  </dashboard>\n");
        }
        output.push_str("</dashboards>\n");
        Ok(output)
    }

    fn format_dashboard(&self, dashboard: &Dashboard) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<dashboard>\n");
        output.push_str(&format!("  <name>{}</name>\n", escape_xml(&dashboard.name)));
        output.push_str(&format!(
            "  <label>{}</label>\n",
            escape_xml(&dashboard.label)
        ));
        output.push_str(&format!(
            "  <author>{}</author>\n",
            escape_xml(&dashboard.author)
        ));
        output.push_str(&format!(
            "  <isDashboard>{}</isDashboard>\n",
            dashboard.is_dashboard
        ));
        output.push_str(&format!(
            "  <isVisible>{}</isVisible>\n",
            dashboard.is_visible
        ));
        if let Some(ref desc) = dashboard.description {
            output.push_str(&format!(
                "  <description>{}</description>\n",
                escape_xml(desc)
            ));
        }
        if let Some(ref version) = dashboard.version {
            output.push_str(&format!("  <version>{}</version>\n", escape_xml(version)));
        }
        if let Some(ref updated) = dashboard.updated {
            output.push_str(&format!("  <updated>{}</updated>\n", escape_xml(updated)));
        }
        if let Some(ref xml_data) = dashboard.xml_data {
            output.push_str(&format!("  <xmlData>{}</xmlData>\n", escape_xml(xml_data)));
        }
        output.push_str("</dashboard>\n");
        Ok(output)
    }

    fn format_datamodels(&self, datamodels: &[DataModel], _detailed: bool) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<datamodels>\n");
        for datamodel in datamodels {
            output.push_str("  <datamodel>\n");
            output.push_str(&format!(
                "    <name>{}</name>\n",
                escape_xml(&datamodel.name)
            ));
            output.push_str(&format!(
                "    <displayName>{}</displayName>\n",
                escape_xml(&datamodel.displayName)
            ));
            output.push_str(&format!(
                "    <owner>{}</owner>\n",
                escape_xml(&datamodel.owner)
            ));
            output.push_str(&format!("    <app>{}</app>\n", escape_xml(&datamodel.app)));
            output.push_str(&format!(
                "    <accelerated>{}</accelerated>\n",
                datamodel.is_accelerated
            ));
            if let Some(ref desc) = datamodel.description {
                output.push_str(&format!(
                    "    <description>{}</description>\n",
                    escape_xml(desc)
                ));
            }
            if let Some(ref updated) = datamodel.updated {
                output.push_str(&format!("    <updated>{}</updated>\n", escape_xml(updated)));
            }
            output.push_str("  </datamodel>\n");
        }
        output.push_str("</datamodels>\n");
        Ok(output)
    }

    fn format_datamodel(&self, datamodel: &DataModel) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<datamodel>\n");
        output.push_str(&format!("  <name>{}</name>\n", escape_xml(&datamodel.name)));
        output.push_str(&format!(
            "  <displayName>{}</displayName>\n",
            escape_xml(&datamodel.displayName)
        ));
        output.push_str(&format!(
            "  <owner>{}</owner>\n",
            escape_xml(&datamodel.owner)
        ));
        output.push_str(&format!("  <app>{}</app>\n", escape_xml(&datamodel.app)));
        output.push_str(&format!(
            "  <accelerated>{}</accelerated>\n",
            datamodel.is_accelerated
        ));
        if let Some(ref desc) = datamodel.description {
            output.push_str(&format!(
                "  <description>{}</description>\n",
                escape_xml(desc)
            ));
        }
        if let Some(ref updated) = datamodel.updated {
            output.push_str(&format!("  <updated>{}</updated>\n", escape_xml(updated)));
        }
        if let Some(ref json_data) = datamodel.json_data {
            output.push_str(&format!(
                "  <jsonData>{}</jsonData>\n",
                escape_xml(json_data)
            ));
        }
        output.push_str("</datamodel>\n");
        Ok(output)
    }

    fn format_shc_status(&self, status: &ShcStatusOutput) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<shc_status>\n");
        output.push_str(&format!(
            "  <is_captain>{}</is_captain>\n",
            status.is_captain
        ));
        output.push_str(&format!(
            "  <is_searchable>{}</is_searchable>\n",
            status.is_searchable
        ));
        output.push_str(&format!(
            "  <captain_uri>{}</captain_uri>\n",
            escape_xml(status.captain_uri.as_deref().unwrap_or("N/A"))
        ));
        output.push_str(&format!(
            "  <member_count>{}</member_count>\n",
            status.member_count
        ));
        output.push_str(&format!(
            "  <minimum_member_count>{}</minimum_member_count>\n",
            status.minimum_member_count.unwrap_or(0)
        ));
        output.push_str(&format!(
            "  <rolling_restart_flag>{}</rolling_restart_flag>\n",
            status.rolling_restart_flag.unwrap_or(false)
        ));
        output.push_str(&format!(
            "  <service_ready_flag>{}</service_ready_flag>\n",
            status.service_ready_flag.unwrap_or(false)
        ));
        output.push_str("</shc_status>\n");
        Ok(output)
    }

    fn format_shc_members(
        &self,
        _members: &[ShcMemberOutput],
        _pagination: &Pagination,
    ) -> Result<String> {
        anyhow::bail!("Failed to format SHC members: XML format not supported. Use JSON format.")
    }

    fn format_shc_captain(&self, captain: &ShcCaptainOutput) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<shc_captain>\n");
        output.push_str(&format!("  <id>{}</id>\n", escape_xml(&captain.id)));
        output.push_str(&format!("  <host>{}</host>\n", escape_xml(&captain.host)));
        output.push_str(&format!("  <port>{}</port>\n", captain.port));
        output.push_str(&format!("  <guid>{}</guid>\n", escape_xml(&captain.guid)));
        output.push_str(&format!(
            "  <is_dynamic_captain>{}</is_dynamic_captain>\n",
            captain.is_dynamic_captain
        ));
        output.push_str(&format!(
            "  <site>{}</site>\n",
            escape_xml(captain.site.as_deref().unwrap_or("N/A"))
        ));
        output.push_str("</shc_captain>\n");
        Ok(output)
    }

    fn format_shc_config(&self, config: &ShcConfigOutput) -> Result<String> {
        let mut output = String::new();
        output.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        output.push_str("<shc_config>\n");
        output.push_str(&format!("  <id>{}</id>\n", escape_xml(&config.id)));
        output.push_str(&format!(
            "  <replication_factor>{}</replication_factor>\n",
            config.replication_factor.unwrap_or(0)
        ));
        output.push_str(&format!(
            "  <captain_uri>{}</captain_uri>\n",
            escape_xml(config.captain_uri.as_deref().unwrap_or("N/A"))
        ));
        output.push_str(&format!(
            "  <shcluster_label>{}</shcluster_label>\n",
            escape_xml(config.shcluster_label.as_deref().unwrap_or("N/A"))
        ));
        output.push_str("</shc_config>\n");
        Ok(output)
    }

    fn format_shc_management(&self, output: &ShcManagementOutput) -> Result<String> {
        let mut result = String::new();
        result.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        result.push_str("<shc_management>\n");
        result.push_str(&format!(
            "  <operation>{}</operation>\n",
            escape_xml(&output.operation)
        ));
        result.push_str(&format!(
            "  <target>{}</target>\n",
            escape_xml(&output.target)
        ));
        result.push_str(&format!("  <success>{}</success>\n", output.success));
        result.push_str(&format!(
            "  <message>{}</message>\n",
            escape_xml(&output.message)
        ));
        result.push_str("</shc_management>\n");
        Ok(result)
    }

    fn format_validation_result(
        &self,
        result: &splunk_client::models::ValidateSplResponse,
    ) -> Result<String> {
        use crate::formatters::common::escape_xml;

        let mut output = String::new();
        output.push_str(
            r#"<?xml version="1.0" encoding="UTF-8"?>
<validation>
"#,
        );
        output.push_str(&format!("  <valid>{}</valid>\n", result.valid));

        if !result.errors.is_empty() {
            output.push_str("  <errors>\n");
            for error in &result.errors {
                output.push_str("    <error>\n");
                output.push_str(&format!(
                    "      <message>{}</message>\n",
                    escape_xml(&error.message)
                ));
                if let Some(line) = error.line {
                    output.push_str(&format!("      <line>{}</line>\n", line));
                }
                if let Some(col) = error.column {
                    output.push_str(&format!("      <column>{}</column>\n", col));
                }
                output.push_str("    </error>\n");
            }
            output.push_str("  </errors>\n");
        }

        if !result.warnings.is_empty() {
            output.push_str("  <warnings>\n");
            for warning in &result.warnings {
                output.push_str("    <warning>\n");
                output.push_str(&format!(
                    "      <message>{}</message>\n",
                    escape_xml(&warning.message)
                ));
                if let Some(line) = warning.line {
                    output.push_str(&format!("      <line>{}</line>\n", line));
                }
                if let Some(col) = warning.column {
                    output.push_str(&format!("      <column>{}</column>\n", col));
                }
                output.push_str("    </warning>\n");
            }
            output.push_str("  </warnings>\n");
        }

        output.push_str("</validation>\n");
        Ok(output)
    }
}
