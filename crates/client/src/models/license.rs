//! License models for Splunk license management API.
//!
//! This module contains types for license usage, pools, and stacks.
//! Includes helper methods for calculating effective usage.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// License usage information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LicenseUsage {
    #[serde(default)]
    pub name: String,
    #[serde(deserialize_with = "crate::serde_helpers::u64_from_string_or_number")]
    pub quota: u64,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::opt_u64_from_string_or_number"
    )]
    pub used_bytes: Option<u64>,
    #[serde(default)]
    pub slaves_usage_bytes: Option<SlavesUsageBytes>,
    pub stack_id: Option<String>,
}

/// License usage can be returned either as a total (standalone) or per-slave breakdown (cluster).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum SlavesUsageBytes {
    Total(#[serde(deserialize_with = "crate::serde_helpers::u64_from_string_or_number")] u64),
    PerSlave(
        #[serde(
            deserialize_with = "crate::serde_helpers::map_string_to_u64_from_string_or_number"
        )]
        HashMap<String, u64>,
    ),
}

impl LicenseUsage {
    /// Returns the best-effort used bytes for display:
    /// - Prefer `used_bytes` when present
    /// - Otherwise use `slaves_usage_bytes` (total or sum of per-slave values)
    pub fn effective_used_bytes(&self) -> u64 {
        if let Some(used) = self.used_bytes {
            return used;
        }
        match &self.slaves_usage_bytes {
            Some(SlavesUsageBytes::Total(total)) => *total,
            Some(SlavesUsageBytes::PerSlave(map)) => map.values().sum(),
            None => 0,
        }
    }

    /// Returns per-slave usage when Splunk provides a breakdown.
    pub fn slaves_breakdown(&self) -> Option<&HashMap<String, u64>> {
        match &self.slaves_usage_bytes {
            Some(SlavesUsageBytes::PerSlave(map)) => Some(map),
            _ => None,
        }
    }
}

/// License pool information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LicensePool {
    #[serde(default)]
    pub name: String,
    #[serde(deserialize_with = "crate::serde_helpers::string_from_number_or_string")]
    pub quota: String,
    #[serde(deserialize_with = "crate::serde_helpers::u64_from_string_or_number")]
    pub used_bytes: u64,
    pub stack_id: String,
    pub description: Option<String>,
}

/// License stack information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LicenseStack {
    #[serde(default)]
    pub name: String,
    #[serde(deserialize_with = "crate::serde_helpers::u64_from_string_or_number")]
    pub quota: u64,
    #[serde(rename = "type")]
    pub type_name: String,
    pub label: String,
}
