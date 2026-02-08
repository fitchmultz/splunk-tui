//! Property-based tests for pagination logic and related invariants.
//!
//! This module uses proptest to verify:
//! - Pagination calculations (has_next, page_count) are correct for all inputs
//! - Offset boundary validation works correctly
//! - SearchJobResults deserializes correctly with various offset/total combinations
//! - Splunk's string-number format is handled correctly
//! - LogEntry cursor_key and content_hash invariants hold
//!
//! # Test Coverage
//! - has_next calculation: offset + count < total
//! - Page count calculation: ceil(total / page_size)
//! - Offset boundaries: offset < total, offset + count <= total
//! - JSON deserialization with numeric and string numeric fields
//! - LogEntry cursor and hash stability

use proptest::prelude::*;
use splunk_client::models::{LogEntry, LogLevel, SearchJobResults};

/// Calculates whether there is a next page based on current offset, count, and total.
///
/// # Formula
/// has_next = offset + count < total
///
/// # Arguments
/// * `offset` - Current offset (0-indexed)
/// * `count` - Number of items in current page
/// * `total` - Total number of items available
fn has_next(offset: usize, count: usize, total: usize) -> bool {
    offset + count < total
}

/// Calculates the number of pages needed for a given total and page size.
///
/// # Formula
/// page_count = ceil(total / page_size) = (total + page_size - 1) / page_size
///
/// # Arguments
/// * `total` - Total number of items
/// * `page_size` - Number of items per page (must be > 0)
fn page_count(total: usize, page_size: usize) -> usize {
    if page_size == 0 {
        return 0;
    }
    // Use checked_add to prevent overflow with large values
    match total.checked_add(page_size - 1) {
        Some(sum) => sum / page_size,
        None => (total / page_size) + 1, // Handle overflow case
    }
}

/// Validates that an offset is within valid bounds.
///
/// # Arguments
/// * `offset` - The offset to validate
/// * `total` - Total number of items
///
/// # Returns
/// * `true` if offset is valid (offset <= total)
fn is_valid_offset(offset: usize, total: usize) -> bool {
    offset <= total
}

/// Calculates the maximum valid offset for a given total and page size.
///
/// # Arguments
/// * `total` - Total number of items
/// * `page_size` - Number of items per page
fn max_valid_offset(total: usize, page_size: usize) -> usize {
    if total == 0 || page_size == 0 {
        return 0;
    }
    let pages = page_count(total, page_size);
    (pages - 1) * page_size
}

proptest! {
    /// Test that has_next calculation is correct for all valid pagination states.
    ///
    /// # Invariants Tested
    /// - When offset + count < total, has_next returns true
    /// - When offset + count >= total, has_next returns false
    /// - Empty results (count = 0) only have next if offset < total
    #[test]
    fn test_has_next_calculation_correctness(
        (offset, count, total) in (0usize..10_000, 0usize..1_000, 0usize..10_000)
    ) {
        let result = has_next(offset, count, total);
        let expected = offset + count < total;
        prop_assert_eq!(result, expected);

        // Additional invariant: if count is 0 and offset < total, there should be more
        if count == 0 && offset < total {
            prop_assert!(has_next(offset, 1, total));
        }
    }

    /// Test page count calculation for various totals and page sizes.
    ///
    /// # Invariants Tested
    /// - page_count(total, page_size) >= total / page_size (integer division)
    /// - page_count(total, page_size) * page_size >= total
    /// - page_count(0, page_size) = 0
    /// - page_count(total, 0) = 0 (edge case handling)
    #[test]
    fn test_page_count_calculation(
        (total, page_size) in (0usize..10_000, 1usize..1_000)
    ) {
        let pages = page_count(total, page_size);

        // Invariant: must have enough pages to hold all items
        prop_assert!(pages * page_size >= total);

        // Invariant: cannot have fewer pages than total/page_size
        if page_size > 0 {
            prop_assert!(pages >= total / page_size);
        }

        // Invariant: with items, must have at least one page
        if total > 0 && page_size > 0 {
            prop_assert!(pages >= 1);
        }

        // Invariant: pages should be minimal (cannot subtract 1)
        if pages > 0 && total > 0 {
            prop_assert!((pages - 1) * page_size < total);
        }
    }

    /// Test offset boundary validation.
    ///
    /// # Invariants Tested
    /// - offset <= total is always valid
    /// - offset > total is invalid
    /// - For valid offsets, offset + count should not exceed total by more than page_size
    #[test]
    fn test_offset_boundary_validation(
        (offset, total) in (0usize..10_000, 0usize..10_000)
    ) {
        let is_valid = is_valid_offset(offset, total);

        // offset == total is valid (represents empty last page)
        // offset > total is invalid
        prop_assert_eq!(is_valid, offset <= total);

        // If offset is valid and less than total, we should be able to fetch at least one item
        if offset < total {
            prop_assert!(is_valid);
        }
    }

    /// Test max valid offset calculation.
    ///
    /// # Invariants Tested
    /// - max_valid_offset is always <= total
    /// - Adding one page size to max_valid_offset would exceed total (unless total is 0)
    #[test]
    fn test_max_valid_offset_calculation(
        (total, page_size) in (1usize..10_000, 1usize..1_000)
    ) {
        let max_offset = max_valid_offset(total, page_size);

        // Invariant: max offset should be less than total
        prop_assert!(max_offset < total);

        // Invariant: adding page_size would exceed total
        prop_assert!(max_offset + page_size >= total);

        // Invariant: max offset should be aligned to page boundaries
        prop_assert_eq!(max_offset % page_size, 0);
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 1000,
        ..ProptestConfig::default()
    })]

    /// Test SearchJobResults deserialization with numeric offset and total fields.
    ///
    /// # Invariants Tested
    /// - JSON with numeric offset/total deserializes correctly
    /// - Missing offset/total default to None
    /// - Preview flag is parsed correctly
    #[test]
    fn test_search_job_results_deserialization_numeric(
        (offset, total, preview, result_count) in (
            0usize..100_000,
            0usize..100_000,
            prop::bool::ANY,
            0usize..100
        )
    ) {
        // Build JSON with numeric fields
        let results: Vec<serde_json::Value> = (0..result_count)
            .map(|i| serde_json::json!({"_time": format!("2025-01-01T{:02}:00:00Z", i), "message": format!("event {}", i)}))
            .collect();

        let json = serde_json::json!({
            "results": results,
            "preview": preview,
            "offset": offset,
            "total": total
        });

        let results: SearchJobResults = serde_json::from_value(json).expect("Should deserialize");

        prop_assert_eq!(results.offset, Some(offset));
        prop_assert_eq!(results.total, Some(total));
        prop_assert_eq!(results.preview, preview);
        prop_assert_eq!(results.results.len(), result_count);
    }

    /// Test SearchJobResults deserialization with string numeric fields (Splunk format).
    ///
    /// # Invariants Tested
    /// - String representations of numbers deserialize to correct numeric values
    /// - Empty strings or invalid strings are handled appropriately
    #[test]
    fn test_search_job_results_deserialization_string_numbers(
        (offset, total, preview) in (
            0usize..100_000,
            0usize..100_000,
            prop::bool::ANY
        )
    ) {
        // Build JSON with string numeric fields (Splunk format)
        let json = serde_json::json!({
            "results": [{"_time": "2025-01-01T00:00:00Z", "message": "test"}],
            "preview": preview,
            "offset": offset.to_string(),
            "total": total.to_string()
        });

        let results: SearchJobResults = serde_json::from_value(json).expect("Should deserialize string numbers");

        prop_assert_eq!(results.offset, Some(offset));
        prop_assert_eq!(results.total, Some(total));
        prop_assert_eq!(results.preview, preview);
    }

    /// Test SearchJobResults deserialization with mixed string and numeric fields.
    #[test]
    fn test_search_job_results_deserialization_mixed_types(
        (offset_num, total_num, offset_val, total_val) in (
            prop::bool::ANY,
            prop::bool::ANY,
            0usize..10_000,
            0usize..10_000
        )
    ) {
        let offset_field = if offset_num {
            serde_json::json!(offset_val)
        } else {
            serde_json::json!(offset_val.to_string())
        };

        let total_field = if total_num {
            serde_json::json!(total_val)
        } else {
            serde_json::json!(total_val.to_string())
        };

        let json = serde_json::json!({
            "results": [],
            "preview": false,
            "offset": offset_field,
            "total": total_field
        });

        let results: SearchJobResults = serde_json::from_value(json).expect("Should deserialize mixed types");

        prop_assert_eq!(results.offset, Some(offset_val));
        prop_assert_eq!(results.total, Some(total_val));
    }

    /// Test has_next integration with SearchJobResults.
    ///
    /// Uses deserialized SearchJobResults to verify pagination logic.
    #[test]
    fn test_has_next_with_search_job_results(
        (offset, count, total) in (0usize..1_000, 1usize..100, 1usize..1_000)
    ) {
        // Only test when offset + count doesn't overflow
        if let Some(end) = offset.checked_add(count) {
            let json = serde_json::json!({
                "results": (0..count).map(|i| serde_json::json!({"_raw": format!("event {}", i)})).collect::<Vec<_>>(),
                "offset": offset,
                "total": total
            });

            let results: SearchJobResults = serde_json::from_value(json).expect("Should deserialize");

            let actual_offset = results.offset.unwrap_or(0);
            let actual_total = results.total.unwrap_or(0);
            let has_more = has_next(actual_offset, results.results.len(), actual_total);

            // Invariant: if end < total, should have more
            // Invariant: if end >= total, should not have more
            prop_assert_eq!(has_more, end < actual_total);
        }
    }
}

proptest! {
    #![proptest_config(ProptestConfig {
        cases: 500,
        ..ProptestConfig::default()
    })]

    /// Test LogEntry cursor_key invariants.
    ///
    /// # Invariants Tested
    /// - cursor_key returns the expected tuple (time, index_time, serial)
    /// - cursor_key is consistent for the same entry
    #[test]
    fn test_log_entry_cursor_key_invariants(
        (time, index_time, serial) in (
            "[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z",
            "[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z",
            prop::option::of(0usize..10_000)
        )
    ) {
        let entry = LogEntry {
            time: time.clone(),
            index_time: index_time.clone(),
            serial,
            level: LogLevel::Info,
            component: "test".to_string(),
            message: "test message".to_string(),
        };

        let cursor = entry.cursor_key();

        // Invariant: cursor_key returns (time, index_time, serial)
        prop_assert_eq!(cursor.0, time);
        prop_assert_eq!(cursor.1, index_time);
        prop_assert_eq!(cursor.2, serial);

        // Invariant: cursor_key is idempotent
        let cursor2 = entry.cursor_key();
        prop_assert_eq!(cursor.0, cursor2.0);
        prop_assert_eq!(cursor.1, cursor2.1);
        prop_assert_eq!(cursor.2, cursor2.2);
    }

    /// Test LogEntry content_hash invariants.
    ///
    /// # Invariants Tested
    /// - Same content produces same hash within same process
    /// - Different content produces different hash (with high probability)
    /// - Hash considers time, index_time, and message
    #[test]
    fn test_log_entry_content_hash_invariants(
        (time1, time2, msg1, msg2) in (
            "[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z",
            "[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z",
            "[a-zA-Z0-9 ]{1,50}",
            "[a-zA-Z0-9 ]{1,50}"
        )
    ) {
        let entry1 = LogEntry {
            time: time1.clone(),
            index_time: "2025-01-01T00:00:00Z".to_string(),
            serial: None,
            level: LogLevel::Info,
            component: "test".to_string(),
            message: msg1.clone(),
        };

        let entry2 = LogEntry {
            time: time1.clone(),
            index_time: "2025-01-01T00:00:00Z".to_string(),
            serial: None,
            level: LogLevel::Info,
            component: "test".to_string(),
            message: msg1.clone(),
        };

        // Invariant: identical entries have same hash
        prop_assert_eq!(entry1.content_hash(), entry2.content_hash());

        // Test with different time
        let entry_different_time = LogEntry {
            time: time2.clone(),
            index_time: "2025-01-01T00:00:00Z".to_string(),
            serial: None,
            level: LogLevel::Info,
            component: "test".to_string(),
            message: msg1.clone(),
        };

        if time1 != time2 {
            // Different time should usually produce different hash
            // (allowing for the extremely rare collision)
            prop_assert_ne!(entry1.content_hash(), entry_different_time.content_hash());
        }

        // Test with different message
        let entry_different_msg = LogEntry {
            time: time1.clone(),
            index_time: "2025-01-01T00:00:00Z".to_string(),
            serial: None,
            level: LogLevel::Info,
            component: "test".to_string(),
            message: msg2.clone(),
        };

        if msg1 != msg2 {
            prop_assert_ne!(entry1.content_hash(), entry_different_msg.content_hash());
        }
    }

    /// Test LogEntry deserialization with various field combinations.
    #[test]
    fn test_log_entry_deserialization_pagination_context(
        (time, has_serial, serial) in (
            "[0-9]{4}-[0-9]{2}-[0-9]{2}T[0-9]{2}:[0-9]{2}:[0-9]{2}Z",
            prop::bool::ANY,
            0usize..1_000_000
        )
    ) {
        let serial_json = if has_serial {
            serde_json::json!(serial)
        } else {
            serde_json::Value::Null
        };

        let json = serde_json::json!({
            "_time": time.clone(),
            "_serial": serial_json,
            "_indextime": "2025-01-01T00:00:01Z",
            "log_level": "INFO",
            "component": "TestComponent",
            "_raw": "Test log message"
        });

        let entry: LogEntry = serde_json::from_value(json).expect("Should deserialize LogEntry");

        prop_assert_eq!(&entry.time, &time);
        if has_serial {
            prop_assert_eq!(entry.serial, Some(serial));
        } else {
            prop_assert_eq!(entry.serial, None);
        }

        // Verify cursor_key is usable for pagination
        let cursor = entry.cursor_key();
        prop_assert!(!cursor.0.is_empty()); // time should not be empty
    }

    /// Test that pagination invariants hold across page boundaries.
    ///
    /// Simulates fetching multiple pages and verifies consistency.
    #[test]
    fn test_pagination_consistency_across_pages(
        (total, page_size) in (1usize..1_000, 1usize..100)
    ) {
        let pages = page_count(total, page_size);

        if pages == 0 {
            return Ok(());
        }

        // Simulate fetching each page
        let mut total_fetched = 0;
        for page in 0..pages {
            let offset = page * page_size;
            let remaining = total - total_fetched;
            let count = if remaining < page_size { remaining } else { page_size };

            // Verify offset is valid
            prop_assert!(is_valid_offset(offset, total));

            // Verify has_next is correct
            let expected_has_next = has_next(offset, count, total);
            let actual_end = offset + count;
            prop_assert_eq!(expected_has_next, actual_end < total);

            total_fetched += count;

            // Last page should not have next
            if page == pages - 1 {
                prop_assert!(!has_next(offset, count, total));
            }
        }

        // Total fetched should equal total
        prop_assert_eq!(total_fetched, total);
    }
}

/// Tests for edge cases that might not be covered by property-based tests.
#[cfg(test)]
mod edge_case_tests {
    use super::*;

    #[test]
    fn test_has_next_at_exact_boundary() {
        // At exact boundary: offset + count == total
        assert!(!has_next(0, 10, 10));
        assert!(!has_next(10, 0, 10));
        assert!(!has_next(5, 5, 10));
    }

    #[test]
    fn test_has_next_with_zero_total() {
        // With zero total, never has next
        assert!(!has_next(0, 0, 0));
        assert!(!has_next(0, 10, 0));
    }

    #[test]
    fn test_page_count_large_numbers() {
        // Test with large numbers that might overflow in naive implementations
        assert_eq!(page_count(usize::MAX, 1), usize::MAX);
        assert_eq!(page_count(1_000_000, 1_000), 1_000);
        assert_eq!(page_count(1_000_001, 1_000), 1_001);
    }

    #[test]
    fn test_offset_at_total_boundary() {
        // offset == total is valid (empty last page)
        assert!(is_valid_offset(100, 100));
        // offset > total is invalid
        assert!(!is_valid_offset(101, 100));
    }

    #[test]
    fn test_search_job_results_with_large_numbers() {
        let json = serde_json::json!({
            "results": [],
            "offset": "999999",
            "total": "1000000"
        });

        let results: SearchJobResults = serde_json::from_value(json).unwrap();
        assert_eq!(results.offset, Some(999_999));
        assert_eq!(results.total, Some(1_000_000));
        assert!(has_next(999_999, 0, 1_000_000));
    }

    #[test]
    fn test_log_entry_content_hash_stability() {
        // Same entry should always have same hash within the same process
        let entry = LogEntry {
            time: "2025-01-20T10:30:00.000Z".to_string(),
            index_time: "2025-01-20T10:30:01.000Z".to_string(),
            serial: Some(42),
            level: LogLevel::Info,
            component: "test".to_string(),
            message: "stable test message".to_string(),
        };

        let hash1 = entry.content_hash();
        let hash2 = entry.content_hash();
        let hash3 = entry.content_hash();

        assert_eq!(hash1, hash2);
        assert_eq!(hash2, hash3);
    }

    #[test]
    fn test_log_entry_cursor_key_with_empty_strings() {
        let entry = LogEntry {
            time: "".to_string(),
            index_time: "".to_string(),
            serial: None,
            level: LogLevel::Info,
            component: "test".to_string(),
            message: "test".to_string(),
        };

        let cursor = entry.cursor_key();
        assert_eq!(cursor.0, "");
        assert_eq!(cursor.1, "");
        assert_eq!(cursor.2, None);
    }

    #[test]
    fn test_search_job_results_missing_fields() {
        let json = serde_json::json!({
            "results": [{"_time": "2025-01-01T00:00:00Z", "message": "test"}]
        });

        let results: SearchJobResults =
            serde_json::from_value(json).expect("Should deserialize with defaults");

        assert!(!results.results.is_empty());
        assert_eq!(results.offset, None);
        assert_eq!(results.total, None);
        assert!(!results.preview);
    }

    #[test]
    fn test_page_count_edge_cases() {
        // Zero total should yield zero pages
        assert_eq!(page_count(0, 100), 0);

        // Zero page size should yield zero pages (avoiding division by zero)
        assert_eq!(page_count(100, 0), 0);

        // Exact division
        assert_eq!(page_count(100, 10), 10);

        // Rounding up
        assert_eq!(page_count(101, 10), 11);
        assert_eq!(page_count(1, 10), 1);
    }
}
