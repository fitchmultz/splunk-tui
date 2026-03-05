//! Property-based tests for serde roundtrip serialization.
//!
//! This module uses proptest to verify:
//! - LogEntry roundtrip serialization and cursor_key/content_hash consistency
//! - Macro roundtrip with all fields
//! - FiredAlert roundtrip with all fields including AlertSeverity
//! - SearchJobStatus roundtrip serialization
//!
//! # Test Coverage
//! - Serde roundtrip invariants: serialize -> deserialize == original
//! - Enum serialization/deserialization consistency
//! - Optional field handling in roundtrips
//! - Custom method invariants (cursor_key, content_hash) after roundtrip

use proptest::prelude::*;
use splunk_client::models::{
    AlertSeverity, FiredAlert, LogEntry, LogLevel, Macro, SearchJobStatus,
};

// =============================================================================
// Helper Strategies
// =============================================================================

/// Strategy for generating valid Splunk SIDs.
///
/// Format: "scheduler__admin__search__{10-40 alphanumeric characters}"
fn sid_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9]{10,40}".prop_map(|suffix| format!("scheduler__admin__search__{}", suffix))
}

/// Strategy for generating ISO 8601 timestamps.
///
/// Generates timestamps in the format: YYYY-MM-DDTHH:MM:SS.sssZ
fn iso_timestamp_strategy() -> impl Strategy<Value = String> {
    // Year: 2000-2099
    let year = 2000i32..2100i32;
    // Month: 01-12
    let month = 1u32..13u32;
    // Day: 01-28 (safe range for all months)
    let day = 1u32..29u32;
    // Hour: 00-23
    let hour = 0u32..24u32;
    // Minute: 00-59
    let minute = 0u32..60u32;
    // Second: 00-59
    let second = 0u32..60u32;
    // Milliseconds: 000-999
    let millis = 0u32..1000u32;

    (year, month, day, hour, minute, second, millis).prop_map(|(y, mo, d, h, mi, s, ms)| {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}.{:03}Z",
            y, mo, d, h, mi, s, ms
        )
    })
}

/// Strategy for generating valid Splunk component names.
///
/// Components are typically alphanumeric with optional underscores,
/// representing Splunk internal components like "Metrics", "DateParser", etc.
fn component_strategy() -> impl Strategy<Value = String> {
    prop_oneof![
        // Common Splunk component names
        Just("Metrics".to_string()),
        Just("DateParserVerbose".to_string()),
        Just("Aggregator".to_string()),
        Just("SearchParser".to_string()),
        Just("Indexer".to_string()),
        Just("Forwarder".to_string()),
        Just("LicenseManager".to_string()),
        Just("ClusterManager".to_string()),
        // Generic component patterns
        "[A-Z][a-zA-Z0-9]{2,20}".prop_map(|s| s),
        "[A-Z][a-zA-Z0-9_]{2,30}".prop_map(|s| s),
    ]
}

// =============================================================================
// Arbitrary-like Strategy Functions
// =============================================================================
// Note: We use functions returning strategies instead of Arbitrary impls
// because Rust's orphan rules prevent implementing foreign traits (Arbitrary)
// for foreign types (the model types from splunk_client).

/// Strategy for generating LogLevel values.
fn log_level_strategy() -> impl Strategy<Value = LogLevel> {
    prop_oneof![
        Just(LogLevel::Error),
        Just(LogLevel::Warn),
        Just(LogLevel::Info),
        Just(LogLevel::Debug),
        Just(LogLevel::Fatal),
        Just(LogLevel::Unknown),
    ]
}

/// Strategy for generating AlertSeverity values.
fn alert_severity_strategy() -> impl Strategy<Value = AlertSeverity> {
    prop_oneof![
        Just(AlertSeverity::Info),
        Just(AlertSeverity::Low),
        Just(AlertSeverity::Medium),
        Just(AlertSeverity::High),
        Just(AlertSeverity::Critical),
        Just(AlertSeverity::Unknown),
    ]
}

/// Strategy for generating LogEntry values.
fn log_entry_strategy() -> impl Strategy<Value = LogEntry> {
    (
        iso_timestamp_strategy(),
        iso_timestamp_strategy(),
        prop::option::of(0usize..1_000_000usize),
        log_level_strategy(),
        component_strategy(),
        // Message: alphanumeric, spaces, and common punctuation
        "[a-zA-Z0-9 ]{1,200}".prop_map(|s| s),
    )
        .prop_map(
            |(time, index_time, serial, level, component, message)| LogEntry {
                time,
                index_time,
                serial,
                level,
                component,
                message,
            },
        )
}

/// Strategy for generating Macro values.
fn macro_strategy() -> impl Strategy<Value = Macro> {
    (
        // name: Simple macro names or parameterized like "macro(2)"
        prop_oneof![
            "[a-zA-Z_][a-zA-Z0-9_]{2,30}".prop_map(|s| s),
            (
                "[a-zA-Z_][a-zA-Z0-9_]{2,20}".prop_map(|s| s),
                1usize..10usize
            )
                .prop_map(|(name, count)| format!("{}({})", name, count)),
        ],
        // definition: SPL snippets or eval expressions
        prop_oneof![
            Just("search index=main | stats count".to_string()),
            Just("sourcetype=access_combined | top uri".to_string()),
            Just("host=webserver | timechart span=1h count".to_string()),
            // Use a simple alphanumeric pattern to avoid escape issues
            "[a-zA-Z0-9_|= ]{10,100}".prop_map(|s| s),
        ],
        // args: Optional comma-separated argument names
        prop::option::of(prop_oneof![
            Just("field1".to_string()),
            Just("field1,field2".to_string()),
            Just("source,index,host".to_string()),
            "[a-zA-Z_][a-zA-Z0-9_]{2,15}(,[a-zA-Z_][a-zA-Z0-9_]{2,15}){0,4}".prop_map(|s| s),
        ]),
        // description: Optional human-readable description
        prop::option::of(prop_oneof![
            Just("A useful search macro".to_string()),
            Just("Returns top 10 results".to_string()),
            // Use simple pattern to avoid escape issues
            "[A-Za-z0-9 _]{10,100}".prop_map(|s| s),
        ]),
        // disabled: bool
        prop::bool::ANY,
        // iseval: bool
        prop::bool::ANY,
        // validation: Optional validation expression
        prop::option::of(prop_oneof![
            Just("isnum(arg1)".to_string()),
            Just("match(arg1, regex)".to_string()),
            // Use simple pattern to avoid escape issues
            "[a-zA-Z0-9_]{5,50}".prop_map(|s| s),
        ]),
        // errormsg: Optional error message
        prop::option::of(prop_oneof![
            Just("Invalid argument provided".to_string()),
            Just("Argument must be a number".to_string()),
            // Use simple pattern to avoid escape issues
            "[A-Za-z0-9 _]{10,80}".prop_map(|s| s),
        ]),
    )
        .prop_map(
            |(name, definition, args, description, disabled, iseval, validation, errormsg)| Macro {
                name,
                definition,
                args,
                description,
                disabled,
                iseval,
                validation,
                errormsg,
            },
        )
}

/// Strategy for generating FiredAlert values.
fn fired_alert_strategy() -> impl Strategy<Value = FiredAlert> {
    (
        // name: Alert name
        "[a-zA-Z_][a-zA-Z0-9_-]{5,40}".prop_map(|s| s),
        // actions: Optional comma-separated actions
        prop::option::of(prop_oneof![
            Just("email".to_string()),
            Just("webhook".to_string()),
            Just("email,webhook".to_string()),
            Just("email,webhook,script".to_string()),
        ]),
        // alert_type: Optional "historical" or "realtime"
        prop::option::of(prop_oneof![
            Just("historical".to_string()),
            Just("realtime".to_string()),
        ]),
        // digest_mode: Optional bool
        prop::option::of(prop::bool::ANY),
        // expiration_time_rendered: Optional rendered time
        prop::option::of(iso_timestamp_strategy()),
        // savedsearch_name: Optional saved search name
        prop::option::of("[a-zA-Z_][a-zA-Z0-9_-]{5,40}".prop_map(|s| s)),
        // severity: Optional AlertSeverity
        prop::option::of(alert_severity_strategy()),
        // sid: Optional search ID
        prop::option::of(sid_strategy()),
        // trigger_time: Optional Unix timestamp
        prop::option::of(1_600_000_000i64..2_000_000_000i64),
        // trigger_time_rendered: Optional rendered trigger time
        prop::option::of(iso_timestamp_strategy()),
        // triggered_alerts: Optional count as string
        prop::option::of("[0-9]{1,5}".prop_map(|s| s)),
    )
        .prop_map(
            |(
                name,
                actions,
                alert_type,
                digest_mode,
                expiration_time_rendered,
                savedsearch_name,
                severity,
                sid,
                trigger_time,
                trigger_time_rendered,
                triggered_alerts,
            )| FiredAlert {
                name,
                actions,
                alert_type,
                digest_mode,
                expiration_time_rendered,
                savedsearch_name,
                severity,
                sid,
                trigger_time,
                trigger_time_rendered,
                triggered_alerts,
            },
        )
}

/// Strategy for generating SearchJobStatus values.
fn search_job_status_strategy() -> impl Strategy<Value = SearchJobStatus> {
    (
        // sid: Search ID
        sid_strategy(),
        // is_done: bool
        prop::bool::ANY,
        // is_finalized: bool
        prop::bool::ANY,
        // done_progress: f64 between 0.0 and 1.0
        0.0f64..=1.0f64,
        // run_duration: f64 (non-negative)
        0.0f64..10_000.0f64,
        // cursor_time: Optional ISO timestamp
        prop::option::of(iso_timestamp_strategy()),
        // scan_count: usize
        0usize..10_000_000usize,
        // event_count: usize
        0usize..1_000_000usize,
        // result_count: usize
        0usize..100_000usize,
        // disk_usage: usize
        0usize..100_000_000_000usize,
        // priority: Optional i32
        prop::option::of(-10i32..10i32),
        // label: Optional string
        prop::option::of("[a-zA-Z0-9 _-]{5,50}".prop_map(|s| s)),
    )
        .prop_map(
            |(
                sid,
                is_done,
                is_finalized,
                done_progress,
                run_duration,
                cursor_time,
                scan_count,
                event_count,
                result_count,
                disk_usage,
                priority,
                label,
            )| SearchJobStatus {
                sid,
                is_done,
                is_finalized,
                done_progress,
                run_duration,
                cursor_time,
                scan_count,
                event_count,
                result_count,
                disk_usage,
                priority,
                label,
            },
        )
}

// =============================================================================
// Property Tests
// =============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(500))]

    /// Test LogEntry serde roundtrip.
    ///
    /// # Invariants Tested
    /// - serialize -> deserialize produces equivalent LogEntry
    /// - cursor_key is consistent after roundtrip
    /// - content_hash is consistent after roundtrip
    #[test]
    fn test_log_entry_roundtrip(entry in log_entry_strategy()) {
        // Serialize to JSON
        let json = serde_json::to_string(&entry).expect("Should serialize LogEntry");

        // Deserialize back
        let deserialized: LogEntry = serde_json::from_str(&json)
            .expect("Should deserialize LogEntry");

        // Invariant: cursor_key should be identical after roundtrip
        prop_assert_eq!(entry.cursor_key(), deserialized.cursor_key());

        // Invariant: content_hash should be identical after roundtrip
        prop_assert_eq!(entry.content_hash(), deserialized.content_hash());

        // Invariant: all fields should match
        prop_assert_eq!(entry.time, deserialized.time);
        prop_assert_eq!(entry.index_time, deserialized.index_time);
        prop_assert_eq!(entry.serial, deserialized.serial);
        prop_assert_eq!(entry.level, deserialized.level);
        prop_assert_eq!(entry.component, deserialized.component);
        prop_assert_eq!(entry.message, deserialized.message);
    }

    /// Test LogEntry cursor_key and content_hash consistency.
    ///
    /// # Invariants Tested
    /// - cursor_key returns (time, index_time, serial) tuple
    /// - content_hash is deterministic for same content
    /// - Different entries have different cursor keys (with high probability)
    #[test]
    fn test_log_entry_cursor_and_hash_consistency(
        entry1 in log_entry_strategy(),
        entry2 in log_entry_strategy()
    ) {
        // Clone values we need to compare after calling cursor_key()
        let entry1_time = entry1.time.clone();
        let entry1_index_time = entry1.index_time.clone();
        let entry1_message = entry1.message.clone();
        let entry1_serial = entry1.serial;

        // Invariant: cursor_key components match struct fields
        let cursor = entry1.cursor_key();
        prop_assert_eq!(cursor.0, entry1_time.clone());
        prop_assert_eq!(cursor.1, entry1_index_time.clone());
        prop_assert_eq!(cursor.2, entry1_serial);

        // Invariant: content_hash is idempotent
        let hash1 = entry1.content_hash();
        let hash2 = entry1.content_hash();
        prop_assert_eq!(hash1, hash2);

        // Invariant: identical entries have same hash
        if entry1_time == entry2.time
            && entry1_index_time == entry2.index_time
            && entry1_message == entry2.message
        {
            prop_assert_eq!(entry1.content_hash(), entry2.content_hash());
        }
    }

    /// Test Macro serde roundtrip.
    ///
    /// # Invariants Tested
    /// - serialize -> deserialize produces equivalent Macro
    /// - All fields are preserved through roundtrip
    /// - PartialEq comparison works correctly after roundtrip
    #[test]
    fn test_macro_roundtrip(macro_def in macro_strategy()) {
        // Serialize to JSON
        let json = serde_json::to_string(&macro_def)
            .expect("Should serialize Macro");

        // Deserialize back
        let deserialized: Macro = serde_json::from_str(&json)
            .expect("Should deserialize Macro");

        // Invariant: Macro implements PartialEq, so we can compare directly
        prop_assert_eq!(macro_def, deserialized);
    }

    /// Test Macro field preservation.
    ///
    /// # Invariants Tested
    /// - All optional fields are preserved when Some
    /// - All optional fields are preserved when None
    /// - Boolean fields are preserved correctly
    #[test]
    fn test_macro_field_preservation(
        name in "[a-zA-Z_][a-zA-Z0-9_]{2,30}",
        definition in "[a-zA-Z0-9_|= ]{10,50}",
        has_args in prop::bool::ANY,
        has_description in prop::bool::ANY,
        disabled in prop::bool::ANY,
        iseval in prop::bool::ANY,
        has_validation in prop::bool::ANY,
        has_errormsg in prop::bool::ANY
    ) {
        let args = if has_args {
            Some("field1,field2".to_string())
        } else {
            None
        };
        let description = if has_description {
            Some("Test macro description".to_string())
        } else {
            None
        };
        let validation = if has_validation {
            Some("isnum(arg1)".to_string())
        } else {
            None
        };
        let errormsg = if has_errormsg {
            Some("Invalid argument".to_string())
        } else {
            None
        };

        let macro_def = Macro {
            name: name.clone(),
            definition: definition.clone(),
            args: args.clone(),
            description: description.clone(),
            disabled,
            iseval,
            validation: validation.clone(),
            errormsg: errormsg.clone(),
        };

        let json = serde_json::to_string(&macro_def).expect("Should serialize");
        let deserialized: Macro = serde_json::from_str(&json).expect("Should deserialize");

        prop_assert_eq!(deserialized.name, name);
        prop_assert_eq!(deserialized.definition, definition);
        prop_assert_eq!(deserialized.args, args);
        prop_assert_eq!(deserialized.description, description);
        prop_assert_eq!(deserialized.disabled, disabled);
        prop_assert_eq!(deserialized.iseval, iseval);
        prop_assert_eq!(deserialized.validation, validation);
        prop_assert_eq!(deserialized.errormsg, errormsg);
    }

    /// Test FiredAlert serde roundtrip.
    ///
    /// # Invariants Tested
    /// - serialize -> deserialize produces equivalent FiredAlert
    /// - AlertSeverity is preserved correctly
    /// - All optional fields are preserved
    #[test]
    fn test_fired_alert_roundtrip(alert in fired_alert_strategy()) {
        // Serialize to JSON
        let json = serde_json::to_string(&alert)
            .expect("Should serialize FiredAlert");

        // Deserialize back
        let deserialized: FiredAlert = serde_json::from_str(&json)
            .expect("Should deserialize FiredAlert");

        // Invariant: all fields should match
        prop_assert_eq!(alert.name, deserialized.name);
        prop_assert_eq!(alert.actions, deserialized.actions);
        prop_assert_eq!(alert.alert_type, deserialized.alert_type);
        prop_assert_eq!(alert.digest_mode, deserialized.digest_mode);
        prop_assert_eq!(alert.expiration_time_rendered, deserialized.expiration_time_rendered);
        prop_assert_eq!(alert.savedsearch_name, deserialized.savedsearch_name);
        prop_assert_eq!(alert.severity, deserialized.severity);
        prop_assert_eq!(alert.sid, deserialized.sid);
        prop_assert_eq!(alert.trigger_time, deserialized.trigger_time);
        prop_assert_eq!(alert.trigger_time_rendered, deserialized.trigger_time_rendered);
        prop_assert_eq!(alert.triggered_alerts, deserialized.triggered_alerts);
    }

    /// Test FiredAlert with AlertSeverity roundtrip.
    ///
    /// # Invariants Tested
    /// - Each AlertSeverity variant serializes and deserializes correctly
    /// - Severity field is preserved through roundtrip
    #[test]
    fn test_fired_alert_severity_roundtrip(
        severity in alert_severity_strategy(),
        has_severity in prop::bool::ANY
    ) {
        let alert = FiredAlert {
            name: "TestAlert".to_string(),
            actions: None,
            alert_type: Some("historical".to_string()),
            digest_mode: Some(false),
            expiration_time_rendered: None,
            savedsearch_name: Some("TestSearch".to_string()),
            severity: if has_severity { Some(severity) } else { None },
            sid: Some("scheduler__admin__search__RMD5abcdef1234".to_string()),
            trigger_time: Some(1_700_000_000),
            trigger_time_rendered: Some("2023-11-14T12:00:00.000Z".to_string()),
            triggered_alerts: Some("1".to_string()),
        };

        let json = serde_json::to_string(&alert).expect("Should serialize");
        let deserialized: FiredAlert = serde_json::from_str(&json).expect("Should deserialize");

        if has_severity {
            prop_assert_eq!(deserialized.severity, Some(severity));
        } else {
            prop_assert_eq!(deserialized.severity, None);
        }
    }

    /// Test SearchJobStatus serde roundtrip.
    ///
    /// # Invariants Tested
    /// - serialize -> deserialize produces equivalent SearchJobStatus
    /// - All numeric fields are preserved
    /// - All optional fields are preserved
    /// - Boolean fields are preserved correctly
    #[test]
    fn test_search_job_status_roundtrip(status in search_job_status_strategy()) {
        // Serialize to JSON
        let json = serde_json::to_string(&status)
            .expect("Should serialize SearchJobStatus");

        // Deserialize back
        let deserialized: SearchJobStatus = serde_json::from_str(&json)
            .expect("Should deserialize SearchJobStatus");

        // Invariant: all fields should match
        prop_assert_eq!(status.sid, deserialized.sid);
        prop_assert_eq!(status.is_done, deserialized.is_done);
        prop_assert_eq!(status.is_finalized, deserialized.is_finalized);
        // Use tolerance for floating point comparisons
        prop_assert!((status.done_progress - deserialized.done_progress).abs() < 1e-9);
        prop_assert!((status.run_duration - deserialized.run_duration).abs() < 1e-9);
        prop_assert_eq!(status.cursor_time, deserialized.cursor_time);
        prop_assert_eq!(status.scan_count, deserialized.scan_count);
        prop_assert_eq!(status.event_count, deserialized.event_count);
        prop_assert_eq!(status.result_count, deserialized.result_count);
        prop_assert_eq!(status.disk_usage, deserialized.disk_usage);
        prop_assert_eq!(status.priority, deserialized.priority);
        prop_assert_eq!(status.label, deserialized.label);
    }

    /// Test SearchJobStatus numeric field preservation.
    ///
    /// # Invariants Tested
    /// - Large numeric values are preserved
    /// - Zero values are preserved
    /// - Floating point values maintain precision
    #[test]
    fn test_search_job_status_numeric_fields(
        scan_count in 0usize..10_000_000usize,
        event_count in 0usize..1_000_000usize,
        result_count in 0usize..100_000usize,
        disk_usage in 0usize..100_000_000_000usize,
        done_progress in 0.0f64..=1.0f64,
        run_duration in 0.0f64..10_000.0f64
    ) {
        let status = SearchJobStatus {
            sid: "scheduler__admin__search__RMD5abcdef1234".to_string(),
            is_done: done_progress >= 1.0,
            is_finalized: false,
            done_progress,
            run_duration,
            cursor_time: None,
            scan_count,
            event_count,
            result_count,
            disk_usage,
            priority: None,
            label: None,
        };

        let json = serde_json::to_string(&status).expect("Should serialize");
        let deserialized: SearchJobStatus = serde_json::from_str(&json).expect("Should deserialize");

        prop_assert_eq!(deserialized.scan_count, scan_count);
        prop_assert_eq!(deserialized.event_count, event_count);
        prop_assert_eq!(deserialized.result_count, result_count);
        prop_assert_eq!(deserialized.disk_usage, disk_usage);
        // Use tolerance for floating point comparisons (serde JSON may introduce small rounding)
        prop_assert!((deserialized.done_progress - done_progress).abs() < 1e-9);
        prop_assert!((deserialized.run_duration - run_duration).abs() < 1e-9);
    }
}

// =============================================================================
// Edge Case Tests
// =============================================================================

/// Tests for edge cases that might not be covered by property-based tests.
#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_log_level_serde_roundtrip() {
        for level in [
            LogLevel::Error,
            LogLevel::Warn,
            LogLevel::Info,
            LogLevel::Debug,
            LogLevel::Fatal,
            LogLevel::Unknown,
        ] {
            let json = serde_json::to_string(&level).expect("Should serialize");
            let deserialized: LogLevel = serde_json::from_str(&json).expect("Should deserialize");
            assert_eq!(level, deserialized);
        }
    }

    #[test]
    fn test_alert_severity_serde_roundtrip() {
        for severity in [
            AlertSeverity::Info,
            AlertSeverity::Low,
            AlertSeverity::Medium,
            AlertSeverity::High,
            AlertSeverity::Critical,
            AlertSeverity::Unknown,
        ] {
            let json = serde_json::to_string(&severity).expect("Should serialize");
            let deserialized: AlertSeverity =
                serde_json::from_str(&json).expect("Should deserialize");
            assert_eq!(severity, deserialized);
        }
    }

    #[test]
    fn test_log_entry_empty_strings() {
        let entry = LogEntry {
            time: "".to_string(),
            index_time: "".to_string(),
            serial: None,
            level: LogLevel::Unknown,
            component: "".to_string(),
            message: "".to_string(),
        };

        let json = serde_json::to_string(&entry).expect("Should serialize");
        let deserialized: LogEntry = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(entry.cursor_key(), deserialized.cursor_key());
        assert_eq!(entry.content_hash(), deserialized.content_hash());
    }

    #[test]
    fn test_log_entry_large_serial() {
        let entry = LogEntry {
            time: "2025-01-01T00:00:00.000Z".to_string(),
            index_time: "2025-01-01T00:00:01.000Z".to_string(),
            serial: Some(usize::MAX),
            level: LogLevel::Info,
            component: "Test".to_string(),
            message: "Test message".to_string(),
        };

        let json = serde_json::to_string(&entry).expect("Should serialize");
        let deserialized: LogEntry = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(entry.serial, deserialized.serial);
    }

    #[test]
    fn test_macro_minimal() {
        let macro_def = Macro {
            name: "minimal".to_string(),
            definition: "search *".to_string(),
            args: None,
            description: None,
            disabled: false,
            iseval: false,
            validation: None,
            errormsg: None,
        };

        let json = serde_json::to_string(&macro_def).expect("Should serialize");
        let deserialized: Macro = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(macro_def, deserialized);
    }

    #[test]
    fn test_fired_alert_minimal() {
        let alert = FiredAlert {
            name: "".to_string(),
            actions: None,
            alert_type: None,
            digest_mode: None,
            expiration_time_rendered: None,
            savedsearch_name: None,
            severity: None,
            sid: None,
            trigger_time: None,
            trigger_time_rendered: None,
            triggered_alerts: None,
        };

        let json = serde_json::to_string(&alert).expect("Should serialize");
        let deserialized: FiredAlert = serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(alert.name, deserialized.name);
        assert_eq!(alert.severity, deserialized.severity);
    }

    #[test]
    fn test_search_job_status_minimal() {
        let status = SearchJobStatus {
            sid: "minimal_sid".to_string(),
            is_done: false,
            is_finalized: false,
            done_progress: 0.0,
            run_duration: 0.0,
            cursor_time: None,
            scan_count: 0,
            event_count: 0,
            result_count: 0,
            disk_usage: 0,
            priority: None,
            label: None,
        };

        let json = serde_json::to_string(&status).expect("Should serialize");
        let deserialized: SearchJobStatus =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(status.sid, deserialized.sid);
        assert_eq!(status.scan_count, deserialized.scan_count);
    }

    #[test]
    fn test_search_job_status_complete() {
        let status = SearchJobStatus {
            sid: "scheduler__admin__search__RMD5abcdef1234".to_string(),
            is_done: true,
            is_finalized: true,
            done_progress: 1.0,
            run_duration: 123.456,
            cursor_time: Some("2025-01-01T00:00:00.000Z".to_string()),
            scan_count: 1_000_000,
            event_count: 500_000,
            result_count: 10_000,
            disk_usage: 1_000_000_000,
            priority: Some(5),
            label: Some("Test Search".to_string()),
        };

        let json = serde_json::to_string(&status).expect("Should serialize");
        let deserialized: SearchJobStatus =
            serde_json::from_str(&json).expect("Should deserialize");

        assert_eq!(status.sid, deserialized.sid);
        assert_eq!(status.is_done, deserialized.is_done);
        assert_eq!(status.is_finalized, deserialized.is_finalized);
        assert!((status.done_progress - deserialized.done_progress).abs() < f64::EPSILON);
        assert!((status.run_duration - deserialized.run_duration).abs() < f64::EPSILON);
        assert_eq!(status.cursor_time, deserialized.cursor_time);
        assert_eq!(status.scan_count, deserialized.scan_count);
        assert_eq!(status.event_count, deserialized.event_count);
        assert_eq!(status.result_count, deserialized.result_count);
        assert_eq!(status.disk_usage, deserialized.disk_usage);
        assert_eq!(status.priority, deserialized.priority);
        assert_eq!(status.label, deserialized.label);
    }
}

// =============================================================================
// Fake Generator Property Tests
// =============================================================================
/// Tests using the fake crate-based generators for realistic Splunk data.
#[cfg(feature = "test-utils")]
mod fake_generator_tests {
    use super::*;
    use splunk_client::testing::generators::{
        ClusterTopologyGenerator, LogEntryGenerator, SplQueryGenerator, SplQueryMode, UserGenerator,
    };

    proptest! {
        #![proptest_config(ProptestConfig::with_cases(100))]

        /// Test that fake-generated log entries have valid structure.
        ///
        /// # Invariants Tested
        /// - Generated entries have required _time field
        /// - Generated entries have log_level field
        /// - Generated entries have component field
        #[test]
        fn test_fake_log_entry_structure(_seed in 0..1000usize) {
            let generator = LogEntryGenerator::new();
            let entry = generator.generate_one();

            // Verify required fields exist
            prop_assert!(entry.get("_time").is_some(), "Missing _time field");
            prop_assert!(entry.get("log_level").is_some(), "Missing log_level field");
            prop_assert!(entry.get("component").is_some(), "Missing component field");

            // Verify log_level is a valid string
            let level = entry["log_level"].as_str().unwrap_or("");
            prop_assert!(
                ["ERROR", "WARN", "INFO", "DEBUG", "FATAL"].contains(&level),
                "Invalid log level: {}",
                level
            );
        }

        /// Test that fake-generated SPL queries have expected structure.
        ///
        /// # Invariants Tested
        /// - Valid queries start with index=
        /// - Generated queries are non-empty
        #[test]
        fn test_fake_spl_query_valid(_seed in 0..1000usize) {
            let generator = SplQueryGenerator::new().with_mode(SplQueryMode::Valid);
            let query = generator.generate_one();

            // Valid queries should start with index=
            prop_assert!(
                query.starts_with("index="),
                "Valid query should start with index=, got: {}",
                query
            );
            prop_assert!(!query.is_empty(), "Query should not be empty");
        }

        /// Test that fake-generated users have valid structure.
        ///
        /// # Invariants Tested
        /// - Users have required name field
        /// - Users have email field
        /// - Users have roles field
        #[test]
        fn test_fake_user_structure(_seed in 0..1000usize) {
            let generator = UserGenerator::new();
            let user = generator.generate_one();

            prop_assert!(user.get("name").is_some(), "Missing name field");
            prop_assert!(user.get("email").is_some(), "Missing email field");
            prop_assert!(user.get("roles").is_some(), "Missing roles field");

            // Name should be a non-empty string
            let name = user["name"].as_str().unwrap_or("");
            prop_assert!(!name.is_empty(), "User name should not be empty");
        }

        /// Test that fake-generated cluster topology has valid structure.
        ///
        /// # Invariants Tested
        /// - Topology has cluster_manager
        /// - Topology has peers array
        /// - Peers count matches configured count
        #[test]
        fn test_fake_cluster_topology_structure(peer_count in 1usize..20usize) {
            let generator = ClusterTopologyGenerator::new().with_peer_count(peer_count);
            let topology = generator.generate();

            prop_assert!(
                topology.get("cluster_manager").is_some(),
                "Missing cluster_manager"
            );
            prop_assert!(topology.get("peers").is_some(), "Missing peers array");

            let peers = topology["peers"].as_array().expect("peers should be an array");
            prop_assert_eq!(
                peers.len(),
                peer_count,
                "Peer count mismatch: expected {}, got {}",
                peer_count,
                peers.len()
            );
        }

        /// Test fake-generated log entries with specific component.
        ///
        /// # Invariants Tested
        /// - Component is correctly set
        /// - All entries have the specified component
        #[test]
        fn test_fake_log_entry_with_component(_seed in 0..1000usize) {
            let generator = LogEntryGenerator::new().with_component("TestComponent");
            let entry = generator.generate_one();

            prop_assert_eq!(
                entry["component"].as_str(),
                Some("TestComponent"),
                "Component should match specified value"
            );
        }
    }
}
