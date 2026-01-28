//! XML formatter implementation.
//!
//! Responsibilities:
//! - Format resources as XML with proper escaping.
//! - Handle nested structures via recursive element generation.
//!
//! Does NOT handle:
//! - Other output formats.
//! - Schema validation.

use crate::commands::list_all::ListAllOutput;
use crate::formatters::common::escape_xml;
use crate::formatters::{ClusterInfoOutput, Formatter, LicenseInfoOutput};
use anyhow::Result;
use splunk_client::models::LogEntry;
use splunk_client::{
    App, HealthCheckOutput, Index, KvStoreStatus, SavedSearch, SearchJobStatus, User,
};
use splunk_config::types::ProfileConfig;
use std::collections::BTreeMap;

/// XML formatter.
pub struct XmlFormatter;

impl Formatter for XmlFormatter {
    fn format_search_results(&self, results: &[serde_json::Value]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<results>\n");

        for result in results {
            // Use nested XML structure instead of flat fields
            let nested = value_to_xml_elements("result", result, "  ");
            xml.push_str(&nested.join("\n"));
            xml.push('\n');
        }

        xml.push_str("</results>");
        Ok(xml)
    }

    fn format_indexes(&self, indexes: &[Index], detailed: bool) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<indexes>\n");

        for index in indexes {
            xml.push_str("  <index>\n");
            xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&index.name)));
            xml.push_str(&format!(
                "    <sizeMB>{}</sizeMB>\n",
                index.current_db_size_mb
            ));
            xml.push_str(&format!(
                "    <events>{}</events>\n",
                index.total_event_count
            ));
            if let Some(max_size) = index.max_total_data_size_mb {
                xml.push_str(&format!("    <maxSizeMB>{}</maxSizeMB>\n", max_size));
            }
            // When detailed, include additional path and retention fields
            if detailed {
                if let Some(frozen_time) = index.frozen_time_period_in_secs {
                    xml.push_str(&format!(
                        "    <retentionSecs>{}</retentionSecs>\n",
                        frozen_time
                    ));
                }
                if let Some(home_path) = &index.home_path {
                    xml.push_str(&format!(
                        "    <homePath>{}</homePath>\n",
                        escape_xml(home_path)
                    ));
                }
                if let Some(cold_path) = &index.cold_db_path {
                    xml.push_str(&format!(
                        "    <coldPath>{}</coldPath>\n",
                        escape_xml(cold_path)
                    ));
                }
                if let Some(thawed_path) = &index.thawed_path {
                    xml.push_str(&format!(
                        "    <thawedPath>{}</thawedPath>\n",
                        escape_xml(thawed_path)
                    ));
                }
            }
            xml.push_str("  </index>\n");
        }

        xml.push_str("</indexes>");
        Ok(xml)
    }

    fn format_jobs(&self, jobs: &[SearchJobStatus]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<jobs>\n");

        for job in jobs {
            xml.push_str("  <job>\n");
            xml.push_str(&format!("    <sid>{}</sid>\n", escape_xml(&job.sid)));
            xml.push_str(&format!("    <done>{}</done>\n", job.is_done));
            xml.push_str(&format!(
                "    <progress>{:.1}</progress>\n",
                job.done_progress * 100.0
            ));
            xml.push_str(&format!("    <results>{}</results>\n", job.result_count));
            xml.push_str(&format!(
                "    <duration>{:.2}</duration>\n",
                job.run_duration
            ));
            xml.push_str("  </job>\n");
        }

        xml.push_str("</jobs>");
        Ok(xml)
    }

    fn format_cluster_info(
        &self,
        cluster_info: &ClusterInfoOutput,
        detailed: bool,
    ) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<cluster>\n");
        xml.push_str(&format!("  <id>{}</id>\n", escape_xml(&cluster_info.id)));
        if let Some(label) = &cluster_info.label {
            xml.push_str(&format!("  <label>{}</label>\n", escape_xml(label)));
        }
        xml.push_str(&format!(
            "  <mode>{}</mode>\n",
            escape_xml(&cluster_info.mode)
        ));
        if let Some(manager_uri) = &cluster_info.manager_uri {
            xml.push_str(&format!(
                "  <managerUri>{}</managerUri>\n",
                escape_xml(manager_uri)
            ));
        }
        if let Some(replication_factor) = cluster_info.replication_factor {
            xml.push_str(&format!(
                "  <replicationFactor>{}</replicationFactor>\n",
                replication_factor
            ));
        }
        if let Some(search_factor) = cluster_info.search_factor {
            xml.push_str(&format!(
                "  <searchFactor>{}</searchFactor>\n",
                search_factor
            ));
        }
        if let Some(status) = &cluster_info.status {
            xml.push_str(&format!("  <status>{}</status>\n", escape_xml(status)));
        }

        // Add peers if detailed
        if detailed && let Some(peers) = &cluster_info.peers {
            xml.push_str("  <peers>\n");
            for peer in peers {
                xml.push_str("    <peer>\n");
                xml.push_str(&format!("      <host>{}</host>\n", escape_xml(&peer.host)));
                xml.push_str(&format!("      <port>{}</port>\n", peer.port));
                xml.push_str(&format!("      <id>{}</id>\n", escape_xml(&peer.id)));
                xml.push_str(&format!(
                    "      <status>{}</status>\n",
                    escape_xml(&peer.status)
                ));
                xml.push_str(&format!(
                    "      <peerState>{}</peerState>\n",
                    escape_xml(&peer.peer_state)
                ));
                if let Some(label) = &peer.label {
                    xml.push_str(&format!("      <label>{}</label>\n", escape_xml(label)));
                }
                if let Some(site) = &peer.site {
                    xml.push_str(&format!("      <site>{}</site>\n", escape_xml(site)));
                }
                xml.push_str(&format!(
                    "      <isCaptain>{}</isCaptain>\n",
                    peer.is_captain
                ));
                xml.push_str("    </peer>\n");
            }
            xml.push_str("  </peers>\n");
        }

        xml.push_str("</cluster>");
        Ok(xml)
    }

    fn format_health(&self, health: &HealthCheckOutput) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<health>\n");

        if let Some(info) = &health.server_info {
            xml.push_str("  <serverInfo>\n");
            xml.push_str(&format!(
                "    <serverName>{}</serverName>\n",
                escape_xml(&info.server_name)
            ));
            xml.push_str(&format!(
                "    <version>{}</version>\n",
                escape_xml(&info.version)
            ));
            xml.push_str(&format!("    <build>{}</build>\n", escape_xml(&info.build)));
            if let Some(os) = &info.os_name {
                xml.push_str(&format!("    <osName>{}</osName>\n", escape_xml(os)));
            }
            xml.push_str("    <roles>\n");
            for role in &info.server_roles {
                xml.push_str(&format!("      <role>{}</role>\n", escape_xml(role)));
            }
            xml.push_str("    </roles>\n");
            xml.push_str("  </serverInfo>\n");
        }

        if let Some(sh) = &health.splunkd_health {
            xml.push_str("  <splunkdHealth>\n");
            xml.push_str(&format!(
                "    <status>{}</status>\n",
                escape_xml(&sh.health)
            ));
            xml.push_str("    <features>\n");
            for (name, feature) in &sh.features {
                xml.push_str(&format!("      <feature name=\"{}\">\n", escape_xml(name)));
                xml.push_str(&format!(
                    "        <health>{}</health>\n",
                    escape_xml(&feature.health)
                ));
                xml.push_str(&format!(
                    "        <status>{}</status>\n",
                    escape_xml(&feature.status)
                ));
                xml.push_str("        <reasons>\n");
                for reason in &feature.reasons {
                    xml.push_str(&format!(
                        "          <reason>{}</reason>\n",
                        escape_xml(reason)
                    ));
                }
                xml.push_str("        </reasons>\n");
                xml.push_str("      </feature>\n");
            }
            xml.push_str("    </features>\n");
            xml.push_str("  </splunkdHealth>\n");
        }

        if let Some(usage) = &health.license_usage {
            xml.push_str("  <licenseUsage>\n");
            for u in usage {
                xml.push_str("    <stack>\n");
                if let Some(stack_id) = &u.stack_id {
                    xml.push_str(&format!(
                        "      <stackId>{}</stackId>\n",
                        escape_xml(stack_id)
                    ));
                }
                xml.push_str(&format!(
                    "      <usedBytes>{}</usedBytes>\n",
                    u.effective_used_bytes()
                ));
                xml.push_str(&format!("      <quotaBytes>{}</quotaBytes>\n", u.quota));
                if let Some(slaves) = u.slaves_breakdown() {
                    xml.push_str("      <slaves>\n");
                    for (name, bytes) in slaves {
                        xml.push_str(&format!(
                            "        <slave name=\"{}\">{}</slave>\n",
                            escape_xml(name),
                            bytes
                        ));
                    }
                    xml.push_str("      </slaves>\n");
                }
                xml.push_str("    </stack>\n");
            }
            xml.push_str("  </licenseUsage>\n");
        }

        if let Some(kv) = &health.kvstore_status {
            xml.push_str("  <kvstoreStatus>\n");
            xml.push_str("    <currentMember>\n");
            xml.push_str(&format!(
                "      <host>{}</host>\n",
                escape_xml(&kv.current_member.host)
            ));
            xml.push_str(&format!("      <port>{}</port>\n", kv.current_member.port));
            xml.push_str(&format!(
                "      <status>{}</status>\n",
                escape_xml(&kv.current_member.status)
            ));
            xml.push_str(&format!(
                "      <replicaSet>{}</replicaSet>\n",
                escape_xml(&kv.current_member.replica_set)
            ));
            xml.push_str("    </currentMember>\n");
            xml.push_str("    <replicationStatus>\n");
            xml.push_str(&format!(
                "      <oplogSize>{}</oplogSize>\n",
                kv.replication_status.oplog_size
            ));
            xml.push_str(&format!(
                "      <oplogUsed>{:.2}</oplogUsed>\n",
                kv.replication_status.oplog_used
            ));
            xml.push_str("    </replicationStatus>\n");
            xml.push_str("  </kvstoreStatus>\n");
        }

        if let Some(lp) = &health.log_parsing_health {
            xml.push_str("  <logParsingHealth>\n");
            xml.push_str(&format!("    <isHealthy>{}</isHealthy>\n", lp.is_healthy));
            xml.push_str(&format!(
                "    <totalErrors>{}</totalErrors>\n",
                lp.total_errors
            ));
            xml.push_str(&format!(
                "    <timeWindow>{}</timeWindow>\n",
                escape_xml(&lp.time_window)
            ));
            xml.push_str("    <errors>\n");
            for err in &lp.errors {
                xml.push_str("      <error>\n");
                xml.push_str(&format!("        <time>{}</time>\n", escape_xml(&err.time)));
                xml.push_str(&format!(
                    "        <sourcetype>{}</sourcetype>\n",
                    escape_xml(&err.sourcetype)
                ));
                xml.push_str(&format!(
                    "        <logLevel>{}</logLevel>\n",
                    escape_xml(&err.log_level)
                ));
                xml.push_str(&format!(
                    "        <message>{}</message>\n",
                    escape_xml(&err.message)
                ));
                xml.push_str("      </error>\n");
            }
            xml.push_str("    </errors>\n");
            xml.push_str("  </logParsingHealth>\n");
        }

        xml.push_str("</health>");
        Ok(xml)
    }

    fn format_kvstore_status(&self, status: &KvStoreStatus) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<kvstoreStatus>\n");
        xml.push_str("  <currentMember>\n");
        xml.push_str(&format!(
            "    <host>{}</host>\n",
            escape_xml(&status.current_member.host)
        ));
        xml.push_str(&format!(
            "    <port>{}</port>\n",
            status.current_member.port
        ));
        xml.push_str(&format!(
            "    <status>{}</status>\n",
            escape_xml(&status.current_member.status)
        ));
        xml.push_str(&format!(
            "    <replicaSet>{}</replicaSet>\n",
            escape_xml(&status.current_member.replica_set)
        ));
        xml.push_str("  </currentMember>\n");
        xml.push_str("  <replicationStatus>\n");
        xml.push_str(&format!(
            "    <oplogSize>{}</oplogSize>\n",
            status.replication_status.oplog_size
        ));
        xml.push_str(&format!(
            "    <oplogUsed>{:.2}</oplogUsed>\n",
            status.replication_status.oplog_used
        ));
        xml.push_str("  </replicationStatus>\n");
        xml.push_str("</kvstoreStatus>");
        Ok(xml)
    }

    fn format_license(&self, license: &LicenseInfoOutput) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<licenseInfo>\n");

        xml.push_str("  <usage>\n");
        for u in &license.usage {
            xml.push_str("    <entry>\n");
            xml.push_str(&format!("      <name>{}</name>\n", escape_xml(&u.name)));
            if let Some(stack_id) = &u.stack_id {
                xml.push_str(&format!(
                    "      <stackId>{}</stackId>\n",
                    escape_xml(stack_id)
                ));
            }
            xml.push_str(&format!(
                "      <usedBytes>{}</usedBytes>\n",
                u.effective_used_bytes()
            ));
            xml.push_str(&format!("      <quotaBytes>{}</quotaBytes>\n", u.quota));
            xml.push_str("    </entry>\n");
        }
        xml.push_str("  </usage>\n");

        xml.push_str("  <pools>\n");
        for p in &license.pools {
            xml.push_str("    <pool>\n");
            xml.push_str(&format!("      <name>{}</name>\n", escape_xml(&p.name)));
            xml.push_str(&format!(
                "      <stackId>{}</stackId>\n",
                escape_xml(&p.stack_id)
            ));
            xml.push_str(&format!("      <usedBytes>{}</usedBytes>\n", p.used_bytes));
            xml.push_str(&format!(
                "      <quotaBytes>{}</quotaBytes>\n",
                escape_xml(&p.quota)
            ));
            if let Some(desc) = &p.description {
                xml.push_str(&format!(
                    "      <description>{}</description>\n",
                    escape_xml(desc)
                ));
            }
            xml.push_str("    </pool>\n");
        }
        xml.push_str("  </pools>\n");

        xml.push_str("  <stacks>\n");
        for s in &license.stacks {
            xml.push_str("    <stack>\n");
            xml.push_str(&format!("      <name>{}</name>\n", escape_xml(&s.name)));
            xml.push_str(&format!("      <label>{}</label>\n", escape_xml(&s.label)));
            xml.push_str(&format!(
                "      <type>{}</type>\n",
                escape_xml(&s.type_name)
            ));
            xml.push_str(&format!("      <quotaBytes>{}</quotaBytes>\n", s.quota));
            xml.push_str("    </stack>\n");
        }
        xml.push_str("  </stacks>\n");

        xml.push_str("</licenseInfo>");
        Ok(xml)
    }

    fn format_logs(&self, logs: &[LogEntry]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<logs>\n");

        for log in logs {
            xml.push_str("  <log>\n");
            xml.push_str(&format!("    <time>{}</time>\n", escape_xml(&log.time)));
            xml.push_str(&format!("    <level>{}</level>\n", escape_xml(&log.level)));
            xml.push_str(&format!(
                "    <component>{}</component>\n",
                escape_xml(&log.component)
            ));
            xml.push_str(&format!(
                "    <message>{}</message>\n",
                escape_xml(&log.message)
            ));
            xml.push_str("  </log>\n");
        }

        xml.push_str("</logs>");
        Ok(xml)
    }

    fn format_users(&self, users: &[User]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<users>\n");

        for user in users {
            xml.push_str("  <user>\n");
            xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&user.name)));

            if let Some(ref realname) = user.realname {
                xml.push_str(&format!(
                    "    <realname>{}</realname>\n",
                    escape_xml(realname)
                ));
            }

            if let Some(ref user_type) = user.user_type {
                xml.push_str(&format!("    <type>{}</type>\n", escape_xml(user_type)));
            }

            if let Some(ref default_app) = user.default_app {
                xml.push_str(&format!(
                    "    <defaultApp>{}</defaultApp>\n",
                    escape_xml(default_app)
                ));
            }

            if !user.roles.is_empty() {
                xml.push_str("    <roles>\n");
                for role in &user.roles {
                    xml.push_str(&format!("      <role>{}</role>\n", escape_xml(role)));
                }
                xml.push_str("    </roles>\n");
            }

            if let Some(last_login) = user.last_successful_login {
                xml.push_str(&format!(
                    "    <lastSuccessfulLogin>{}</lastSuccessfulLogin>\n",
                    last_login
                ));
            }

            xml.push_str("  </user>\n");
        }

        xml.push_str("</users>\n");
        Ok(xml)
    }

    fn format_apps(&self, apps: &[App]) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<apps>\n");

        for app in apps {
            xml.push_str("  <app>\n");
            xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&app.name)));

            if let Some(ref label) = app.label {
                xml.push_str(&format!("    <label>{}</label>\n", escape_xml(label)));
            }

            if let Some(ref version) = app.version {
                xml.push_str(&format!("    <version>{}</version>\n", escape_xml(version)));
            }

            xml.push_str(&format!("    <disabled>{}</disabled>\n", app.disabled));

            if let Some(ref author) = app.author {
                xml.push_str(&format!("    <author>{}</author>\n", escape_xml(author)));
            }

            xml.push_str("  </app>\n");
        }

        xml.push_str("</apps>");
        Ok(xml)
    }

    fn format_app_info(&self, app: &App) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<app>\n");

        xml.push_str(&format!("  <name>{}</name>\n", escape_xml(&app.name)));

        if let Some(ref label) = app.label {
            xml.push_str(&format!("  <label>{}</label>\n", escape_xml(label)));
        }

        if let Some(ref version) = app.version {
            xml.push_str(&format!("  <version>{}</version>\n", escape_xml(version)));
        }

        xml.push_str(&format!("  <disabled>{}</disabled>\n", app.disabled));

        if let Some(ref author) = app.author {
            xml.push_str(&format!("  <author>{}</author>\n", escape_xml(author)));
        }

        if let Some(ref desc) = app.description {
            xml.push_str(&format!(
                "  <description>{}</description>\n",
                escape_xml(desc)
            ));
        }

        if let Some(configured) = app.is_configured {
            xml.push_str(&format!(
                "  <is_configured>{}</is_configured>\n",
                configured
            ));
        }

        if let Some(visible) = app.is_visible {
            xml.push_str(&format!("  <is_visible>{}</is_visible>\n", visible));
        }

        xml.push_str("</app>");
        Ok(xml)
    }

    fn format_list_all(&self, output: &ListAllOutput) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<list_all>\n");
        xml.push_str(&format!(
            "  <timestamp>{}</timestamp>\n",
            escape_xml(&output.timestamp)
        ));
        xml.push_str("  <resources>\n");

        for resource in &output.resources {
            xml.push_str("    <resource>\n");
            xml.push_str(&format!(
                "      <type>{}</type>\n",
                escape_xml(&resource.resource_type)
            ));
            xml.push_str(&format!("      <count>{}</count>\n", resource.count));
            xml.push_str(&format!(
                "      <status>{}</status>\n",
                escape_xml(&resource.status)
            ));
            if let Some(error) = &resource.error {
                xml.push_str(&format!("      <error>{}</error>\n", escape_xml(error)));
            }
            xml.push_str("    </resource>\n");
        }

        xml.push_str("  </resources>\n");
        xml.push_str("</list_all>");
        Ok(xml)
    }

    fn format_saved_searches(&self, searches: &[SavedSearch]) -> Result<String> {
        let mut xml =
            String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<saved-searches>\n");

        for search in searches {
            xml.push_str("  <saved-search>\n");
            xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&search.name)));
            xml.push_str(&format!("    <disabled>{}</disabled>\n", search.disabled));
            if let Some(ref desc) = search.description {
                xml.push_str(&format!(
                    "    <description>{}</description>\n",
                    escape_xml(desc)
                ));
            }
            xml.push_str("  </saved-search>\n");
        }

        xml.push_str("</saved-searches>");
        Ok(xml)
    }

    fn format_saved_search_info(&self, search: &SavedSearch) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<saved-search>\n");

        xml.push_str(&format!("  <name>{}</name>\n", escape_xml(&search.name)));
        xml.push_str(&format!("  <disabled>{}</disabled>\n", search.disabled));
        if let Some(ref desc) = search.description {
            xml.push_str(&format!(
                "  <description>{}</description>\n",
                escape_xml(desc)
            ));
        }
        xml.push_str(&format!(
            "  <search>{}</search>\n",
            escape_xml(&search.search)
        ));
        xml.push_str("</saved-search>");
        Ok(xml)
    }

    fn format_job_details(&self, job: &SearchJobStatus) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<job>\n");

        xml.push_str(&format!("  <sid>{}</sid>\n", escape_xml(&job.sid)));
        xml.push_str(&format!("  <done>{}</done>\n", job.is_done));
        xml.push_str(&format!("  <finalized>{}</finalized>\n", job.is_finalized));
        xml.push_str(&format!(
            "  <progress>{:.2}</progress>\n",
            job.done_progress * 100.0
        ));
        xml.push_str(&format!("  <duration>{:.2}</duration>\n", job.run_duration));
        xml.push_str(&format!("  <scanCount>{}</scanCount>\n", job.scan_count));
        xml.push_str(&format!("  <eventCount>{}</eventCount>\n", job.event_count));
        xml.push_str(&format!(
            "  <resultCount>{}</resultCount>\n",
            job.result_count
        ));
        xml.push_str(&format!("  <diskUsage>{}</diskUsage>\n", job.disk_usage));

        if let Some(priority) = job.priority {
            xml.push_str(&format!("  <priority>{}</priority>\n", priority));
        }
        if let Some(ref cursor_time) = job.cursor_time {
            xml.push_str(&format!(
                "  <cursorTime>{}</cursorTime>\n",
                escape_xml(cursor_time)
            ));
        }
        if let Some(ref label) = job.label {
            xml.push_str(&format!("  <label>{}</label>\n", escape_xml(label)));
        }

        xml.push_str("</job>");
        Ok(xml)
    }

    fn format_profile(&self, profile_name: &str, profile: &ProfileConfig) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<profile>\n");

        xml.push_str(&format!("  <name>{}</name>\n", escape_xml(profile_name)));

        let base_url = profile.base_url.as_deref().unwrap_or("");
        xml.push_str(&format!(
            "  <base_url>{}</base_url>\n",
            escape_xml(base_url)
        ));

        let username = profile.username.as_deref().unwrap_or("");
        xml.push_str(&format!(
            "  <username>{}</username>\n",
            escape_xml(username)
        ));

        let password_display = match &profile.password {
            Some(_) => "****",
            None => "",
        };
        xml.push_str(&format!(
            "  <password>{}</password>\n",
            escape_xml(password_display)
        ));

        let token_display = match &profile.api_token {
            Some(_) => "****",
            None => "",
        };
        xml.push_str(&format!(
            "  <api_token>{}</api_token>\n",
            escape_xml(token_display)
        ));

        if let Some(skip_verify) = profile.skip_verify {
            xml.push_str(&format!("  <skip_verify>{}</skip_verify>\n", skip_verify));
        }

        if let Some(timeout) = profile.timeout_seconds {
            xml.push_str(&format!(
                "  <timeout_seconds>{}</timeout_seconds>\n",
                timeout
            ));
        }

        if let Some(max_retries) = profile.max_retries {
            xml.push_str(&format!("  <max_retries>{}</max_retries>\n", max_retries));
        }

        xml.push_str("</profile>");
        Ok(xml)
    }

    fn format_profiles(&self, profiles: &BTreeMap<String, ProfileConfig>) -> Result<String> {
        let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<profiles>\n");
        for (name, profile) in profiles {
            xml.push_str("  <profile>\n");
            xml.push_str(&format!("    <name>{}</name>\n", escape_xml(name)));
            if let Some(ref url) = profile.base_url {
                xml.push_str(&format!("    <base_url>{}</base_url>\n", escape_xml(url)));
            }
            if let Some(ref user) = profile.username {
                xml.push_str(&format!("    <username>{}</username>\n", escape_xml(user)));
            }
            if let Some(skip) = profile.skip_verify {
                xml.push_str(&format!("    <skip_verify>{}</skip_verify>\n", skip));
            }
            if let Some(timeout) = profile.timeout_seconds {
                xml.push_str(&format!(
                    "    <timeout_seconds>{}</timeout_seconds>\n",
                    timeout
                ));
            }
            if let Some(retries) = profile.max_retries {
                xml.push_str(&format!("    <max_retries>{}</max_retries>\n", retries));
            }
            xml.push_str("  </profile>\n");
        }
        xml.push_str("</profiles>\n");
        Ok(xml)
    }
}

/// Convert a JSON value to nested XML element(s).
///
/// Returns a vector of XML element strings. For primitive values, returns
/// a single element. For arrays and objects, returns multiple nested elements.
fn value_to_xml_elements(name: &str, value: &serde_json::Value, indent: &str) -> Vec<String> {
    match value {
        serde_json::Value::Null => {
            vec![format!(
                "{}<{}></{}>",
                indent,
                escape_xml(name),
                escape_xml(name)
            )]
        }
        serde_json::Value::Bool(b) => {
            vec![format!(
                "{}<{}>{}</{}>",
                indent,
                escape_xml(name),
                b,
                escape_xml(name)
            )]
        }
        serde_json::Value::Number(n) => {
            vec![format!(
                "{}<{}>{}</{}>",
                indent,
                escape_xml(name),
                n,
                escape_xml(name)
            )]
        }
        serde_json::Value::String(s) => {
            vec![format!(
                "{}<{}>{}</{}>",
                indent,
                escape_xml(name),
                escape_xml(s),
                escape_xml(name)
            )]
        }
        serde_json::Value::Array(arr) => {
            let mut elems = vec![format!("{}<{}>", indent, escape_xml(name))];
            for item in arr.iter() {
                let item_name = "item";
                elems.extend(value_to_xml_elements(
                    item_name,
                    item,
                    &format!("{}  ", indent),
                ));
            }
            elems.push(format!("{}</{}>", indent, escape_xml(name)));
            elems
        }
        serde_json::Value::Object(obj) => {
            let mut elems = vec![format!("{}<{}>", indent, escape_xml(name))];
            for (key, val) in obj {
                elems.extend(value_to_xml_elements(key, val, &format!("{}  ", indent)));
            }
            elems.push(format!("{}</{}>", indent, escape_xml(name)));
            elems
        }
    }
}
