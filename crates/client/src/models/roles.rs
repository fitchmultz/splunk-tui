//! Role models for Splunk role management API.
//!
//! This module contains types for listing and managing Splunk roles.

use serde::{Deserialize, Serialize};

/// Splunk role information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Role {
    #[serde(default)]
    pub name: String,
    /// Capabilities assigned to this role
    #[serde(default, rename = "capabilities")]
    pub capabilities: Vec<String>,
    /// Search indexes this role has access to
    #[serde(default, rename = "searchIndexes")]
    pub search_indexes: Vec<String>,
    /// Search filter (restricts search results)
    #[serde(default, rename = "searchFilter")]
    pub search_filter: Option<String>,
    /// Roles imported by this role (inherits capabilities)
    #[serde(default, rename = "importedRoles")]
    pub imported_roles: Vec<String>,
    /// Default app for this role
    #[serde(default, rename = "defaultApp")]
    pub default_app: Option<String>,
    /// Cumulative search jobs quota
    #[serde(
        default,
        rename = "cumulativeSrchJobsQuota",
        deserialize_with = "crate::serde_helpers::opt_i32_from_string_or_number"
    )]
    pub cumulative_srch_jobs_quota: Option<i32>,
    /// Cumulative real-time search jobs quota
    #[serde(
        default,
        rename = "cumulativeRTSrchJobsQuota",
        deserialize_with = "crate::serde_helpers::opt_i32_from_string_or_number"
    )]
    pub cumulative_rt_srch_jobs_quota: Option<i32>,
}

/// Role entry wrapper.
#[derive(Debug, Deserialize, Clone)]
pub struct RoleEntry {
    pub name: String,
    pub content: Role,
}

/// Role list response.
#[derive(Debug, Deserialize, Clone)]
pub struct RoleListResponse {
    pub entry: Vec<RoleEntry>,
}

/// Parameters for creating a new role.
#[derive(Debug, Clone, Default)]
pub struct CreateRoleParams {
    /// The role name (required).
    pub name: String,
    /// Capabilities to assign to the role.
    pub capabilities: Vec<String>,
    /// Search indexes the role can access.
    pub search_indexes: Vec<String>,
    /// Search filter to restrict results.
    pub search_filter: Option<String>,
    /// Roles to import (inherit capabilities from).
    pub imported_roles: Vec<String>,
    /// Default app for the role.
    pub default_app: Option<String>,
}

/// Parameters for modifying an existing role.
#[derive(Debug, Clone, Default)]
pub struct ModifyRoleParams {
    /// Capabilities to assign to the role (replaces existing).
    pub capabilities: Option<Vec<String>>,
    /// Search indexes the role can access (replaces existing).
    pub search_indexes: Option<Vec<String>>,
    /// Search filter to restrict results.
    pub search_filter: Option<String>,
    /// Roles to import (replaces existing).
    pub imported_roles: Option<Vec<String>>,
    /// Default app for the role.
    pub default_app: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_role() {
        let json = r#"{
            "name": "admin",
            "capabilities": ["admin_all_objects", "edit_users", "edit_roles"],
            "searchIndexes": ["*", "_audit", "_internal"],
            "searchFilter": "",
            "importedRoles": [],
            "defaultApp": "search",
            "cumulativeSrchJobsQuota": 100,
            "cumulativeRTSrchJobsQuota": 50
        }"#;
        let role: Role = serde_json::from_str(json).unwrap();
        assert_eq!(role.name, "admin");
        assert_eq!(
            role.capabilities,
            vec!["admin_all_objects", "edit_users", "edit_roles"]
        );
        assert_eq!(role.search_indexes, vec!["*", "_audit", "_internal"]);
        assert_eq!(role.search_filter, Some("".to_string()));
        assert!(role.imported_roles.is_empty());
        assert_eq!(role.default_app, Some("search".to_string()));
        assert_eq!(role.cumulative_srch_jobs_quota, Some(100));
        assert_eq!(role.cumulative_rt_srch_jobs_quota, Some(50));
    }

    #[test]
    fn test_deserialize_role_with_optional_fields_missing() {
        let json = r#"{
            "name": "minimal_role",
            "capabilities": []
        }"#;
        let role: Role = serde_json::from_str(json).unwrap();
        assert_eq!(role.name, "minimal_role");
        assert!(role.capabilities.is_empty());
        assert!(role.search_indexes.is_empty());
        assert_eq!(role.search_filter, None);
        assert!(role.imported_roles.is_empty());
        assert_eq!(role.default_app, None);
        assert_eq!(role.cumulative_srch_jobs_quota, None);
        assert_eq!(role.cumulative_rt_srch_jobs_quota, None);
    }

    #[test]
    fn test_deserialize_role_with_imported_roles() {
        let json = r#"{
            "name": "custom_power",
            "capabilities": ["search"],
            "importedRoles": ["power", "user"]
        }"#;
        let role: Role = serde_json::from_str(json).unwrap();
        assert_eq!(role.name, "custom_power");
        assert_eq!(role.imported_roles, vec!["power", "user"]);
    }
}
