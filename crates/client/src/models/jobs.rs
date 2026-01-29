//! Search job models for Splunk search API.
//!
//! This module contains types for creating, monitoring, and retrieving
//! results from Splunk search jobs.

use serde::{Deserialize, Serialize};

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
    #[serde(default)]
    pub runDuration: f64,
    pub cursorTime: Option<String>,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::u64_from_string_or_number"
    )]
    pub scanCount: u64,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::u64_from_string_or_number"
    )]
    pub eventCount: u64,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::u64_from_string_or_number"
    )]
    pub resultCount: u64,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::opt_u64_from_string_or_number"
    )]
    pub statusBuckets: Option<u64>,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::u64_from_string_or_number"
    )]
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
    #[serde(default)]
    pub runDuration: f64,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::u64_from_string_or_number"
    )]
    pub scanCount: u64,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::u64_from_string_or_number"
    )]
    pub eventCount: u64,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::u64_from_string_or_number"
    )]
    pub resultCount: u64,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::u64_from_string_or_number"
    )]
    pub diskUsage: u64,
    #[serde(default)]
    pub priority: Option<i32>,
    #[serde(default)]
    pub label: Option<String>,
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
    #[serde(rename = "runDuration", default)]
    pub run_duration: f64,
    #[serde(rename = "cursorTime")]
    pub cursor_time: Option<String>,
    #[serde(
        rename = "scanCount",
        default,
        deserialize_with = "crate::serde_helpers::u64_from_string_or_number"
    )]
    pub scan_count: u64,
    #[serde(
        rename = "eventCount",
        default,
        deserialize_with = "crate::serde_helpers::u64_from_string_or_number"
    )]
    pub event_count: u64,
    #[serde(
        rename = "resultCount",
        default,
        deserialize_with = "crate::serde_helpers::u64_from_string_or_number"
    )]
    pub result_count: u64,
    #[serde(
        rename = "diskUsage",
        default,
        deserialize_with = "crate::serde_helpers::u64_from_string_or_number"
    )]
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
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::opt_u64_from_string_or_number"
    )]
    pub offset: Option<u64>,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::opt_u64_from_string_or_number"
    )]
    pub total: Option<u64>,
}

#[cfg(test)]
mod tests {
    use super::*;

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
    fn test_deserialize_job_content_with_optional_counts_missing() {
        let json = r#"{
            "sid": "sid123",
            "isDone": false,
            "isFinalized": false,
            "doneProgress": 0.0
        }"#;
        let content: JobContent = serde_json::from_str(json).unwrap();
        assert_eq!(content.sid, "sid123");
        assert_eq!(content.scanCount, 0);
        assert_eq!(content.eventCount, 0);
        assert_eq!(content.resultCount, 0);
    }
}
