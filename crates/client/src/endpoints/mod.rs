//! REST API endpoint implementations.

mod alerts;
mod audit;
mod auth;
mod capabilities;
mod cluster;
mod configs;
mod dashboards;
mod datamodels;
mod form_params;
mod forwarders;
pub mod hec;
mod indexes;
mod inputs;
mod jobs;
mod kvstore;
mod license;
mod logs;
mod lookups;
mod macros;
mod parsing;
mod request;
mod roles;
pub mod search;
mod search_peers;
mod server;
mod shc;
mod users;
mod workload;

// Re-export form parameter macros for use by endpoint modules
pub use crate::{form_params, form_params_str};

pub use alerts::{get_fired_alert, list_fired_alerts};
pub use audit::{get_recent_audit_events, list_audit_events};
pub use auth::login;
pub use capabilities::list_capabilities;
pub use cluster::{
    decommission_peer, get_cluster_info, get_cluster_peers, rebalance_cluster, remove_peers,
    set_maintenance_mode,
};
pub use configs::{get_config_stanza, list_config_files, list_config_stanzas};
pub use dashboards::{get_dashboard, list_dashboards};
pub use datamodels::{get_datamodel, list_datamodels};
pub use forwarders::list_forwarders;
pub use indexes::{create_index, delete_index, get_index, list_indexes, modify_index};
pub use inputs::{disable_input, enable_input, list_inputs_by_type};
pub use jobs::{cancel_job, delete_job, get_job, list_jobs};
pub use kvstore::{
    create_collection, delete_collection, delete_collection_record, get_kvstore_status,
    insert_collection_record, list_collection_records, list_collections, modify_collection,
};
pub use license::{
    activate_license, create_license_pool, deactivate_license, delete_license_pool,
    get_license_usage, install_license, list_installed_licenses, list_license_pools,
    list_license_stacks, modify_license_pool,
};
pub use logs::get_internal_logs;
pub use lookups::{
    delete_lookup_table, download_lookup_table, list_lookup_tables, upload_lookup_table,
};
pub use macros::{
    CreateMacroRequest, UpdateMacroRequest, create_macro, delete_macro, get_macro, list_macros,
    update_macro,
};
pub use parsing::check_log_parsing_health;
pub use request::send_request_with_retry;
pub use roles::{create_role, delete_role, list_roles, modify_role};
pub use search::{
    CreateJobOptions, OutputMode, SavedSearchUpdateParams, create_job, create_saved_search,
    delete_saved_search, get_job_status, get_results, get_saved_search, list_saved_searches,
    update_saved_search, wait_for_job, wait_for_job_with_progress,
};
pub use search_peers::list_search_peers;
pub use server::*;
pub use shc::{
    add_shc_member, get_shc_captain, get_shc_config, get_shc_members, get_shc_status,
    remove_shc_member, rolling_restart_shc, set_shc_captain,
};
pub use users::{create_user, delete_user, list_users, modify_user};
pub use workload::{list_workload_pools, list_workload_rules};

use crate::error::ClientError;

/// Safely extract the first entry's content from a Splunk API response.
///
/// Splunk REST API responses typically have the structure:
/// `{ "entry": [ { "content": { ... } } ] }`
///
/// This helper safely navigates that structure and returns an error if any
/// part is missing (empty entry array, missing entry field, missing content field).
pub(crate) fn extract_entry_content(
    resp: &serde_json::Value,
) -> Result<&serde_json::Value, ClientError> {
    let entry = resp
        .get("entry")
        .and_then(|e| e.as_array())
        .and_then(|arr| arr.first())
        .ok_or_else(|| {
            ClientError::InvalidResponse("Missing or empty 'entry' array in response".to_string())
        })?;

    entry
        .get("content")
        .ok_or_else(|| ClientError::InvalidResponse("Missing 'content' field in entry".to_string()))
}

/// Safely extract a message from the first entry's content.
///
/// Used for management API responses that return a message in the content.
pub(crate) fn extract_entry_message(resp: &serde_json::Value) -> Option<String> {
    resp.get("entry")?
        .as_array()?
        .first()?
        .get("content")?
        .get("message")?
        .as_str()
        .map(|s| s.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_entry_content_success() {
        let resp = json!({
            "entry": [{
                "content": {
                    "id": "test-123",
                    "name": "test"
                }
            }]
        });

        let content = extract_entry_content(&resp).unwrap();
        assert_eq!(content["id"], "test-123");
    }

    #[test]
    fn test_extract_entry_content_missing_entry() {
        let resp = json!({ "other": "field" });

        let result = extract_entry_content(&resp);
        assert!(matches!(result, Err(ClientError::InvalidResponse(_))));
    }

    #[test]
    fn test_extract_entry_content_empty_entry_array() {
        let resp = json!({ "entry": [] });

        let result = extract_entry_content(&resp);
        assert!(matches!(result, Err(ClientError::InvalidResponse(_))));
    }

    #[test]
    fn test_extract_entry_content_missing_content() {
        let resp = json!({
            "entry": [{ "name": "test" }]
        });

        let result = extract_entry_content(&resp);
        assert!(matches!(result, Err(ClientError::InvalidResponse(_))));
    }

    #[test]
    fn test_extract_entry_message_success() {
        let resp = json!({
            "entry": [{
                "content": {
                    "message": "Operation successful"
                }
            }]
        });

        assert_eq!(
            extract_entry_message(&resp),
            Some("Operation successful".to_string())
        );
    }

    #[test]
    fn test_extract_entry_message_missing() {
        let resp = json!({
            "entry": [{
                "content": { "other": "field" }
            }]
        });

        assert_eq!(extract_entry_message(&resp), None);
    }

    #[test]
    fn test_extract_entry_message_empty_entry() {
        let resp = json!({ "entry": [] });
        assert_eq!(extract_entry_message(&resp), None);
    }
}
