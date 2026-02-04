//! Workload management XML formatter.
//!
//! Responsibilities:
//! - Format workload pools and rules as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_client::{WorkloadPool, WorkloadRule};

/// Format workload pools as XML.
pub fn format_workload_pools(pools: &[WorkloadPool], detailed: bool) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<workload_pools>\n");

    for pool in pools {
        xml.push_str("  <pool>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&pool.name)));

        if let Some(cpu_weight) = pool.cpu_weight {
            xml.push_str(&format!("    <cpuWeight>{}</cpuWeight>\n", cpu_weight));
        }

        if let Some(mem_weight) = pool.mem_weight {
            xml.push_str(&format!("    <memWeight>{}</memWeight>\n", mem_weight));
        }

        if let Some(default) = pool.default_pool {
            xml.push_str(&format!("    <defaultPool>{}</defaultPool>\n", default));
        }

        if let Some(enabled) = pool.enabled {
            xml.push_str(&format!("    <enabled>{}</enabled>\n", enabled));
        }

        if detailed {
            if let Some(concurrency) = pool.search_concurrency {
                xml.push_str(&format!(
                    "    <searchConcurrency>{}</searchConcurrency>\n",
                    concurrency
                ));
            }

            if let Some(ref time_range) = pool.search_time_range {
                xml.push_str(&format!(
                    "    <searchTimeRange>{}</searchTimeRange>\n",
                    escape_xml(time_range)
                ));
            }

            if let Some(admission) = pool.admission_rules_enabled {
                xml.push_str(&format!(
                    "    <admissionRulesEnabled>{}</admissionRulesEnabled>\n",
                    admission
                ));
            }

            if let Some(cpu_cores) = pool.cpu_cores {
                xml.push_str(&format!("    <cpuCores>{}</cpuCores>\n", cpu_cores));
            }

            if let Some(mem_limit) = pool.mem_limit {
                xml.push_str(&format!("    <memLimit>{}</memLimit>\n", mem_limit));
            }
        }

        xml.push_str("  </pool>\n");
    }

    xml.push_str("</workload_pools>");
    Ok(xml)
}

/// Format workload rules as XML.
pub fn format_workload_rules(rules: &[WorkloadRule], detailed: bool) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<workload_rules>\n");

    for rule in rules {
        xml.push_str("  <rule>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&rule.name)));

        if let Some(ref predicate) = rule.predicate {
            xml.push_str(&format!(
                "    <predicate>{}</predicate>\n",
                escape_xml(predicate)
            ));
        }

        if let Some(ref pool) = rule.workload_pool {
            xml.push_str(&format!(
                "    <workloadPool>{}</workloadPool>\n",
                escape_xml(pool)
            ));
        }

        if let Some(enabled) = rule.enabled {
            xml.push_str(&format!("    <enabled>{}</enabled>\n", enabled));
        }

        if detailed {
            if let Some(ref user) = rule.user {
                xml.push_str(&format!("    <user>{}</user>\n", escape_xml(user)));
            }

            if let Some(ref app) = rule.app {
                xml.push_str(&format!("    <app>{}</app>\n", escape_xml(app)));
            }

            if let Some(ref search_type) = rule.search_type {
                xml.push_str(&format!(
                    "    <searchType>{}</searchType>\n",
                    escape_xml(search_type)
                ));
            }

            if let Some(ref time_range) = rule.search_time_range {
                xml.push_str(&format!(
                    "    <searchTimeRange>{}</searchTimeRange>\n",
                    escape_xml(time_range)
                ));
            }

            if let Some(order) = rule.order {
                xml.push_str(&format!("    <order>{}</order>\n", order));
            }
        }

        xml.push_str("  </rule>\n");
    }

    xml.push_str("</workload_rules>");
    Ok(xml)
}
