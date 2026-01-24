//! Data models for Splunk API responses.

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Generic Splunk REST API response wrapper.
#[derive(Debug, Deserialize, Clone)]
pub struct SplunkResponse<T> {
    pub entry: Vec<Entry<T>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Entry<T> {
    pub name: String,
    pub content: T,
    pub acl: Option<Acl>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Acl {
    pub app: String,
    pub owner: String,
    pub perms: Option<Perms>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Perms {
    pub read: Vec<String>,
    pub write: Vec<String>,
}

/// A single message from Splunk (usually in error responses).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SplunkMessage {
    #[serde(rename = "type")]
    pub message_type: String,
    pub text: String,
}

/// A collection of messages from Splunk.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SplunkMessages {
    pub messages: Vec<SplunkMessage>,
}

/// Search job status information.
#[derive(Debug, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct SearchJob {
    pub sid: String,
    #[serde(default)]
    pub isDone: bool,
    #[serde(default)]
    pub isFinalized: bool,
    pub doneProgress: f64,
    pub runDuration: f64,
    pub cursorTime: Option<String>,
    pub scanCount: u64,
    pub eventCount: u64,
    pub resultCount: u64,
    pub statusBuckets: Option<u64>,
    pub diskUsage: u64,
}

/// Search job list response.
#[derive(Debug, Deserialize, Clone)]
pub struct SearchJobListResponse {
    pub entry: Vec<JobEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct JobEntry {
    pub name: String,
    pub content: JobContent,
}

#[derive(Debug, Deserialize, Clone)]
#[allow(non_snake_case)]
pub struct JobContent {
    pub sid: String,
    #[serde(rename = "isDone", default)]
    pub is_done: bool,
    #[serde(rename = "isFinalized", default)]
    pub is_finalized: bool,
    #[serde(rename = "doneProgress", default)]
    pub done_progress: f64,
    pub runDuration: f64,
    pub scanCount: u64,
    pub eventCount: u64,
    pub resultCount: u64,
}

/// Search job status (detailed).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SearchJobStatus {
    pub sid: String,
    #[serde(default, rename = "isDone")]
    pub is_done: bool,
    #[serde(default, rename = "isFinalized")]
    pub is_finalized: bool,
    #[serde(rename = "doneProgress", default)]
    pub done_progress: f64,
    #[serde(rename = "runDuration")]
    pub run_duration: f64,
    #[serde(rename = "cursorTime")]
    pub cursor_time: Option<String>,
    #[serde(rename = "scanCount")]
    pub scan_count: u64,
    #[serde(rename = "eventCount")]
    pub event_count: u64,
    #[serde(rename = "resultCount")]
    pub result_count: u64,
    #[serde(rename = "diskUsage")]
    pub disk_usage: u64,
    #[serde(rename = "priority")]
    pub priority: Option<i32>,
    pub label: Option<String>,
}

/// Search job results.
#[derive(Debug, Deserialize, Clone)]
pub struct SearchJobResults {
    pub results: Vec<serde_json::Value>,
    #[serde(default)]
    pub preview: bool,
    pub offset: Option<u64>,
    pub total: Option<u64>,
}

/// Index information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Index {
    pub name: String,
    #[serde(rename = "maxTotalDataSizeMB")]
    pub max_total_data_size_mb: Option<u64>,
    #[serde(rename = "currentDBSizeMB")]
    pub current_db_size_mb: u64,
    #[serde(rename = "totalEventCount")]
    pub total_event_count: u64,
    #[serde(rename = "maxWarmDBCount")]
    pub max_warm_db_count: Option<u64>,
    #[serde(rename = "maxHotBuckets")]
    pub max_hot_buckets: Option<u64>,
    #[serde(rename = "frozenTimePeriodInSecs")]
    pub frozen_time_period_in_secs: Option<u64>,
    #[serde(rename = "coldDBPath")]
    pub cold_db_path: Option<String>,
    #[serde(rename = "homePath")]
    pub home_path: Option<String>,
    #[serde(rename = "thawedPath")]
    pub thawed_path: Option<String>,
    #[serde(rename = "coldToFrozenDir")]
    pub cold_to_frozen_dir: Option<String>,
    #[serde(rename = "primaryIndex")]
    pub primary_index: Option<bool>,
}

/// Index list response.
#[derive(Debug, Deserialize, Clone)]
pub struct IndexListResponse {
    pub entry: Vec<IndexEntry>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct IndexEntry {
    pub name: String,
    pub content: Index,
}

/// Saved search information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedSearch {
    #[serde(default)]
    pub name: String,
    pub search: String,
    pub description: Option<String>,
    #[serde(default)]
    pub disabled: bool,
}

/// Saved search entry.
#[derive(Debug, Deserialize, Clone)]
pub struct SavedSearchEntry {
    pub name: String,
    pub content: SavedSearch,
}

/// Saved search list response.
#[derive(Debug, Deserialize, Clone)]
pub struct SavedSearchListResponse {
    pub entry: Vec<SavedSearchEntry>,
}

/// Splunk app information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct App {
    #[serde(default)]
    pub name: String,
    pub label: Option<String>,
    pub version: Option<String>,
    #[serde(rename = "is_configured")]
    pub is_configured: Option<bool>,
    #[serde(rename = "is_visible")]
    pub is_visible: Option<bool>,
    #[serde(default)]
    pub disabled: bool,
    pub description: Option<String>,
    pub author: Option<String>,
}

/// App entry wrapper.
#[derive(Debug, Deserialize, Clone)]
pub struct AppEntry {
    pub name: String,
    pub content: App,
}

/// App list response.
#[derive(Debug, Deserialize, Clone)]
pub struct AppListResponse {
    pub entry: Vec<AppEntry>,
}

/// Splunk user information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct User {
    #[serde(default)]
    pub name: String,
    pub realname: Option<String>,
    pub email: Option<String>,
    #[serde(rename = "type")]
    pub user_type: Option<String>, // e.g., "Splunk", "SSO", etc.
    #[serde(rename = "defaultApp")]
    pub default_app: Option<String>,
    #[serde(default)]
    pub roles: Vec<String>,
    #[serde(rename = "lastSuccessfulLogin")]
    pub last_successful_login: Option<u64>, // Unix timestamp
}

/// User entry wrapper.
#[derive(Debug, Deserialize, Clone)]
pub struct UserEntry {
    pub name: String,
    pub content: User,
}

/// User list response.
#[derive(Debug, Deserialize, Clone)]
pub struct UserListResponse {
    pub entry: Vec<UserEntry>,
}

/// Cluster information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClusterInfo {
    pub id: String,
    pub label: Option<String>,
    pub mode: String,
    pub manager_uri: Option<String>,
    pub replication_factor: Option<u32>,
    pub search_factor: Option<u32>,
    pub status: Option<String>,
}

/// Cluster peer information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ClusterPeer {
    pub id: String,
    pub label: Option<String>,
    pub status: String,
    pub peer_state: String,
    pub site: Option<String>,
    pub guid: String,
    pub host: String,
    pub port: u32,
    pub replication_count: Option<u32>,
    pub replication_status: Option<String>,
    pub bundle_replication_count: Option<u32>,
    #[serde(rename = "is_captain")]
    pub is_captain: Option<bool>,
}

/// Authentication response from login.
#[derive(Debug, Deserialize, Clone)]
#[allow(dead_code)]
pub struct AuthResponse {
    #[serde(rename = "sessionKey")]
    pub session_key: String,
}

/// Server information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ServerInfo {
    #[serde(rename = "serverName")]
    pub server_name: String,
    pub version: String,
    pub build: String,
    pub mode: Option<String>,
    #[serde(rename = "server_roles", default)]
    pub server_roles: Vec<String>,
    #[serde(rename = "os_name")]
    pub os_name: Option<String>,
}

/// Health feature information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct HealthFeature {
    pub health: String,
    pub status: String,
    pub disabled: i32,
    pub reasons: Vec<String>,
}

/// System-wide health information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SplunkHealth {
    pub health: String,
    pub features: HashMap<String, HealthFeature>,
}

/// License usage information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LicenseUsage {
    #[serde(default)]
    pub name: String,
    pub quota: u64,
    pub used_bytes: u64,
    pub slaves_usage_bytes: Option<HashMap<String, u64>>,
    pub stack_id: Option<String>,
}

/// License pool information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LicensePool {
    #[serde(default)]
    pub name: String,
    pub quota: u64,
    pub used_bytes: u64,
    pub stack_id: String,
    pub description: Option<String>,
}

/// License stack information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct LicenseStack {
    #[serde(default)]
    pub name: String,
    pub quota: u64,
    #[serde(rename = "type")]
    pub type_name: String,
    pub label: String,
}

/// KVStore member information.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct KvStoreMember {
    pub guid: String,
    pub host: String,
    pub port: u32,
    #[serde(rename = "replicaSet")]
    pub replica_set: String,
    pub status: String,
}

/// KVStore replication status.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct KvStoreReplicationStatus {
    #[serde(rename = "oplogSize")]
    pub oplog_size: u64,
    #[serde(rename = "oplogUsed")]
    pub oplog_used: f64,
}

/// KVStore status information.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct KvStoreStatus {
    #[serde(rename = "currentMember")]
    pub current_member: KvStoreMember,
    #[serde(rename = "replicationStatus")]
    pub replication_status: KvStoreReplicationStatus,
}

/// A single log parsing error entry.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct LogParsingError {
    #[serde(rename = "_time")]
    pub time: String,
    pub source: String,
    pub sourcetype: String,
    pub message: String,
    #[serde(default)]
    pub log_level: String,
    #[serde(default)]
    pub component: String,
}

/// A generic internal log entry.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct LogEntry {
    #[serde(rename = "_time")]
    pub time: String,
    #[serde(rename = "_indextime", default)]
    pub index_time: String,
    #[serde(rename = "_serial", default)]
    pub serial: Option<u64>,
    #[serde(rename = "log_level", default)]
    pub level: String,
    #[serde(default)]
    pub component: String,
    #[serde(rename = "_raw")]
    pub message: String,
}

impl LogEntry {
    /// Returns a cursor key combining time, index_time, and serial for uniqueness.
    pub fn cursor_key(&self) -> (&str, &str, Option<u64>) {
        (&self.time, &self.index_time, self.serial)
    }
}

/// Health check result for log parsing errors.
#[derive(Debug, Deserialize, Clone, Serialize)]
pub struct LogParsingHealth {
    pub is_healthy: bool,
    pub total_errors: usize,
    pub errors: Vec<LogParsingError>,
    pub time_window: String,
}

/// Aggregated health check output.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HealthCheckOutput {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub server_info: Option<ServerInfo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub splunkd_health: Option<SplunkHealth>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub license_usage: Option<Vec<LicenseUsage>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub kvstore_status: Option<KvStoreStatus>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub log_parsing_health: Option<LogParsingHealth>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_auth_response() {
        let json = r#"{"sessionKey": "test-session-key"}"#;
        let resp: AuthResponse = serde_json::from_str(json).unwrap();
        assert_eq!(resp.session_key, "test-session-key");
    }

    #[test]
    fn test_deserialize_search_job_status() {
        let json = r#"{
            "sid": "test-sid",
            "isDone": true,
            "doneProgress": 1.0,
            "runDuration": 0.0,
            "scanCount": 0,
            "eventCount": 0,
            "resultCount": 100,
            "diskUsage": 0
        }"#;
        let status: SearchJobStatus = serde_json::from_str(json).unwrap();
        assert_eq!(status.sid, "test-sid");
        assert!(status.is_done);
        assert_eq!(status.result_count, 100);
    }

    #[test]
    fn test_deserialize_log_parsing_error() {
        let json = r#"{
            "_time": "2025-01-20T10:30:00.000Z",
            "source": "/opt/splunk/var/log/splunk/metrics.log",
            "sourcetype": "splunkd",
            "message": "Failed to parse timestamp",
            "log_level": "ERROR",
            "component": "DateParserVerbose"
        }"#;
        let error: LogParsingError = serde_json::from_str(json).unwrap();
        assert_eq!(error.time, "2025-01-20T10:30:00.000Z");
        assert_eq!(error.source, "/opt/splunk/var/log/splunk/metrics.log");
        assert_eq!(error.sourcetype, "splunkd");
        assert_eq!(error.message, "Failed to parse timestamp");
        assert_eq!(error.log_level, "ERROR");
        assert_eq!(error.component, "DateParserVerbose");
    }

    #[test]
    fn test_deserialize_log_parsing_health() {
        let json = r#"{
            "is_healthy": false,
            "total_errors": 5,
            "errors": [],
            "time_window": "-24h"
        }"#;
        let health: LogParsingHealth = serde_json::from_str(json).unwrap();
        assert!(!health.is_healthy);
        assert_eq!(health.total_errors, 5);
        assert_eq!(health.time_window, "-24h");
        assert!(health.errors.is_empty());
    }

    #[test]
    fn test_deserialize_splunk_messages() {
        let json = r#"{
            "messages": [
                {
                    "type": "ERROR",
                    "text": "Invalid username or password"
                }
            ]
        }"#;
        let msgs: SplunkMessages = serde_json::from_str(json).unwrap();
        assert_eq!(msgs.messages.len(), 1);
        assert_eq!(msgs.messages[0].message_type, "ERROR");
        assert_eq!(msgs.messages[0].text, "Invalid username or password");
    }

    #[test]
    fn test_deserialize_log_entry() {
        let json = r#"{
            "_time": "2025-01-20T10:30:00.000Z",
            "log_level": "INFO",
            "component": "Metrics",
            "_raw": "some log message"
        }"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.time, "2025-01-20T10:30:00.000Z");
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.component, "Metrics");
        assert_eq!(entry.message, "some log message");
        // index_time defaults to empty string when missing
        assert_eq!(entry.index_time, "");
        assert_eq!(entry.serial, None);
    }

    #[test]
    fn test_deserialize_log_entry_with_cursor_fields() {
        let json = r#"{
            "_time": "2025-01-20T10:30:00.000Z",
            "_indextime": "2025-01-20T10:30:01.000Z",
            "_serial": 42,
            "log_level": "INFO",
            "component": "Metrics",
            "_raw": "some log message"
        }"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.time, "2025-01-20T10:30:00.000Z");
        assert_eq!(entry.index_time, "2025-01-20T10:30:01.000Z");
        assert_eq!(entry.serial, Some(42));
        assert_eq!(entry.level, "INFO");
        assert_eq!(entry.component, "Metrics");
        assert_eq!(entry.message, "some log message");
        assert_eq!(
            entry.cursor_key(),
            (
                "2025-01-20T10:30:00.000Z",
                "2025-01-20T10:30:01.000Z",
                Some(42)
            )
        );
    }

    #[test]
    fn test_deserialize_app() {
        let json = r#"{
            "name": "Splunk_TA_example",
            "label": "Example Add-on",
            "version": "1.2.3",
            "is_configured": true,
            "is_visible": true,
            "disabled": false,
            "description": "An example Splunk app",
            "author": "Splunk"
        }"#;
        let app: App = serde_json::from_str(json).unwrap();
        assert_eq!(app.name, "Splunk_TA_example");
        assert_eq!(app.label, Some("Example Add-on".to_string()));
        assert_eq!(app.version, Some("1.2.3".to_string()));
        assert_eq!(app.is_configured, Some(true));
        assert_eq!(app.is_visible, Some(true));
        assert!(!app.disabled);
        assert_eq!(app.description, Some("An example Splunk app".to_string()));
        assert_eq!(app.author, Some("Splunk".to_string()));
    }

    #[test]
    fn test_deserialize_app_with_optional_fields_missing() {
        let json = r#"{
            "name": "minimal_app",
            "disabled": true
        }"#;
        let app: App = serde_json::from_str(json).unwrap();
        assert_eq!(app.name, "minimal_app");
        assert_eq!(app.label, None);
        assert_eq!(app.version, None);
        assert_eq!(app.is_configured, None);
        assert_eq!(app.is_visible, None);
        assert!(app.disabled);
        assert_eq!(app.description, None);
        assert_eq!(app.author, None);
    }

    #[test]
    fn test_deserialize_user() {
        let json = r#"{
            "name": "admin",
            "realname": "Administrator",
            "email": "admin@example.com",
            "type": "Splunk",
            "defaultApp": "search",
            "roles": ["admin", "power"],
            "lastSuccessfulLogin": 1737712345
        }"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.name, "admin");
        assert_eq!(user.realname, Some("Administrator".to_string()));
        assert_eq!(user.email, Some("admin@example.com".to_string()));
        assert_eq!(user.user_type, Some("Splunk".to_string()));
        assert_eq!(user.default_app, Some("search".to_string()));
        assert_eq!(user.roles, vec!["admin", "power"]);
        assert_eq!(user.last_successful_login, Some(1737712345));
    }

    #[test]
    fn test_deserialize_user_with_optional_fields_missing() {
        let json = r#"{
            "name": "minimal_user",
            "roles": []
        }"#;
        let user: User = serde_json::from_str(json).unwrap();
        assert_eq!(user.name, "minimal_user");
        assert_eq!(user.realname, None);
        assert_eq!(user.email, None);
        assert_eq!(user.user_type, None);
        assert_eq!(user.default_app, None);
        assert!(user.roles.is_empty());
        assert_eq!(user.last_successful_login, None);
    }
}
