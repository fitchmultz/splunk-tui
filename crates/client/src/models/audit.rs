//! Audit event models for Splunk audit logging API.
//!
//! This module contains types for listing and viewing Splunk audit events.
//! Audit events track user actions and are important for compliance.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Parameters for listing audit events with time range filters.
#[derive(Debug, Clone, Default)]
pub struct ListAuditEventsParams {
    /// Earliest time for events (e.g., "-24h", "2024-01-01T00:00:00")
    pub earliest: Option<String>,
    /// Latest time for events (e.g., "now", "2024-01-02T00:00:00")
    pub latest: Option<String>,
    /// Maximum number of events to return
    pub count: Option<usize>,
    /// Offset for pagination
    pub offset: Option<usize>,
    /// Filter by user
    pub user: Option<String>,
    /// Filter by action
    pub action: Option<String>,
}

/// Audit action types for Splunk audit events.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[derive(Default)]
pub enum AuditAction {
    /// User login action
    Login,
    /// User logout action
    Logout,
    /// Search execution action
    Search,
    /// User edit action
    EditUser,
    /// User creation action
    CreateUser,
    /// User deletion action
    DeleteUser,
    /// Role edit action
    EditRole,
    /// Unknown or unrecognized action
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for AuditAction {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditAction::Login => write!(f, "login"),
            AuditAction::Logout => write!(f, "logout"),
            AuditAction::Search => write!(f, "search"),
            AuditAction::EditUser => write!(f, "edit_user"),
            AuditAction::CreateUser => write!(f, "create_user"),
            AuditAction::DeleteUser => write!(f, "delete_user"),
            AuditAction::EditRole => write!(f, "edit_role"),
            AuditAction::Unknown => write!(f, "unknown"),
        }
    }
}

/// Audit result types for Splunk audit events.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
#[derive(Default)]
pub enum AuditResult {
    /// Action completed successfully
    Success,
    /// Action failed
    Failure,
    /// Unknown or unrecognized result
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for AuditResult {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AuditResult::Success => write!(f, "success"),
            AuditResult::Failure => write!(f, "failure"),
            AuditResult::Unknown => write!(f, "unknown"),
        }
    }
}

/// A single audit event entry.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AuditEvent {
    /// Timestamp of the event
    #[serde(rename = "_time")]
    pub time: String,
    /// User who performed the action
    #[serde(default)]
    pub user: String,
    /// Action performed (e.g., "login", "search", "edit_user")
    #[serde(default)]
    pub action: AuditAction,
    /// Target of the action (e.g., resource name)
    #[serde(default)]
    pub target: String,
    /// Action result (e.g., "success", "failure")
    #[serde(default)]
    pub result: AuditResult,
    /// Client IP address
    #[serde(default, rename = "client_ip")]
    pub client_ip: String,
    /// Additional details about the event
    #[serde(default)]
    pub details: String,
    /// Raw event data
    #[serde(rename = "_raw", default)]
    pub raw: String,
}

/// Audit event list response entry.
#[derive(Debug, Deserialize, Clone)]
pub struct AuditEventEntry {
    pub name: String,
    pub content: AuditEvent,
}

/// Audit event list response.
#[derive(Debug, Deserialize, Clone)]
pub struct AuditEventListResponse {
    pub entry: Vec<AuditEventEntry>,
}

impl crate::name_merge::HasName for AuditEvent {
    fn set_name(&mut self, name: String) {
        // Audit events use the target field to store the entry name
        self.target = name;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_audit_event() {
        let json = r#"{
            "_time": "2025-01-20T10:30:00.000Z",
            "user": "admin",
            "action": "login",
            "target": "splunkd",
            "result": "success",
            "client_ip": "192.168.1.1",
            "details": "Login via web",
            "_raw": "admin logged in from 192.168.1.1"
        }"#;
        let event: AuditEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.time, "2025-01-20T10:30:00.000Z");
        assert_eq!(event.user, "admin");
        assert_eq!(event.action, AuditAction::Login);
        assert_eq!(event.result, AuditResult::Success);
        assert_eq!(event.client_ip, "192.168.1.1");
        assert_eq!(event.target, "splunkd");
        assert_eq!(event.details, "Login via web");
    }

    #[test]
    fn test_deserialize_audit_event_with_missing_fields() {
        // Test that missing optional fields use defaults
        let json = r#"{
            "_time": "2025-01-20T10:30:00.000Z",
            "user": "admin",
            "action": "search"
        }"#;
        let event: AuditEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.time, "2025-01-20T10:30:00.000Z");
        assert_eq!(event.user, "admin");
        assert_eq!(event.action, AuditAction::Search);
        assert_eq!(event.target, "");
        assert_eq!(event.result, AuditResult::Unknown);
        assert_eq!(event.client_ip, "");
        assert_eq!(event.details, "");
        assert_eq!(event.raw, "");
    }

    #[test]
    fn test_list_audit_events_params_default() {
        let params = ListAuditEventsParams::default();
        assert!(params.earliest.is_none());
        assert!(params.latest.is_none());
        assert!(params.count.is_none());
        assert!(params.offset.is_none());
        assert!(params.user.is_none());
        assert!(params.action.is_none());
    }

    // AuditAction tests
    #[test]
    fn test_audit_action_deserialize_all_variants() {
        let test_cases = vec![
            ("login", AuditAction::Login),
            ("logout", AuditAction::Logout),
            ("search", AuditAction::Search),
            ("edit_user", AuditAction::EditUser),
            ("create_user", AuditAction::CreateUser),
            ("delete_user", AuditAction::DeleteUser),
            ("edit_role", AuditAction::EditRole),
        ];

        for (json_value, expected) in test_cases {
            let json = format!(r#""{}""#, json_value);
            let action: AuditAction = serde_json::from_str(&json).unwrap();
            assert_eq!(action, expected, "Failed for {}", json_value);
        }
    }

    #[test]
    fn test_audit_action_deserialize_unknown() {
        // Unknown variants should deserialize to Unknown
        let json = r#""some_unknown_action""#;
        let action: AuditAction = serde_json::from_str(json).unwrap();
        assert_eq!(action, AuditAction::Unknown);
    }

    #[test]
    fn test_audit_action_default() {
        assert_eq!(AuditAction::default(), AuditAction::Unknown);
    }

    #[test]
    fn test_audit_action_display() {
        assert_eq!(AuditAction::Login.to_string(), "login");
        assert_eq!(AuditAction::Logout.to_string(), "logout");
        assert_eq!(AuditAction::Search.to_string(), "search");
        assert_eq!(AuditAction::EditUser.to_string(), "edit_user");
        assert_eq!(AuditAction::CreateUser.to_string(), "create_user");
        assert_eq!(AuditAction::DeleteUser.to_string(), "delete_user");
        assert_eq!(AuditAction::EditRole.to_string(), "edit_role");
        assert_eq!(AuditAction::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_audit_action_serialize() {
        assert_eq!(
            serde_json::to_string(&AuditAction::Login).unwrap(),
            r#""login""#
        );
        assert_eq!(
            serde_json::to_string(&AuditAction::EditUser).unwrap(),
            r#""edit_user""#
        );
        assert_eq!(
            serde_json::to_string(&AuditAction::Unknown).unwrap(),
            r#""unknown""#
        );
    }

    #[test]
    fn test_audit_action_clone() {
        let action = AuditAction::Search;
        let cloned = action.clone();
        assert_eq!(action, cloned);
    }

    #[test]
    fn test_audit_action_equality() {
        assert_eq!(AuditAction::Login, AuditAction::Login);
        assert_ne!(AuditAction::Login, AuditAction::Logout);
        assert_eq!(AuditAction::Unknown, AuditAction::default());
    }

    // AuditResult tests
    #[test]
    fn test_audit_result_deserialize_all_variants() {
        let success: AuditResult = serde_json::from_str(r#""success""#).unwrap();
        assert_eq!(success, AuditResult::Success);

        let failure: AuditResult = serde_json::from_str(r#""failure""#).unwrap();
        assert_eq!(failure, AuditResult::Failure);
    }

    #[test]
    fn test_audit_result_deserialize_unknown() {
        // Unknown variants should deserialize to Unknown
        let json = r#""some_unknown_result""#;
        let result: AuditResult = serde_json::from_str(json).unwrap();
        assert_eq!(result, AuditResult::Unknown);
    }

    #[test]
    fn test_audit_result_default() {
        assert_eq!(AuditResult::default(), AuditResult::Unknown);
    }

    #[test]
    fn test_audit_result_display() {
        assert_eq!(AuditResult::Success.to_string(), "success");
        assert_eq!(AuditResult::Failure.to_string(), "failure");
        assert_eq!(AuditResult::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_audit_result_serialize() {
        assert_eq!(
            serde_json::to_string(&AuditResult::Success).unwrap(),
            r#""success""#
        );
        assert_eq!(
            serde_json::to_string(&AuditResult::Failure).unwrap(),
            r#""failure""#
        );
        assert_eq!(
            serde_json::to_string(&AuditResult::Unknown).unwrap(),
            r#""unknown""#
        );
    }

    #[test]
    fn test_audit_result_clone() {
        let result = AuditResult::Success;
        let cloned = result;
        assert_eq!(result, cloned);
    }

    #[test]
    fn test_audit_result_equality() {
        assert_eq!(AuditResult::Success, AuditResult::Success);
        assert_ne!(AuditResult::Success, AuditResult::Failure);
        assert_eq!(AuditResult::Unknown, AuditResult::default());
    }

    #[test]
    fn test_audit_result_is_copy() {
        // Verify Copy trait works (compile-time check)
        let result = AuditResult::Success;
        let _copied = result;
        // If result is not Copy, this would fail to compile
        let _ = result;
    }

    // Integration tests for AuditEvent with enums
    #[test]
    fn test_deserialize_audit_event_all_actions() {
        let actions = vec![
            ("login", AuditAction::Login),
            ("logout", AuditAction::Logout),
            ("search", AuditAction::Search),
            ("edit_user", AuditAction::EditUser),
            ("create_user", AuditAction::CreateUser),
            ("delete_user", AuditAction::DeleteUser),
            ("edit_role", AuditAction::EditRole),
        ];

        for (action_str, expected_action) in actions {
            let json = format!(
                r#"{{"_time": "2025-01-20T10:30:00.000Z", "action": "{}"}}"#,
                action_str
            );
            let event: AuditEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event.action, expected_action);
        }
    }

    #[test]
    fn test_deserialize_audit_event_all_results() {
        let results = vec![
            ("success", AuditResult::Success),
            ("failure", AuditResult::Failure),
        ];

        for (result_str, expected_result) in results {
            let json = format!(
                r#"{{"_time": "2025-01-20T10:30:00.000Z", "result": "{}"}}"#,
                result_str
            );
            let event: AuditEvent = serde_json::from_str(&json).unwrap();
            assert_eq!(event.result, expected_result);
        }
    }

    #[test]
    fn test_deserialize_audit_event_unknown_action_and_result() {
        let json = r#"{
            "_time": "2025-01-20T10:30:00.000Z",
            "action": "custom_action",
            "result": "partial_success"
        }"#;
        let event: AuditEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.action, AuditAction::Unknown);
        assert_eq!(event.result, AuditResult::Unknown);
    }

    #[test]
    fn test_deserialize_audit_event_failure_result() {
        let json = r#"{
            "_time": "2025-01-20T10:30:00.000Z",
            "user": "admin",
            "action": "login",
            "result": "failure"
        }"#;
        let event: AuditEvent = serde_json::from_str(json).unwrap();
        assert_eq!(event.action, AuditAction::Login);
        assert_eq!(event.result, AuditResult::Failure);
    }
}
