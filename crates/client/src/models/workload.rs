//! Workload management models for Splunk workload API.
//!
//! This module contains types for listing and managing Splunk workload
//! pools and rules for resource allocation.
//!
//! # What this module handles:
//! - Deserialization of workload pool and rule data from Splunk REST API
//! - Type-safe representation of workload management metadata
//!
//! # What this module does NOT handle:
//! - Direct HTTP API calls (see [`crate::endpoints::workload`])
//! - Client-side filtering or searching
//! - Modifying workload configuration (read-only for initial implementation)

use serde::{Deserialize, Serialize};
use std::fmt;

/// Type of search for workload rule matching.
///
/// Represents the different search types that can be used in workload
/// rule predicates to match searches and assign them to specific pools.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum SearchType {
    /// Ad-hoc searches initiated by users.
    Adhoc,
    /// Scheduled searches (reports, alerts).
    Scheduled,
    /// Data model acceleration searches.
    Datamodel,
    /// Unknown or unrecognized search type.
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for SearchType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            SearchType::Adhoc => "adhoc",
            SearchType::Scheduled => "scheduled",
            SearchType::Datamodel => "datamodel",
            SearchType::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

/// Workload Pool information.
///
/// Represents a Splunk workload pool that allocates CPU and memory resources
/// to search workloads.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkloadPool {
    /// The pool name.
    #[serde(default)]
    pub name: String,
    /// CPU weight for resource allocation.
    #[serde(rename = "cpuWeight")]
    pub cpu_weight: Option<u32>,
    /// Memory weight for resource allocation.
    #[serde(rename = "memWeight")]
    pub mem_weight: Option<u32>,
    /// Whether this is the default pool.
    #[serde(rename = "defaultPool")]
    pub default_pool: Option<bool>,
    /// Whether the pool is enabled.
    pub enabled: Option<bool>,
    /// Maximum concurrent searches allowed.
    #[serde(rename = "searchConcurrency")]
    pub search_concurrency: Option<u32>,
    /// Search time range restriction.
    #[serde(rename = "searchTimeRange")]
    pub search_time_range: Option<String>,
    /// Whether admission rules are enabled for this pool.
    #[serde(rename = "admissionRulesEnabled")]
    pub admission_rules_enabled: Option<bool>,
    /// CPU cores allocated to this pool.
    #[serde(rename = "cpuCores")]
    pub cpu_cores: Option<f64>,
    /// Memory limit in MB.
    #[serde(rename = "memLimit")]
    pub mem_limit: Option<usize>,
}

/// Workload Rule information.
///
/// Represents a Splunk workload rule that assigns searches to specific
/// pools based on criteria like user, app, or search type.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct WorkloadRule {
    /// The rule name.
    #[serde(default)]
    pub name: String,
    /// The predicate (condition) for matching searches.
    pub predicate: Option<String>,
    /// The target workload pool for matched searches.
    #[serde(rename = "workloadPool")]
    pub workload_pool: Option<String>,
    /// User to match (if specified).
    pub user: Option<String>,
    /// App to match (if specified).
    pub app: Option<String>,
    /// Search type to match (if specified).
    #[serde(rename = "searchType", skip_serializing_if = "Option::is_none")]
    pub search_type: Option<SearchType>,
    /// Search time range to match (if specified).
    #[serde(rename = "searchTimeRange")]
    pub search_time_range: Option<String>,
    /// Whether the rule is enabled.
    pub enabled: Option<bool>,
    /// Order/priority of the rule.
    pub order: Option<u32>,
}

/// Workload Pool list response wrapper.
#[derive(Debug, Deserialize, Clone)]
pub struct WorkloadPoolListResponse {
    /// The list of pool entries returned by the API.
    pub entry: Vec<WorkloadPoolEntry>,
}

/// A single workload pool entry in the list response.
#[derive(Debug, Deserialize, Clone)]
pub struct WorkloadPoolEntry {
    /// The entry name (pool identifier).
    pub name: String,
    /// The pool content/data.
    pub content: WorkloadPool,
}

/// Workload Rule list response wrapper.
#[derive(Debug, Deserialize, Clone)]
pub struct WorkloadRuleListResponse {
    /// The list of rule entries returned by the API.
    pub entry: Vec<WorkloadRuleEntry>,
}

/// A single workload rule entry in the list response.
#[derive(Debug, Deserialize, Clone)]
pub struct WorkloadRuleEntry {
    /// The entry name (rule identifier).
    pub name: String,
    /// The rule content/data.
    pub content: WorkloadRule,
}

#[cfg(test)]
mod tests {
    use super::*;

    mod search_type_tests {
        use super::*;

        #[test]
        fn deserialize_adhoc() {
            let json = r#""adhoc""#;
            let result: SearchType = serde_json::from_str(json).unwrap();
            assert_eq!(result, SearchType::Adhoc);
        }

        #[test]
        fn deserialize_scheduled() {
            let json = r#""scheduled""#;
            let result: SearchType = serde_json::from_str(json).unwrap();
            assert_eq!(result, SearchType::Scheduled);
        }

        #[test]
        fn deserialize_datamodel() {
            let json = r#""datamodel""#;
            let result: SearchType = serde_json::from_str(json).unwrap();
            assert_eq!(result, SearchType::Datamodel);
        }

        #[test]
        fn deserialize_unknown_value_fallback() {
            // Unknown values should deserialize to Unknown variant
            let json = r#""some_random_type""#;
            let result: SearchType = serde_json::from_str(json).unwrap();
            assert_eq!(result, SearchType::Unknown);
        }

        #[test]
        fn deserialize_empty_string_fallback() {
            let json = r#""""#;
            let result: SearchType = serde_json::from_str(json).unwrap();
            assert_eq!(result, SearchType::Unknown);
        }

        #[test]
        fn display_adhoc() {
            assert_eq!(SearchType::Adhoc.to_string(), "adhoc");
        }

        #[test]
        fn display_scheduled() {
            assert_eq!(SearchType::Scheduled.to_string(), "scheduled");
        }

        #[test]
        fn display_datamodel() {
            assert_eq!(SearchType::Datamodel.to_string(), "datamodel");
        }

        #[test]
        fn display_unknown() {
            assert_eq!(SearchType::Unknown.to_string(), "unknown");
        }

        #[test]
        fn default_is_unknown() {
            assert_eq!(SearchType::default(), SearchType::Unknown);
        }

        #[test]
        fn serialize_adhoc() {
            let search_type = SearchType::Adhoc;
            let json = serde_json::to_string(&search_type).unwrap();
            assert_eq!(json, r#""adhoc""#);
        }

        #[test]
        fn serialize_scheduled() {
            let search_type = SearchType::Scheduled;
            let json = serde_json::to_string(&search_type).unwrap();
            assert_eq!(json, r#""scheduled""#);
        }

        #[test]
        fn serialize_datamodel() {
            let search_type = SearchType::Datamodel;
            let json = serde_json::to_string(&search_type).unwrap();
            assert_eq!(json, r#""datamodel""#);
        }

        #[test]
        fn serialize_unknown() {
            let search_type = SearchType::Unknown;
            let json = serde_json::to_string(&search_type).unwrap();
            assert_eq!(json, r#""unknown""#);
        }
    }

    mod workload_rule_integration_tests {
        use super::*;

        #[test]
        fn deserialize_workload_rule_with_adhoc_search_type() {
            let json = r#"{
                "name": "rule1",
                "searchType": "adhoc",
                "enabled": true
            }"#;
            let rule: WorkloadRule = serde_json::from_str(json).unwrap();
            assert_eq!(rule.name, "rule1");
            assert_eq!(rule.search_type, Some(SearchType::Adhoc));
            assert_eq!(rule.enabled, Some(true));
        }

        #[test]
        fn deserialize_workload_rule_with_scheduled_search_type() {
            let json = r#"{
                "name": "rule2",
                "searchType": "scheduled"
            }"#;
            let rule: WorkloadRule = serde_json::from_str(json).unwrap();
            assert_eq!(rule.search_type, Some(SearchType::Scheduled));
        }

        #[test]
        fn deserialize_workload_rule_with_datamodel_search_type() {
            let json = r#"{
                "name": "rule3",
                "searchType": "datamodel"
            }"#;
            let rule: WorkloadRule = serde_json::from_str(json).unwrap();
            assert_eq!(rule.search_type, Some(SearchType::Datamodel));
        }

        #[test]
        fn deserialize_workload_rule_with_unknown_search_type() {
            let json = r#"{
                "name": "rule4",
                "searchType": "custom_type"
            }"#;
            let rule: WorkloadRule = serde_json::from_str(json).unwrap();
            assert_eq!(rule.search_type, Some(SearchType::Unknown));
        }

        #[test]
        fn deserialize_workload_rule_without_search_type() {
            let json = r#"{
                "name": "rule5"
            }"#;
            let rule: WorkloadRule = serde_json::from_str(json).unwrap();
            assert_eq!(rule.search_type, None);
        }

        #[test]
        fn deserialize_workload_rule_with_null_search_type() {
            let json = r#"{
                "name": "rule6",
                "searchType": null
            }"#;
            let rule: WorkloadRule = serde_json::from_str(json).unwrap();
            assert_eq!(rule.search_type, None);
        }

        #[test]
        fn serialize_workload_rule_with_search_type() {
            let rule = WorkloadRule {
                name: "test_rule".to_string(),
                predicate: Some("user=admin".to_string()),
                workload_pool: Some("pool1".to_string()),
                user: Some("admin".to_string()),
                app: None,
                search_type: Some(SearchType::Adhoc),
                search_time_range: None,
                enabled: Some(true),
                order: Some(1),
            };
            let json = serde_json::to_string(&rule).unwrap();
            assert!(json.contains("\"searchType\":\"adhoc\""));
        }

        #[test]
        fn serialize_workload_rule_without_search_type() {
            let rule = WorkloadRule {
                name: "test_rule".to_string(),
                predicate: None,
                workload_pool: None,
                user: None,
                app: None,
                search_type: None,
                search_time_range: None,
                enabled: None,
                order: None,
            };
            let json = serde_json::to_string(&rule).unwrap();
            // When search_type is None, it should not appear in serialized output
            assert!(!json.contains("searchType"));
        }
    }
}
