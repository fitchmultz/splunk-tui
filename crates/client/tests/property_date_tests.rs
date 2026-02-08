//! Property-based tests for date/time parsing in Splunk client.
//!
//! This module tests the robustness of timestamp handling with randomly
//! generated inputs to ensure the client correctly parses various date/time
//! formats that Splunk might return.
//!
//! # Test Coverage
//! - Unix timestamp validity (range handling)
//! - ISO 8601 timestamp format validation
//! - Timestamp string roundtrip through JSON serialization
//! - LogEntry deserialization with various time formats
//!
//! # Invariants
//! - Timestamps within the valid range (2010-2035) must parse correctly
//! - ISO 8601 formats with Z suffix or timezone offsets must be preserved
//! - JSON roundtrip must maintain timestamp integrity
//!
//! # What this does NOT handle
//! - Invalid timestamp formats (those are tested in unit tests)
//! - Timezone conversion semantics
//! - Leap second handling

use proptest::prelude::*;
use serde::{Deserialize, Serialize};

/// Minimum valid Unix timestamp (2010-01-01T00:00:00Z)
const MIN_TIMESTAMP: i64 = 1262304000;

/// Maximum valid Unix timestamp (2035-01-01T00:00:00Z)
const MAX_TIMESTAMP: i64 = 2051222400;

/// Helper struct for testing timestamp serialization roundtrips.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
struct TimestampWrapper {
    #[serde(rename = "_time")]
    time: String,
}

/// Generates valid ISO 8601 timestamps within the test range.
///
/// Produces timestamps in formats like:
/// - `2015-08-15T14:30:00Z`
/// - `2020-01-01T00:00:00.000Z`
/// - `2025-12-31T23:59:59.999+00:00`
fn iso8601_timestamp_strategy() -> impl Strategy<Value = String> {
    (MIN_TIMESTAMP..=MAX_TIMESTAMP).prop_flat_map(|unix_ts| {
        // Convert to date components for generating valid ISO 8601
        let datetime = time::OffsetDateTime::from_unix_timestamp(unix_ts)
            .unwrap_or(time::OffsetDateTime::UNIX_EPOCH);

        // Strategy for milliseconds (0-999)
        let millis = 0..1000u16;

        // Strategy for timezone offset (-12:00 to +14:00 in minutes)
        let tz_offset = -720..=840i16;

        (Just(datetime), millis, tz_offset).prop_map(|(dt, ms, tz_min)| {
            let with_millis = dt.replace_nanosecond(ms as u32 * 1_000_000).unwrap_or(dt);

            let with_tz = if tz_min == 0 {
                with_millis.to_offset(time::UtcOffset::UTC)
            } else {
                let offset = time::UtcOffset::from_whole_seconds(tz_min as i32 * 60)
                    .unwrap_or(time::UtcOffset::UTC);
                with_millis.to_offset(offset)
            };

            // Format using RFC3339 (handles both with and without milliseconds)
            with_tz
                .format(&time::format_description::well_known::Rfc3339)
                .unwrap_or_else(|_| dt.to_string())
        })
    })
}

/// Generates Unix timestamps as strings (Splunk sometimes returns these).
fn unix_timestamp_string_strategy() -> impl Strategy<Value = String> {
    (MIN_TIMESTAMP..=MAX_TIMESTAMP).prop_map(|ts| ts.to_string())
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1000,
        ..ProptestConfig::default()
    })]

    /// Tests that Unix timestamps within the valid range can be parsed as i64.
    #[test]
    fn unix_timestamp_validity(timestamp in MIN_TIMESTAMP..=MAX_TIMESTAMP) {
        // Verify timestamp is within expected bounds
        prop_assert!(timestamp >= MIN_TIMESTAMP);
        prop_assert!(timestamp <= MAX_TIMESTAMP);

        // Verify we can convert to OffsetDateTime
        let dt = time::OffsetDateTime::from_unix_timestamp(timestamp);
        prop_assert!(dt.is_ok(), "Timestamp {} should convert to OffsetDateTime", timestamp);

        // Verify roundtrip
        let dt = dt.unwrap();
        let back_to_ts = dt.unix_timestamp();
        prop_assert_eq!(timestamp, back_to_ts, "Unix timestamp roundtrip failed");
    }

    /// Tests that ISO 8601 timestamps are valid and parseable.
    #[test]
    fn iso8601_timestamp_format_validation(timestamp_str in iso8601_timestamp_strategy()) {
        // Verify the timestamp string is not empty
        prop_assert!(!timestamp_str.is_empty());

        // Verify it contains expected ISO 8601 components
        prop_assert!(
            timestamp_str.contains('T') || timestamp_str.contains('t'),
            "ISO 8601 timestamp should contain 'T'"
        );

        // Verify it can be parsed by time crate
        let parsed = time::OffsetDateTime::parse(
            &timestamp_str,
            &time::format_description::well_known::Rfc3339
        );
        prop_assert!(
            parsed.is_ok(),
            "Timestamp '{}' should parse as RFC 3339: {:?}",
            timestamp_str,
            parsed.err()
        );

        // Verify the parsed timestamp is within our valid range
        if let Ok(dt) = parsed {
            let unix_ts = dt.unix_timestamp();
            prop_assert!(
                (MIN_TIMESTAMP..=MAX_TIMESTAMP).contains(&unix_ts),
                "Parsed timestamp {} is outside valid range",
                unix_ts
            );
        }
    }

    /// Tests that timestamp strings roundtrip correctly through JSON.
    #[test]
    fn timestamp_string_roundtrip_through_json(timestamp_str in iso8601_timestamp_strategy()) {
        let wrapper = TimestampWrapper {
            time: timestamp_str.clone(),
        };

        // Serialize to JSON
        let json = serde_json::to_string(&wrapper);
        prop_assert!(
            json.is_ok(),
            "Failed to serialize timestamp wrapper: {:?}",
            json.err()
        );

        // Deserialize back
        let json = json.unwrap();
        let deserialized: Result<TimestampWrapper, _> = serde_json::from_str(&json);
        prop_assert!(
            deserialized.is_ok(),
            "Failed to deserialize timestamp wrapper from '{}': {:?}",
            json,
            deserialized.err()
        );

        // Verify roundtrip integrity
        let deserialized = deserialized.unwrap();
        prop_assert_eq!(
            &wrapper.time,
            &deserialized.time,
            "Timestamp roundtrip through JSON failed: '{}' vs '{}'",
            &wrapper.time,
            &deserialized.time
        );
    }

    /// Tests LogEntry-like deserialization with ISO 8601 timestamps.
    #[test]
    fn log_entry_iso8601_z_format(year in 2010u32..=2035, month in 1u8..=12, day in 1u8..=28) {
        // Construct a valid ISO 8601 timestamp with Z suffix
        let timestamp = format!("{:04}-{:02}-{:02}T12:00:00.000Z", year, month, day);

        let json = format!(
            r#"{{"_time":"{}","log_level":"INFO","component":"Test","_raw":"test message"}}"#,
            timestamp
        );

        // Parse as a generic JSON value first to verify structure
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "JSON should parse: {}", json);

        let parsed = parsed.unwrap();
        let time_value = parsed.get("_time").and_then(|v| v.as_str());
        prop_assert!(time_value.is_some(), "_time field should exist and be a string");
        prop_assert_eq!(time_value.unwrap(), timestamp);
    }

    /// Tests LogEntry-like deserialization with Unix timestamp as string.
    #[test]
    fn log_entry_unix_timestamp_string(timestamp in unix_timestamp_string_strategy()) {
        let json = format!(
            r#"{{"_time":"{}","log_level":"WARN","component":"Metrics","_raw":"metric data"}}"#,
            timestamp
        );

        // Parse as generic JSON to verify structure
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "JSON with unix timestamp should parse: {}", json);

        let parsed = parsed.unwrap();
        let time_value = parsed.get("_time").and_then(|v| v.as_str());
        prop_assert!(time_value.is_some(), "_time field should exist");
        prop_assert_eq!(time_value.unwrap(), timestamp.clone());

        // Verify the timestamp is numeric
        let numeric_check: Result<i64, _> = timestamp.parse();
        prop_assert!(
            numeric_check.is_ok(),
            "Timestamp string should be parseable as i64: {}",
            timestamp
        );
    }

    /// Tests various edge case timestamp formats that Splunk might return.
    #[test]
    fn log_entry_various_time_formats(
        has_millis in prop::bool::ANY,
        use_z_suffix in prop::bool::ANY,
        offset_hours in -12i8..=14
    ) {
        // Base timestamp
        let base_ts = 1609459200i64; // 2021-01-01 00:00:00 UTC
        let dt = time::OffsetDateTime::from_unix_timestamp(base_ts).unwrap();

        // Format based on parameters
        let formatted = if has_millis {
            if use_z_suffix {
                dt.format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default()
            } else {
                // Format with explicit offset
                let offset_str = format!("{:+03}:00", offset_hours);
                dt.format(&time::format_description::well_known::Rfc3339)
                    .unwrap_or_default()
                    .replace('Z', &offset_str)
            }
        } else {
            // Simple format without milliseconds
            format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}{}",
                dt.year(),
                dt.month() as u8,
                dt.day(),
                dt.hour(),
                dt.minute(),
                dt.second(),
                if use_z_suffix { "Z" } else { "" }
            )
        };

        let json = format!(
            r#"{{"_time":"{}","_indextime":"{}","_raw":"test"}}"#,
            formatted, formatted
        );

        // Should parse as valid JSON
        let parsed: Result<serde_json::Value, _> = serde_json::from_str(&json);
        prop_assert!(parsed.is_ok(), "Edge case format should parse: {}", json);
    }
}

/// Module for testing LogEntry with various Splunk date formats.
///
/// This module focuses on testing the `LogEntry` struct specifically,
/// ensuring it can handle the variety of timestamp formats that
/// Splunk's REST API may return in different contexts.
#[cfg(test)]
mod splunk_date_formats {
    use super::*;
    use splunk_client::models::{LogEntry, LogLevel};

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 500,
            ..ProptestConfig::default()
        })]

        /// Tests LogEntry deserialization with ISO 8601 format with Z suffix.
        /// This is the most common format returned by Splunk.
        #[test]
        fn log_entry_iso8601_with_z(
            year in 2010u32..=2034,
            month in 1u8..=12,
            day in 1u8..=28,
            hour in 0u8..=23,
            minute in 0u8..=59,
            second in 0u8..=59
        ) {
            let timestamp = format!(
                "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
                year, month, day, hour, minute, second
            );

            let json = format!(
                r#"{{"_time":"{}","log_level":"INFO","component":"TestComponent","_raw":"Test message content"}}"#,
                timestamp
            );

            let entry: Result<LogEntry, _> = serde_json::from_str(&json);
            prop_assert!(entry.is_ok(), "LogEntry should deserialize with Z format: {}", json);

            let entry = entry.unwrap();
            prop_assert_eq!(entry.time, timestamp);
            prop_assert_eq!(entry.level, LogLevel::Info);
            prop_assert_eq!(entry.component, "TestComponent");
        }

        /// Tests LogEntry deserialization with ISO 8601 format with millisecond precision.
        #[test]
        fn log_entry_iso8601_with_millis(
            year in 2010u32..=2034,
            month in 1u8..=12,
            day in 1u8..=28,
            millis in 0u16..=999
        ) {
            let timestamp = format!(
                "{:04}-{:02}-{:02}T12:00:00.{:03}Z",
                year, month, day, millis
            );

            let json = format!(
                r#"{{"_time":"{}","_indextime":"{}","_serial":42,"log_level":"ERROR","component":"DateParserVerbose","_raw":"Failed to parse timestamp"}}"#,
                timestamp, timestamp
            );

            let entry: Result<LogEntry, _> = serde_json::from_str(&json);
            prop_assert!(
                entry.is_ok(),
                "LogEntry should deserialize with millisecond format: {}",
                json
            );

            let entry = entry.unwrap();
            prop_assert_eq!(entry.time, timestamp.clone());
            prop_assert_eq!(entry.index_time, timestamp);
            prop_assert_eq!(entry.serial, Some(42));
            prop_assert_eq!(entry.level, LogLevel::Error);
        }

        /// Tests LogEntry deserialization with Unix timestamp as string.
        /// Some Splunk endpoints return timestamps as numeric strings.
        #[test]
        fn log_entry_unix_timestamp_as_string(timestamp in MIN_TIMESTAMP..=MAX_TIMESTAMP) {
            let timestamp_str = timestamp.to_string();

            let json = format!(
                r#"{{"_time":"{}","log_level":"DEBUG","component":"Metrics","_raw":"CPU usage: 45%"}}"#,
                timestamp_str
            );

            let entry: Result<LogEntry, _> = serde_json::from_str(&json);
            prop_assert!(
                entry.is_ok(),
                "LogEntry should deserialize with Unix timestamp string: {}",
                json
            );

            let entry = entry.unwrap();
            prop_assert_eq!(entry.time, timestamp_str);
        }

        /// Tests LogEntry with various timezone offset formats.
        #[test]
        fn log_entry_with_timezone_offset(
            offset_hours in -12i8..=14,
            offset_minutes in 0u8..=59
        ) {
            // Skip invalid offset combinations
            prop_assume!(offset_hours != 0 || offset_minutes == 0);

            let sign = if offset_hours >= 0 { '+' } else { '-' };
            let abs_hours = offset_hours.abs();

            let timestamp = format!(
                "2021-06-15T10:30:00.000{}{:02}:{:02}",
                sign, abs_hours, offset_minutes
            );

            let json = format!(
                r#"{{"_time":"{}","log_level":"WARN","component":"Splunkd","_raw":"Warning message"}}"#,
                timestamp
            );

            let entry: Result<LogEntry, _> = serde_json::from_str(&json);
            prop_assert!(
                entry.is_ok(),
                "LogEntry should deserialize with timezone offset: {}",
                json
            );

            let entry = entry.unwrap();
            prop_assert_eq!(entry.time, timestamp);
        }

        /// Tests LogEntry with empty/minimal fields.
        #[test]
        fn log_entry_edge_cases(timestamp in iso8601_timestamp_strategy()) {
            // Test with empty index_time (common in some Splunk responses)
            let json = format!(
                r#"{{"_time":"{}","_raw":"Minimal entry"}}"#,
                timestamp
            );

            let entry: Result<LogEntry, _> = serde_json::from_str(&json);
            prop_assert!(
                entry.is_ok(),
                "LogEntry should deserialize with minimal fields: {}",
                json
            );

            let entry = entry.unwrap();
            prop_assert_eq!(entry.time, timestamp);
            prop_assert_eq!(entry.index_time, ""); // Default empty string
            prop_assert_eq!(entry.level, LogLevel::Unknown); // Default level
            prop_assert_eq!(entry.component, ""); // Default empty
        }

        /// Tests LogEntry JSON roundtrip preserves all fields.
        #[test]
        fn log_entry_json_roundtrip(
            time in iso8601_timestamp_strategy(),
            index_time in iso8601_timestamp_strategy(),
            serial in 0usize..=10000,
            message in "[a-zA-Z0-9 ]{1,50}"
        ) {
            let entry = LogEntry {
                time,
                index_time,
                serial: Some(serial),
                level: LogLevel::Info,
                component: "RoundtripTest".to_string(),
                message: message.clone(),
            };

            // Serialize to JSON
            let json = serde_json::to_string(&entry);
            prop_assert!(json.is_ok(), "LogEntry should serialize");

            // Deserialize back
            let json = json.unwrap();
            let deserialized: Result<LogEntry, _> = serde_json::from_str(&json);
            prop_assert!(deserialized.is_ok(), "LogEntry should deserialize: {}", json);

            let deserialized = deserialized.unwrap();
            prop_assert_eq!(entry.time, deserialized.time);
            prop_assert_eq!(entry.index_time, deserialized.index_time);
            prop_assert_eq!(entry.serial, deserialized.serial);
            prop_assert_eq!(entry.level, deserialized.level);
            prop_assert_eq!(entry.component, deserialized.component);
            prop_assert_eq!(entry.message, deserialized.message);
        }
    }
}

/// Tests for timestamp range boundary conditions.
#[cfg(test)]
mod timestamp_boundary_tests {
    use super::*;

    proptest! {
        #![proptest_config(ProptestConfig {
            cases: 500,
            ..ProptestConfig::default()
        })]

        /// Tests that timestamps at boundary values are handled correctly.
        #[test]
        fn boundary_timestamp_handling(
            timestamp in prop_oneof![
                Just(MIN_TIMESTAMP),
                Just(MAX_TIMESTAMP),
                Just(1577836800i64), // 2020-01-01
                Just(1609459200i64), // 2021-01-01
                Just(1640995200i64), // 2022-01-01
            ]
        ) {
            let dt = time::OffsetDateTime::from_unix_timestamp(timestamp);
            prop_assert!(dt.is_ok());

            let dt = dt.unwrap();
            let formatted = dt.format(&time::format_description::well_known::Rfc3339).unwrap();

            // Verify it can be used in a JSON context
            let wrapper = TimestampWrapper { time: formatted };
            let json = serde_json::to_string(&wrapper);
            prop_assert!(json.is_ok());
        }

        /// Tests timestamp ordering preserves expected relationships.
        #[test]
        fn timestamp_ordering(ts1 in MIN_TIMESTAMP..MAX_TIMESTAMP, ts2 in MIN_TIMESTAMP..MAX_TIMESTAMP) {
            let dt1 = time::OffsetDateTime::from_unix_timestamp(ts1).unwrap();
            let dt2 = time::OffsetDateTime::from_unix_timestamp(ts2).unwrap();

            let fmt1 = dt1.format(&time::format_description::well_known::Rfc3339).unwrap();
            let fmt2 = dt2.format(&time::format_description::well_known::Rfc3339).unwrap();

            // The string comparison should match the timestamp comparison
            // when both use the same timezone (UTC in this case)
            let ordering = ts1.cmp(&ts2);
            let string_ordering = fmt1.cmp(&fmt2);

            prop_assert_eq!(
                ordering,
                string_ordering,
                "Timestamp ordering should match string ordering: {} vs {}",
                fmt1,
                fmt2
            );
        }
    }
}
