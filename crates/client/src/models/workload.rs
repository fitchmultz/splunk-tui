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
    pub mem_limit: Option<u64>,
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
    #[serde(rename = "searchType")]
    pub search_type: Option<String>,
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
