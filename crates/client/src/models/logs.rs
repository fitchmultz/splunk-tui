//! Log and health check models for Splunk internal logs.
//!
//! This module contains types for log parsing errors, internal log entries,
//! and health check aggregation.

use serde::{Deserialize, Serialize};
use std::fmt;

use crate::models::{
    kvstore::KvStoreStatus,
    license::LicenseUsage,
    server::{ServerInfo, SplunkHealth},
};

/// Log level for Splunk internal logs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "UPPERCASE")]
#[derive(Default)]
pub enum LogLevel {
    /// Error level.
    Error,
    /// Warning level.
    Warn,
    /// Info level.
    Info,
    /// Debug level.
    Debug,
    /// Fatal level.
    Fatal,
    /// Unknown level for fallback.
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for LogLevel {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LogLevel::Error => write!(f, "ERROR"),
            LogLevel::Warn => write!(f, "WARN"),
            LogLevel::Info => write!(f, "INFO"),
            LogLevel::Debug => write!(f, "DEBUG"),
            LogLevel::Fatal => write!(f, "FATAL"),
            LogLevel::Unknown => write!(f, "UNKNOWN"),
        }
    }
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
    pub log_level: LogLevel,
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
        deserialize_with = "crate::serde_helpers::opt_usize_from_string_or_number"
    )]
    pub serial: Option<usize>,
    #[serde(rename = "log_level", default)]
    pub level: LogLevel,
    #[serde(default)]
    pub component: String,
    #[serde(rename = "_raw")]
    pub message: String,
}

impl LogEntry {
    /// Returns a cursor key combining time, index_time, and serial for uniqueness.
    pub fn cursor_key(&self) -> (&str, &str, Option<usize>) {
        (&self.time, &self.index_time, self.serial)
    }

    /// Compares two log entries for sorting (newest first).
    ///
    /// Returns `Ordering::Less` if `self` is newer than `other` (should come first),
    /// `Ordering::Greater` if `self` is older (should come later),
    /// and `Ordering::Equal` if they are equivalent.
    ///
    /// Sort order: time DESC, index_time DESC, serial DESC.
    /// Empty index_time sorts after non-empty (treated as older).
    /// None serial sorts after Some (treated as older).
    pub fn cmp_newest_first(&self, other: &Self) -> std::cmp::Ordering {
        // Compare time descending (newer times first)
        match other.time.cmp(&self.time) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        // Compare index_time descending
        match other.index_time.cmp(&self.index_time) {
            std::cmp::Ordering::Equal => {}
            ord => return ord,
        }
        // Compare serial descending (None sorts after Some)
        other.serial.cmp(&self.serial)
    }

    /// Returns a content-based hash for cursor comparison when serial is missing.
    /// Uses time + index_time + message to create a stable identifier.
    pub fn content_hash(&self) -> usize {
        use std::collections::hash_map::DefaultHasher;
        use std::hash::{Hash, Hasher};

        let mut hasher = DefaultHasher::new();
        self.time.hash(&mut hasher);
        self.index_time.hash(&mut hasher);
        self.message.hash(&mut hasher);
        hasher.finish() as usize
    }
}

/// Sorts logs by time, index_time, and serial in descending order (newest first).
///
/// This ensures deterministic ordering regardless of API response order.
/// Sort order matches the Splunk API: time DESC, index_time DESC, serial DESC.
///
/// # Example
///
/// ```
/// use splunk_client::models::{LogEntry, LogLevel};
///
/// let mut logs = vec![
///     LogEntry {
///         time: "2025-01-20T10:00:00.000Z".to_string(),
///         index_time: "2025-01-20T10:00:01.000Z".to_string(),
///         serial: Some(1),
///         level: LogLevel::Info,
///         component: "test".to_string(),
///         message: "older".to_string(),
///     },
///     LogEntry {
///         time: "2025-01-20T10:01:00.000Z".to_string(),
///         index_time: "2025-01-20T10:01:01.000Z".to_string(),
///         serial: Some(2),
///         level: LogLevel::Info,
///         component: "test".to_string(),
///         message: "newer".to_string(),
///     },
/// ];
///
/// splunk_client::models::logs::sort_logs_newest_first(&mut logs);
/// assert_eq!(logs[0].message, "newer");
/// assert_eq!(logs[1].message, "older");
/// ```
pub fn sort_logs_newest_first(logs: &mut [LogEntry]) {
    logs.sort_by(LogEntry::cmp_newest_first);
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
        assert_eq!(error.log_level, LogLevel::Error);
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
        assert_eq!(entry.level, LogLevel::Info);
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
        assert_eq!(entry.level, LogLevel::Info);
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
    fn test_content_hash_is_stable_for_same_entry() {
        let entry1 = LogEntry {
            time: "2025-01-20T10:30:00.000Z".to_string(),
            index_time: "2025-01-20T10:30:01.000Z".to_string(),
            serial: None,
            level: LogLevel::Info,
            component: "Metrics".to_string(),
            message: "test message content".to_string(),
        };
        let entry2 = LogEntry {
            time: "2025-01-20T10:30:00.000Z".to_string(),
            index_time: "2025-01-20T10:30:01.000Z".to_string(),
            serial: None,
            level: LogLevel::Info,
            component: "Metrics".to_string(),
            message: "test message content".to_string(),
        };

        // Same content should produce same hash within the same process
        assert_eq!(entry1.content_hash(), entry2.content_hash());
    }

    #[test]
    fn test_content_hash_differs_for_different_content() {
        let entry1 = LogEntry {
            time: "2025-01-20T10:30:00.000Z".to_string(),
            index_time: "2025-01-20T10:30:01.000Z".to_string(),
            serial: None,
            level: LogLevel::Info,
            component: "Metrics".to_string(),
            message: "first message".to_string(),
        };
        let entry2 = LogEntry {
            time: "2025-01-20T10:30:00.000Z".to_string(),
            index_time: "2025-01-20T10:30:01.000Z".to_string(),
            serial: None,
            level: LogLevel::Info,
            component: "Metrics".to_string(),
            message: "second message".to_string(),
        };

        // Different messages should produce different hashes
        assert_ne!(entry1.content_hash(), entry2.content_hash());
    }

    #[test]
    fn test_content_hash_considers_time_and_index_time() {
        let base_entry = LogEntry {
            time: "2025-01-20T10:30:00.000Z".to_string(),
            index_time: "2025-01-20T10:30:01.000Z".to_string(),
            serial: None,
            level: LogLevel::Info,
            component: "Metrics".to_string(),
            message: "test message".to_string(),
        };

        let different_time = LogEntry {
            time: "2025-01-20T10:31:00.000Z".to_string(),
            ..base_entry.clone()
        };
        let different_index_time = LogEntry {
            index_time: "2025-01-20T10:30:02.000Z".to_string(),
            ..base_entry.clone()
        };

        // Different time should produce different hash
        assert_ne!(base_entry.content_hash(), different_time.content_hash());
        // Different index_time should produce different hash
        assert_ne!(
            base_entry.content_hash(),
            different_index_time.content_hash()
        );
    }

    #[test]
    fn test_sort_logs_newest_first_by_time() {
        let mut logs = vec![
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:01.000Z".to_string(),
                serial: Some(1),
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "oldest".to_string(),
            },
            LogEntry {
                time: "2025-01-20T10:02:00.000Z".to_string(),
                index_time: "2025-01-20T10:02:01.000Z".to_string(),
                serial: Some(3),
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "newest".to_string(),
            },
            LogEntry {
                time: "2025-01-20T10:01:00.000Z".to_string(),
                index_time: "2025-01-20T10:01:01.000Z".to_string(),
                serial: Some(2),
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "middle".to_string(),
            },
        ];

        sort_logs_newest_first(&mut logs);

        assert_eq!(logs[0].message, "newest");
        assert_eq!(logs[1].message, "middle");
        assert_eq!(logs[2].message, "oldest");
    }

    #[test]
    fn test_sort_logs_newest_first_tie_breaker_index_time() {
        // Same time, different index_time
        let mut logs = vec![
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:01.000Z".to_string(),
                serial: Some(1),
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "older".to_string(),
            },
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:03.000Z".to_string(),
                serial: Some(3),
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "newest".to_string(),
            },
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:02.000Z".to_string(),
                serial: Some(2),
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "middle".to_string(),
            },
        ];

        sort_logs_newest_first(&mut logs);

        assert_eq!(logs[0].message, "newest");
        assert_eq!(logs[1].message, "middle");
        assert_eq!(logs[2].message, "older");
    }

    #[test]
    fn test_sort_logs_newest_first_tie_breaker_serial() {
        // Same time and index_time, different serial
        let mut logs = vec![
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:01.000Z".to_string(),
                serial: Some(10),
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "older".to_string(),
            },
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:01.000Z".to_string(),
                serial: Some(30),
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "newest".to_string(),
            },
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:01.000Z".to_string(),
                serial: Some(20),
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "middle".to_string(),
            },
        ];

        sort_logs_newest_first(&mut logs);

        assert_eq!(logs[0].message, "newest");
        assert_eq!(logs[1].message, "middle");
        assert_eq!(logs[2].message, "older");
    }

    #[test]
    fn test_sort_logs_newest_first_with_none_serial() {
        // None serial should sort after Some (treated as older)
        let mut logs = vec![
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:01.000Z".to_string(),
                serial: None,
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "no_serial".to_string(),
            },
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:01.000Z".to_string(),
                serial: Some(10),
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "with_serial".to_string(),
            },
        ];

        sort_logs_newest_first(&mut logs);

        // Some(10) should come before None (newer)
        assert_eq!(logs[0].message, "with_serial");
        assert_eq!(logs[1].message, "no_serial");
    }

    #[test]
    fn test_sort_logs_newest_first_with_empty_index_time() {
        // Empty index_time should sort after non-empty (treated as older)
        let mut logs = vec![
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "".to_string(),
                serial: Some(1),
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "empty_index".to_string(),
            },
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:01.000Z".to_string(),
                serial: Some(2),
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "with_index".to_string(),
            },
        ];

        sort_logs_newest_first(&mut logs);

        // Non-empty index_time should come before empty (newer)
        assert_eq!(logs[0].message, "with_index");
        assert_eq!(logs[1].message, "empty_index");
    }

    #[test]
    fn test_sort_logs_newest_first_empty_vec() {
        let mut logs: Vec<LogEntry> = vec![];
        sort_logs_newest_first(&mut logs);
        assert!(logs.is_empty());
    }

    #[test]
    fn test_sort_logs_newest_first_single_element() {
        let mut logs = vec![LogEntry {
            time: "2025-01-20T10:00:00.000Z".to_string(),
            index_time: "2025-01-20T10:00:01.000Z".to_string(),
            serial: Some(1),
            level: LogLevel::Info,
            component: "test".to_string(),
            message: "only".to_string(),
        }];

        sort_logs_newest_first(&mut logs);

        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].message, "only");
    }

    #[test]
    fn test_cmp_newest_first_all_none_serials() {
        // All entries with None serial should be sorted by time/index_time only
        let mut logs = vec![
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:01.000Z".to_string(),
                serial: None,
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "first".to_string(),
            },
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:03.000Z".to_string(),
                serial: None,
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "third".to_string(),
            },
            LogEntry {
                time: "2025-01-20T10:00:00.000Z".to_string(),
                index_time: "2025-01-20T10:00:02.000Z".to_string(),
                serial: None,
                level: LogLevel::Info,
                component: "test".to_string(),
                message: "second".to_string(),
            },
        ];

        sort_logs_newest_first(&mut logs);

        assert_eq!(logs[0].message, "third");
        assert_eq!(logs[1].message, "second");
        assert_eq!(logs[2].message, "first");
    }

    // LogLevel enum tests

    #[test]
    fn test_log_level_deserialize_error() {
        let json = r#""ERROR""#;
        let level: LogLevel = serde_json::from_str(json).unwrap();
        assert_eq!(level, LogLevel::Error);
    }

    #[test]
    fn test_log_level_deserialize_warn() {
        let json = r#""WARN""#;
        let level: LogLevel = serde_json::from_str(json).unwrap();
        assert_eq!(level, LogLevel::Warn);
    }

    #[test]
    fn test_log_level_deserialize_info() {
        let json = r#""INFO""#;
        let level: LogLevel = serde_json::from_str(json).unwrap();
        assert_eq!(level, LogLevel::Info);
    }

    #[test]
    fn test_log_level_deserialize_debug() {
        let json = r#""DEBUG""#;
        let level: LogLevel = serde_json::from_str(json).unwrap();
        assert_eq!(level, LogLevel::Debug);
    }

    #[test]
    fn test_log_level_deserialize_fatal() {
        let json = r#""FATAL""#;
        let level: LogLevel = serde_json::from_str(json).unwrap();
        assert_eq!(level, LogLevel::Fatal);
    }

    #[test]
    fn test_log_level_deserialize_unknown() {
        let json = r#""TRACE""#;
        let level: LogLevel = serde_json::from_str(json).unwrap();
        assert_eq!(level, LogLevel::Unknown);
    }

    #[test]
    fn test_log_level_default() {
        let level: LogLevel = Default::default();
        assert_eq!(level, LogLevel::Unknown);
    }

    #[test]
    fn test_log_level_display() {
        assert_eq!(format!("{}", LogLevel::Error), "ERROR");
        assert_eq!(format!("{}", LogLevel::Warn), "WARN");
        assert_eq!(format!("{}", LogLevel::Info), "INFO");
        assert_eq!(format!("{}", LogLevel::Debug), "DEBUG");
        assert_eq!(format!("{}", LogLevel::Fatal), "FATAL");
        assert_eq!(format!("{}", LogLevel::Unknown), "UNKNOWN");
    }

    #[test]
    fn test_log_level_serialize() {
        assert_eq!(
            serde_json::to_string(&LogLevel::Error).unwrap(),
            r#""ERROR""#
        );
        assert_eq!(serde_json::to_string(&LogLevel::Warn).unwrap(), r#""WARN""#);
        assert_eq!(serde_json::to_string(&LogLevel::Info).unwrap(), r#""INFO""#);
        assert_eq!(
            serde_json::to_string(&LogLevel::Debug).unwrap(),
            r#""DEBUG""#
        );
        assert_eq!(
            serde_json::to_string(&LogLevel::Fatal).unwrap(),
            r#""FATAL""#
        );
        assert_eq!(
            serde_json::to_string(&LogLevel::Unknown).unwrap(),
            r#""UNKNOWN""#
        );
    }

    #[test]
    fn test_log_entry_missing_level_defaults_to_unknown() {
        let json = r#"{
            "_time": "2025-01-20T10:30:00.000Z",
            "component": "Metrics",
            "_raw": "some log message"
        }"#;
        let entry: LogEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.level, LogLevel::Unknown);
    }

    #[test]
    fn test_log_parsing_error_missing_level_defaults_to_unknown() {
        let json = r#"{
            "_time": "2025-01-20T10:30:00.000Z",
            "source": "/opt/splunk/var/log/splunk/metrics.log",
            "sourcetype": "splunkd",
            "message": "Failed to parse timestamp",
            "component": "DateParserVerbose"
        }"#;
        let error: LogParsingError = serde_json::from_str(json).unwrap();
        assert_eq!(error.log_level, LogLevel::Unknown);
    }
}
