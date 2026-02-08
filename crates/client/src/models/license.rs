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
use std::fmt;

/// License type enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum LicenseType {
    /// Enterprise license type.
    Enterprise,
    /// Forwarder license type.
    Forwarder,
    /// Free license type.
    Free,
    /// Trial license type.
    Trial,
    /// Unknown license type (fallback for unrecognized values).
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for LicenseType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Enterprise => "enterprise",
            Self::Forwarder => "forwarder",
            Self::Free => "free",
            Self::Trial => "trial",
            Self::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

/// License status enumeration.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum LicenseStatus {
    /// License is active and valid.
    Active,
    /// License is inactive.
    Inactive,
    /// License has expired.
    Expired,
    /// Unknown license status (fallback for unrecognized values).
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for LicenseStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let s = match self {
            Self::Active => "active",
            Self::Inactive => "inactive",
            Self::Expired => "expired",
            Self::Unknown => "unknown",
        };
        write!(f, "{}", s)
    }
}

/// License usage information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LicenseUsage {
    #[serde(default)]
    pub name: String,
    #[serde(deserialize_with = "crate::serde_helpers::usize_from_string_or_number")]
    pub quota: usize,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::opt_usize_from_string_or_number"
    )]
    pub used_bytes: Option<usize>,
    #[serde(default)]
    pub slaves_usage_bytes: Option<SlavesUsageBytes>,
    pub stack_id: Option<String>,
}

/// License usage can be returned either as a total (standalone) or per-slave breakdown (cluster).
#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(untagged)]
pub enum SlavesUsageBytes {
    Total(#[serde(deserialize_with = "crate::serde_helpers::usize_from_string_or_number")] usize),
    PerSlave(
        #[serde(
            deserialize_with = "crate::serde_helpers::map_string_to_usize_from_string_or_number"
        )]
        HashMap<String, usize>,
    ),
}

impl LicenseUsage {
    /// Returns the best-effort used bytes for display:
    /// - Prefer `used_bytes` when present
    /// - Otherwise use `slaves_usage_bytes` (total or sum of per-slave values)
    pub fn effective_used_bytes(&self) -> usize {
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
    pub fn slaves_breakdown(&self) -> Option<&HashMap<String, usize>> {
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
    #[serde(deserialize_with = "crate::serde_helpers::usize_from_string_or_number")]
    pub used_bytes: usize,
    pub stack_id: String,
    pub description: Option<String>,
}

/// License stack information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LicenseStack {
    #[serde(default)]
    pub name: String,
    #[serde(deserialize_with = "crate::serde_helpers::usize_from_string_or_number")]
    pub quota: usize,
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
    pub license_type: LicenseType,
    /// License status (e.g., "active", "inactive")
    #[serde(default)]
    pub status: LicenseStatus,
    /// License quota in bytes
    #[serde(deserialize_with = "crate::serde_helpers::usize_from_string_or_number")]
    pub quota_bytes: usize,
    /// License expiration time (ISO 8601 format)
    pub expiration_time: Option<String>,
    /// Features enabled by this license
    #[serde(default)]
    pub features: Vec<String>,
}

impl InstalledLicense {
    /// Check if the license is currently active.
    pub fn is_active(&self) -> bool {
        self.status == LicenseStatus::Active
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
    pub quota_bytes: Option<usize>,
    /// Pool description (optional)
    pub description: Option<String>,
}

/// Parameters for modifying an existing license pool.
#[derive(Debug, Clone, Default)]
pub struct ModifyPoolParams {
    /// New quota in bytes (optional)
    pub quota_bytes: Option<usize>,
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

#[cfg(test)]
mod tests {
    use super::*;

    // LicenseType tests

    #[test]
    fn test_license_type_default_is_unknown() {
        assert_eq!(LicenseType::default(), LicenseType::Unknown);
    }

    #[test]
    fn test_license_type_deserialize_known_values() {
        assert_eq!(
            serde_json::from_str::<LicenseType>("\"enterprise\"").unwrap(),
            LicenseType::Enterprise
        );
        assert_eq!(
            serde_json::from_str::<LicenseType>("\"forwarder\"").unwrap(),
            LicenseType::Forwarder
        );
        assert_eq!(
            serde_json::from_str::<LicenseType>("\"free\"").unwrap(),
            LicenseType::Free
        );
        assert_eq!(
            serde_json::from_str::<LicenseType>("\"trial\"").unwrap(),
            LicenseType::Trial
        );
    }

    #[test]
    fn test_license_type_deserialize_unknown_value() {
        // Unknown values should deserialize to Unknown variant
        assert_eq!(
            serde_json::from_str::<LicenseType>("\"custom\"").unwrap(),
            LicenseType::Unknown
        );
        assert_eq!(
            serde_json::from_str::<LicenseType>("\"unknown\"").unwrap(),
            LicenseType::Unknown
        );
    }

    #[test]
    fn test_license_type_serialize() {
        assert_eq!(
            serde_json::to_string(&LicenseType::Enterprise).unwrap(),
            "\"enterprise\""
        );
        assert_eq!(
            serde_json::to_string(&LicenseType::Forwarder).unwrap(),
            "\"forwarder\""
        );
        assert_eq!(
            serde_json::to_string(&LicenseType::Free).unwrap(),
            "\"free\""
        );
        assert_eq!(
            serde_json::to_string(&LicenseType::Trial).unwrap(),
            "\"trial\""
        );
        assert_eq!(
            serde_json::to_string(&LicenseType::Unknown).unwrap(),
            "\"unknown\""
        );
    }

    #[test]
    fn test_license_type_display() {
        assert_eq!(format!("{}", LicenseType::Enterprise), "enterprise");
        assert_eq!(format!("{}", LicenseType::Forwarder), "forwarder");
        assert_eq!(format!("{}", LicenseType::Free), "free");
        assert_eq!(format!("{}", LicenseType::Trial), "trial");
        assert_eq!(format!("{}", LicenseType::Unknown), "unknown");
    }

    // LicenseStatus tests

    #[test]
    fn test_license_status_default_is_unknown() {
        assert_eq!(LicenseStatus::default(), LicenseStatus::Unknown);
    }

    #[test]
    fn test_license_status_deserialize_known_values() {
        assert_eq!(
            serde_json::from_str::<LicenseStatus>("\"active\"").unwrap(),
            LicenseStatus::Active
        );
        assert_eq!(
            serde_json::from_str::<LicenseStatus>("\"inactive\"").unwrap(),
            LicenseStatus::Inactive
        );
        assert_eq!(
            serde_json::from_str::<LicenseStatus>("\"expired\"").unwrap(),
            LicenseStatus::Expired
        );
    }

    #[test]
    fn test_license_status_deserialize_unknown_value() {
        // Unknown values should deserialize to Unknown variant
        assert_eq!(
            serde_json::from_str::<LicenseStatus>("\"pending\"").unwrap(),
            LicenseStatus::Unknown
        );
        assert_eq!(
            serde_json::from_str::<LicenseStatus>("\"unknown\"").unwrap(),
            LicenseStatus::Unknown
        );
    }

    #[test]
    fn test_license_status_serialize() {
        assert_eq!(
            serde_json::to_string(&LicenseStatus::Active).unwrap(),
            "\"active\""
        );
        assert_eq!(
            serde_json::to_string(&LicenseStatus::Inactive).unwrap(),
            "\"inactive\""
        );
        assert_eq!(
            serde_json::to_string(&LicenseStatus::Expired).unwrap(),
            "\"expired\""
        );
        assert_eq!(
            serde_json::to_string(&LicenseStatus::Unknown).unwrap(),
            "\"unknown\""
        );
    }

    #[test]
    fn test_license_status_display() {
        assert_eq!(format!("{}", LicenseStatus::Active), "active");
        assert_eq!(format!("{}", LicenseStatus::Inactive), "inactive");
        assert_eq!(format!("{}", LicenseStatus::Expired), "expired");
        assert_eq!(format!("{}", LicenseStatus::Unknown), "unknown");
    }

    #[test]
    fn test_license_status_is_copy() {
        // Verify Copy trait works
        let status = LicenseStatus::Active;
        let copied = status;
        assert_eq!(status, copied); // status should still be usable
    }

    // InstalledLicense tests

    #[test]
    fn test_installed_license_deserialize_with_enums() {
        let json = r#"{
            "name": "test-license",
            "type": "enterprise",
            "status": "active",
            "quota_bytes": "1073741824",
            "expiration_time": "2025-12-31T00:00:00Z",
            "features": ["feature1", "feature2"]
        }"#;

        let license: InstalledLicense = serde_json::from_str(json).unwrap();
        assert_eq!(license.name, "test-license");
        assert_eq!(license.license_type, LicenseType::Enterprise);
        assert_eq!(license.status, LicenseStatus::Active);
        assert_eq!(license.quota_bytes, 1073741824);
        assert_eq!(
            license.expiration_time,
            Some("2025-12-31T00:00:00Z".to_string())
        );
        assert_eq!(license.features, vec!["feature1", "feature2"]);
    }

    #[test]
    fn test_installed_license_deserialize_default_status() {
        let json = r#"{
            "name": "test-license",
            "type": "forwarder",
            "quota_bytes": "536870912"
        }"#;

        let license: InstalledLicense = serde_json::from_str(json).unwrap();
        assert_eq!(license.license_type, LicenseType::Forwarder);
        assert_eq!(license.status, LicenseStatus::Unknown); // default
    }

    #[test]
    fn test_installed_license_is_active() {
        let active = InstalledLicense {
            name: "active-license".to_string(),
            license_type: LicenseType::Enterprise,
            status: LicenseStatus::Active,
            quota_bytes: 1073741824,
            expiration_time: None,
            features: vec![],
        };
        assert!(active.is_active());

        let inactive = InstalledLicense {
            name: "inactive-license".to_string(),
            license_type: LicenseType::Enterprise,
            status: LicenseStatus::Inactive,
            quota_bytes: 1073741824,
            expiration_time: None,
            features: vec![],
        };
        assert!(!inactive.is_active());

        let expired = InstalledLicense {
            name: "expired-license".to_string(),
            license_type: LicenseType::Trial,
            status: LicenseStatus::Expired,
            quota_bytes: 536870912,
            expiration_time: None,
            features: vec![],
        };
        assert!(!expired.is_active());

        let unknown = InstalledLicense {
            name: "unknown-license".to_string(),
            license_type: LicenseType::Unknown,
            status: LicenseStatus::Unknown,
            quota_bytes: 0,
            expiration_time: None,
            features: vec![],
        };
        assert!(!unknown.is_active());
    }

    #[test]
    fn test_installed_license_deserialize_unknown_license_type() {
        let json = r#"{
            "name": "custom-license",
            "type": "custom_type",
            "status": "active",
            "quota_bytes": "1073741824"
        }"#;

        let license: InstalledLicense = serde_json::from_str(json).unwrap();
        assert_eq!(license.license_type, LicenseType::Unknown);
        assert_eq!(license.status, LicenseStatus::Active);
    }

    #[test]
    fn test_installed_license_deserialize_unknown_status() {
        let json = r#"{
            "name": "custom-license",
            "type": "enterprise",
            "status": "pending_validation",
            "quota_bytes": "1073741824"
        }"#;

        let license: InstalledLicense = serde_json::from_str(json).unwrap();
        assert_eq!(license.license_type, LicenseType::Enterprise);
        assert_eq!(license.status, LicenseStatus::Unknown);
    }
}
