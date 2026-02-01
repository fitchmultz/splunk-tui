//! License models for Splunk license management API.
//!
//! This module contains types for license usage, pools, stacks, and installation.
//! Includes helper methods for calculating effective usage and managing licenses.
//!
//! # What this module handles:
//! - License usage information (quota, used bytes, per-slave breakdown)
//! - License pools (quota allocation across stacks)
//! - License stacks (license grouping)
//! - Installed licenses (license file management)
//! - License installation and configuration parameters
//!
//! # What this module does NOT handle:
//! - HTTP API calls (see `crate::endpoints::license`)
//! - License violation handling or alerting
//! - License file parsing (.sla format internals)

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

/// Represents an installed license on the Splunk server.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct InstalledLicense {
    /// License name/identifier
    #[serde(default)]
    pub name: String,
    /// License type (e.g., "enterprise", "forwarder")
    #[serde(rename = "type")]
    pub license_type: String,
    /// License status (e.g., "active", "inactive")
    #[serde(default)]
    pub status: String,
    /// License quota in bytes
    #[serde(deserialize_with = "crate::serde_helpers::u64_from_string_or_number")]
    pub quota_bytes: u64,
    /// License expiration time (ISO 8601 format)
    pub expiration_time: Option<String>,
    /// Features enabled by this license
    #[serde(default)]
    pub features: Vec<String>,
}

impl InstalledLicense {
    /// Check if the license is currently active.
    pub fn is_active(&self) -> bool {
        self.status.eq_ignore_ascii_case("active")
    }
}

/// Parameters for creating a new license pool.
#[derive(Debug, Clone, Default)]
pub struct CreatePoolParams {
    /// Pool name (required)
    pub name: String,
    /// Stack ID to associate with (required)
    pub stack_id: String,
    /// Quota in bytes (optional, defaults to stack quota)
    pub quota_bytes: Option<u64>,
    /// Pool description (optional)
    pub description: Option<String>,
}

/// Parameters for modifying an existing license pool.
#[derive(Debug, Clone, Default)]
pub struct ModifyPoolParams {
    /// New quota in bytes (optional)
    pub quota_bytes: Option<u64>,
    /// New description (optional)
    pub description: Option<String>,
}

/// Result of a license installation operation.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LicenseInstallResult {
    /// Whether the installation was successful
    pub success: bool,
    /// Human-readable message
    pub message: String,
    /// Name of the installed license (if successful)
    pub license_name: Option<String>,
}

/// License activation/deactivation result.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LicenseActivationResult {
    /// Whether the operation was successful
    pub success: bool,
    /// Human-readable message
    pub message: String,
}
