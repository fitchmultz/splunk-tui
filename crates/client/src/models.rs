//! Data models for Splunk API responses.

use serde::{Deserialize, Serialize};

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

/// Cluster information.
#[derive(Debug, Deserialize, Clone)]
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
#[derive(Debug, Deserialize, Clone)]
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
}
