//! Audit event models for Splunk audit logging API.
//!
//! This module contains types for listing and viewing Splunk audit events.
//! Audit events track user actions and are important for compliance.

use serde::{Deserialize, Serialize};

/// Parameters for listing audit events with time range filters.
#[derive(Debug, Clone, Default)]
pub struct ListAuditEventsParams {
    /// Earliest time for events (e.g., "-24h", "2024-01-01T00:00:00")
    pub earliest: Option<String>,
    /// Latest time for events (e.g., "now", "2024-01-02T00:00:00")
    pub latest: Option<String>,
    /// Maximum number of events to return
    pub count: Option<u64>,
    /// Offset for pagination
    pub offset: Option<u64>,
    /// Filter by user
    pub user: Option<String>,
    /// Filter by action
    pub action: Option<String>,
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
    pub action: String,
    /// Target of the action (e.g., resource name)
    #[serde(default)]
    pub target: String,
    /// Action result (e.g., "success", "failure")
    #[serde(default)]
    pub result: String,
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
        assert_eq!(event.action, "login");
        assert_eq!(event.result, "success");
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
        assert_eq!(event.action, "search");
        assert_eq!(event.target, "");
        assert_eq!(event.result, "");
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
}
