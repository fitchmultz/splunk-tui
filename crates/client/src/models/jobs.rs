//! Search job models for Splunk search API.
//!
//! This module contains types for creating, monitoring, and retrieving
//! results from Splunk search jobs.
//!
//! # What this module handles:
//! - Search job status and results
//! - SPL validation request/response types
//!
//! # What this module does NOT handle:
//! - Search execution logic (see [`crate::client::search`])
//! - HTTP transport (see [`crate::endpoints::search`])

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
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub scanCount: usize,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub eventCount: usize,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub resultCount: usize,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::opt_usize_from_string_or_number"
    )]
    pub statusBuckets: Option<usize>,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub diskUsage: usize,
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
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub scanCount: usize,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub eventCount: usize,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub resultCount: usize,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub diskUsage: usize,
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
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub scan_count: usize,
    #[serde(
        rename = "eventCount",
        default,
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub event_count: usize,
    #[serde(
        rename = "resultCount",
        default,
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub result_count: usize,
    #[serde(
        rename = "diskUsage",
        default,
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub disk_usage: usize,
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
        deserialize_with = "crate::serde_helpers::opt_usize_from_string_or_number"
    )]
    pub offset: Option<usize>,
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::opt_usize_from_string_or_number"
    )]
    pub total: Option<usize>,
}

/// SPL validation request.
///
/// Sent to the Splunk search parser endpoint to validate SPL syntax
/// without executing the search.
#[derive(Debug, Serialize, Clone)]
pub struct ValidateSplRequest {
    /// The SPL query to validate
    pub search: String,
}

/// SPL validation response.
///
/// Contains the result of parsing the SPL query, including any errors
/// or warnings detected by Splunk's parser.
#[derive(Debug, Deserialize, Clone)]
pub struct ValidateSplResponse {
    /// Whether the SPL is valid (no errors)
    pub valid: bool,
    /// List of syntax errors found
    #[serde(default)]
    pub errors: Vec<SplError>,
    /// List of warnings found
    #[serde(default)]
    pub warnings: Vec<SplWarning>,
}

/// SPL syntax error.
#[derive(Debug, Deserialize, Clone)]
pub struct SplError {
    /// Error message describing the problem
    pub message: String,
    /// Line number where the error occurred (if available)
    pub line: Option<u32>,
    /// Column number where the error occurred (if available)
    pub column: Option<u32>,
}

/// SPL syntax warning.
#[derive(Debug, Deserialize, Clone)]
pub struct SplWarning {
    /// Warning message
    pub message: String,
    /// Line number where the warning occurred (if available)
    pub line: Option<u32>,
    /// Column number where the warning occurred (if available)
    pub column: Option<u32>,
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

    #[test]
    fn test_validate_spl_request_serialization() {
        let request = ValidateSplRequest {
            search: "search index=main | stats count".to_string(),
        };
        let json = serde_json::to_string(&request).unwrap();
        assert!(json.contains("search index=main | stats count"));
    }

    #[test]
    fn test_validate_spl_response_deserialization_valid() {
        let json = r#"{
            "valid": true,
            "errors": [],
            "warnings": []
        }"#;
        let response: ValidateSplResponse = serde_json::from_str(json).unwrap();
        assert!(response.valid);
        assert!(response.errors.is_empty());
        assert!(response.warnings.is_empty());
    }

    #[test]
    fn test_validate_spl_response_deserialization_with_errors() {
        let json = r#"{
            "valid": false,
            "errors": [
                {"message": "Syntax error at position 10", "line": 1, "column": 10}
            ],
            "warnings": []
        }"#;
        let response: ValidateSplResponse = serde_json::from_str(json).unwrap();
        assert!(!response.valid);
        assert_eq!(response.errors.len(), 1);
        assert_eq!(response.errors[0].message, "Syntax error at position 10");
        assert_eq!(response.errors[0].line, Some(1));
        assert_eq!(response.errors[0].column, Some(10));
    }

    #[test]
    fn test_validate_spl_response_deserialization_with_warnings() {
        let json = r#"{
            "valid": true,
            "errors": [],
            "warnings": [
                {"message": "Deprecated command usage", "line": 2, "column": 5}
            ]
        }"#;
        let response: ValidateSplResponse = serde_json::from_str(json).unwrap();
        assert!(response.valid);
        assert_eq!(response.warnings.len(), 1);
        assert_eq!(response.warnings[0].message, "Deprecated command usage");
        assert_eq!(response.warnings[0].line, Some(2));
        assert_eq!(response.warnings[0].column, Some(5));
    }

    #[test]
    fn test_spl_error_without_line_column() {
        let json = r#"{"message": "General error"}"#;
        let error: SplError = serde_json::from_str(json).unwrap();
        assert_eq!(error.message, "General error");
        assert_eq!(error.line, None);
        assert_eq!(error.column, None);
    }
}
