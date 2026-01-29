//! Health check XML formatter.
//!
//! Responsibilities:
//! - Format health check results and KVStore status as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_client::{HealthCheckOutput, KvStoreStatus};

/// Format health check results as XML.
pub fn format_health(health: &HealthCheckOutput) -> Result<String> {
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

/// Format KVStore status as XML.
pub fn format_kvstore_status(status: &KvStoreStatus) -> Result<String> {
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
