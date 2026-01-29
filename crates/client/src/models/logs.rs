//! Log and health check models for Splunk internal logs.
//!
//! This module contains types for log parsing errors, internal log entries,
//! and health check aggregation.

use serde::{Deserialize, Serialize};

use crate::models::{
    kvstore::KvStoreStatus,
    license::LicenseUsage,
    server::{ServerInfo, SplunkHealth},
};

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
    #[serde(
        rename = "_serial",
        default,
        deserialize_with = "crate::serde_helpers::opt_u64_from_string_or_number"
    )]
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
}
