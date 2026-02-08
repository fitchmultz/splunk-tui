//! Property-based tests for serde_helpers deserializers.
//!
//! This module tests all custom deserializers in splunk_client::serde_helpers
//! using proptest to ensure they handle various input types correctly:
//! - Numbers (integers, floats)
//! - String representations of numbers
//! - Null values
//! - Missing fields (for optional variants)
//!
//! # Test Coverage
//! - `usize_from_string_or_number` - required usize from number or string
//! - `opt_usize_from_string_or_number` - optional usize from number, string, null, or missing
//! - `string_from_number_or_string` - required String from number or string
//! - `u64_from_string_or_number` - required u64 from number or string
//! - `opt_u64_from_string_or_number` - optional u64 from number, string, null, or missing
//!
//! # Invariants
//! - All deserializers must accept both JSON numbers and numeric strings
//! - Optional deserializers must handle null and missing fields
//! - Float values should be converted to their string representation
//! - Parsing errors should be propagated correctly
//!
//! # What this does NOT handle
//! - Testing deserialization of invalid/unsupported types (those are unit tests)
//! - Testing the internal enums (U64OrString, StringOrNumber)

use proptest::prelude::*;
use serde::Deserialize;

// Wrapper structs for testing each deserializer

#[derive(Debug, Deserialize, PartialEq)]
struct UsizeWrapper {
    #[serde(deserialize_with = "splunk_client::serde_helpers::usize_from_string_or_number")]
    value: usize,
}

#[derive(Debug, Deserialize, PartialEq)]
struct OptUsizeWrapper {
    #[serde(
        default,
        deserialize_with = "splunk_client::serde_helpers::opt_usize_from_string_or_number"
    )]
    value: Option<usize>,
}

#[derive(Debug, Deserialize, PartialEq)]
struct StringWrapper {
    #[serde(deserialize_with = "splunk_client::serde_helpers::string_from_number_or_string")]
    value: String,
}

#[derive(Debug, Deserialize, PartialEq)]
struct U64Wrapper {
    #[serde(deserialize_with = "splunk_client::serde_helpers::u64_from_string_or_number")]
    value: u64,
}

#[derive(Debug, Deserialize, PartialEq)]
struct OptU64Wrapper {
    #[serde(
        default,
        deserialize_with = "splunk_client::serde_helpers::opt_u64_from_string_or_number"
    )]
    value: Option<u64>,
}

// ============================================================================
// usize_from_string_or_number tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Test that usize deserializer correctly handles JSON integer numbers.
    #[test]
    fn usize_from_number(num: usize) {
        let json = format!(r#"{{"value":{}}}"#, num);
        let parsed: UsizeWrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, num);
    }

    /// Test that usize deserializer correctly handles numeric strings.
    #[test]
    fn usize_from_string(num in 0usize..10_000_000usize) {
        let json = format!(r#"{{"value":"{}"}}"#, num);
        let parsed: UsizeWrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, num);
    }
}

// ============================================================================
// opt_usize_from_string_or_number tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Test that optional usize deserializer correctly handles JSON integer numbers.
    #[test]
    fn opt_usize_from_number(num: usize) {
        let json = format!(r#"{{"value":{}}}"#, num);
        let parsed: OptUsizeWrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, Some(num));
    }

    /// Test that optional usize deserializer correctly handles numeric strings.
    #[test]
    fn opt_usize_from_string(num in 0usize..10_000_000usize) {
        let json = format!(r#"{{"value":"{}"}}"#, num);
        let parsed: OptUsizeWrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, Some(num));
    }
}

// ============================================================================
// string_from_number_or_string tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Test that string deserializer correctly handles JSON integer numbers (u64 range).
    #[test]
    fn string_from_u64_number(num: u64) {
        let json = format!(r#"{{"value":{}}}"#, num);
        let parsed: StringWrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, num.to_string());
    }

    /// Test that string deserializer correctly handles JSON integer numbers (i64 positive).
    #[test]
    fn string_from_i64_positive(num in 0i64..i64::MAX) {
        let json = format!(r#"{{"value":{}}}"#, num);
        let parsed: StringWrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, num.to_string());
    }

    /// Test that string deserializer correctly handles JSON integer numbers (i64 negative).
    #[test]
    fn string_from_i64_negative(num in i64::MIN..0i64) {
        let json = format!(r#"{{"value":{}}}"#, num);
        let parsed: StringWrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, num.to_string());
    }

    /// Test that string deserializer correctly handles string values.
    #[test]
    fn string_from_string(s in "[a-zA-Z0-9_]{1,50}") {
        let json = format!(r#"{{"value":"{}"}}"#, s);
        let parsed: StringWrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, s);
    }

    /// Test that string deserializer correctly handles float values.
    #[test]
    fn string_from_f64(num in -1e10f64..1e10f64) {
        // Skip special float values (NaN, Infinity)
        prop_assume!(num.is_finite());
        let json = format!(r#"{{"value":{}}}"#, num);
        let parsed: StringWrapper = serde_json::from_str(&json).unwrap();
        // Float formatting may vary between serialization and to_string()
        // Instead verify the parsed string converts back to the same float value
        let parsed_as_f64: f64 = parsed.value.parse().unwrap();
        // Use relative epsilon comparison for float equality
        let epsilon = (num.abs() * 1e-10).max(1e-10);
        prop_assert!(
            (parsed_as_f64 - num).abs() < epsilon,
            "Float mismatch: parsed {} vs original {} (epsilon {})",
            parsed_as_f64, num, epsilon
        );
    }
}

// ============================================================================
// u64_from_string_or_number tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Test that u64 deserializer correctly handles JSON integer numbers.
    #[test]
    fn u64_from_number(num: u64) {
        let json = format!(r#"{{"value":{}}}"#, num);
        let parsed: U64Wrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, num);
    }

    /// Test that u64 deserializer correctly handles numeric strings.
    #[test]
    fn u64_from_string(num in 0u64..10_000_000_000u64) {
        let json = format!(r#"{{"value":"{}"}}"#, num);
        let parsed: U64Wrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, num);
    }
}

// ============================================================================
// opt_u64_from_string_or_number tests
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Test that optional u64 deserializer correctly handles JSON integer numbers.
    #[test]
    fn opt_u64_from_number(num: u64) {
        let json = format!(r#"{{"value":{}}}"#, num);
        let parsed: OptU64Wrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, Some(num));
    }

    /// Test that optional u64 deserializer correctly handles numeric strings.
    #[test]
    fn opt_u64_from_string(num in 0u64..10_000_000_000u64) {
        let json = format!(r#"{{"value":"{}"}}"#, num);
        let parsed: OptU64Wrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, Some(num));
    }
}

// ============================================================================
// Edge case tests for signed integer handling in unsigned deserializers
// ============================================================================

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1000))]

    /// Test that usize deserializer correctly handles positive i64 values.
    #[test]
    fn usize_from_positive_i64(num in 0i64..i64::MAX) {
        // Ensure the value fits in usize
        prop_assume!(num as u64 <= usize::MAX as u64);
        let json = format!(r#"{{"value":{}}}"#, num);
        let parsed: UsizeWrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, num as usize);
    }

    /// Test that u64 deserializer correctly handles positive i64 values.
    #[test]
    fn u64_from_positive_i64(num in 0i64..i64::MAX) {
        let json = format!(r#"{{"value":{}}}"#, num);
        let parsed: U64Wrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, num as u64);
    }

    /// Test that optional usize deserializer correctly handles positive i64 values.
    #[test]
    fn opt_usize_from_positive_i64(num in 0i64..i64::MAX) {
        // Ensure the value fits in usize
        prop_assume!(num as u64 <= usize::MAX as u64);
        let json = format!(r#"{{"value":{}}}"#, num);
        let parsed: OptUsizeWrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, Some(num as usize));
    }

    /// Test that optional u64 deserializer correctly handles positive i64 values.
    #[test]
    fn opt_u64_from_positive_i64(num in 0i64..i64::MAX) {
        let json = format!(r#"{{"value":{}}}"#, num);
        let parsed: OptU64Wrapper = serde_json::from_str(&json).unwrap();
        prop_assert_eq!(parsed.value, Some(num as u64));
    }
}

// ============================================================================
// Null and missing field tests (non-property based)
// ============================================================================

#[cfg(test)]
mod null_and_missing_tests {
    use super::*;

    #[test]
    fn opt_usize_from_null() {
        let json = r#"{"value":null}"#;
        let parsed: OptUsizeWrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, None);
    }

    #[test]
    fn opt_usize_from_missing() {
        let json = r#"{}"#;
        let parsed: OptUsizeWrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, None);
    }

    #[test]
    fn opt_u64_from_null() {
        let json = r#"{"value":null}"#;
        let parsed: OptU64Wrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, None);
    }

    #[test]
    fn opt_u64_from_missing() {
        let json = r#"{}"#;
        let parsed: OptU64Wrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, None);
    }
}

// ============================================================================
// Boundary value tests
// ============================================================================

#[cfg(test)]
mod boundary_tests {
    use super::*;

    #[test]
    fn usize_max_value_from_number() {
        let json = format!(r#"{{"value":{}}}"#, usize::MAX);
        let parsed: UsizeWrapper = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.value, usize::MAX);
    }

    #[test]
    fn usize_max_value_from_string() {
        let json = format!(r#"{{"value":"{}"}}"#, usize::MAX);
        let parsed: UsizeWrapper = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.value, usize::MAX);
    }

    #[test]
    fn u64_max_value_from_number() {
        let json = format!(r#"{{"value":{}}}"#, u64::MAX);
        let parsed: U64Wrapper = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.value, u64::MAX);
    }

    #[test]
    fn u64_max_value_from_string() {
        let json = format!(r#"{{"value":"{}"}}"#, u64::MAX);
        let parsed: U64Wrapper = serde_json::from_str(&json).unwrap();
        assert_eq!(parsed.value, u64::MAX);
    }

    #[test]
    fn usize_zero_from_number() {
        let json = r#"{"value":0}"#;
        let parsed: UsizeWrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, 0);
    }

    #[test]
    fn usize_zero_from_string() {
        let json = r#"{"value":"0"}"#;
        let parsed: UsizeWrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, 0);
    }

    #[test]
    fn u64_zero_from_number() {
        let json = r#"{"value":0}"#;
        let parsed: U64Wrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, 0);
    }

    #[test]
    fn u64_zero_from_string() {
        let json = r#"{"value":"0"}"#;
        let parsed: U64Wrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, 0);
    }

    #[test]
    fn string_from_zero() {
        let json = r#"{"value":0}"#;
        let parsed: StringWrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, "0");
    }

    #[test]
    fn string_from_zero_string() {
        let json = r#"{"value":"0"}"#;
        let parsed: StringWrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, "0");
    }

    #[test]
    fn opt_usize_zero_from_number() {
        let json = r#"{"value":0}"#;
        let parsed: OptUsizeWrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, Some(0));
    }

    #[test]
    fn opt_u64_zero_from_number() {
        let json = r#"{"value":0}"#;
        let parsed: OptU64Wrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, Some(0));
    }

    #[test]
    fn string_from_large_f64() {
        // Test a large float that should serialize to scientific notation
        let json = r#"{"value":1.5e20}"#;
        let parsed: StringWrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, "150000000000000000000");
    }

    #[test]
    fn string_from_small_f64() {
        // Test a small float
        let json = r#"{"value":0.000001}"#;
        let parsed: StringWrapper = serde_json::from_str(json).unwrap();
        assert_eq!(parsed.value, "0.000001");
    }
}
