//! Formatter tests split by concept.
//!
//! This module contains tests for the formatters module, organized by concept
//! rather than by resource type to avoid excessive fragmentation.
//!
//! Test organization:
//! - `common_tests.rs`: Escaping, helpers, output format parsing
//! - `csv_tests.rs`: CSV formatter tests + JSON flattening tests + Unicode tests
//! - `json_tests.rs`: JSON formatter tests + Unicode tests
//! - `xml_tests.rs`: XML formatter tests + Unicode tests
//! - `table_tests.rs`: Table formatter tests + Unicode tests
//! - `empty_tests.rs`: Empty result set tests
//! - `streaming_tests.rs`: Streaming/tail mode tests

mod common_tests;
mod csv_tests;
mod empty_tests;
mod json_tests;
mod streaming_tests;
mod table_tests;
mod xml_tests;

use splunk_client::models::{LogEntry, LogLevel};

/// Helper function to create a test log entry for streaming tests.
pub fn make_test_log_entry(time: &str, level: &str, component: &str, message: &str) -> LogEntry {
    let log_level = match level {
        "ERROR" => LogLevel::Error,
        "WARN" => LogLevel::Warn,
        "INFO" => LogLevel::Info,
        "DEBUG" => LogLevel::Debug,
        "FATAL" => LogLevel::Fatal,
        _ => LogLevel::Unknown,
    };
    LogEntry {
        time: time.to_string(),
        index_time: "2025-01-24T12:00:01.000Z".to_string(),
        serial: Some(1),
        level: log_level,
        component: component.to_string(),
        message: message.to_string(),
    }
}
