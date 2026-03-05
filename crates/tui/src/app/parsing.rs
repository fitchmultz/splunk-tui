//! Parsing helpers for converting API string values to typed values.
//!
//! Responsibilities:
//! - Parse numeric fields from Splunk API responses with proper error logging
//! - Handle edge cases like "auto", empty strings, and malformed values
//!
//! Does NOT handle:
//! - Does NOT handle key event parsing (see input/helpers.rs)
//! - Does NOT handle configuration parsing (see splunk-config)

use crate::app::App;

impl App {
    /// Parse a numeric index field from string to usize, logging a warning on failure.
    ///
    /// The Splunk API returns numeric fields as `Option<String>`, but we need
    /// `Option<usize>` for the UI. Parse failures indicate potential API version
    /// mismatches or unexpected values (e.g., "auto" instead of a number).
    ///
    /// # Arguments
    ///
    /// * `value` - The string value to parse
    /// * `field_name` - Name of the field being parsed (for logging)
    /// * `index_name` - Name of the index (for logging context)
    ///
    /// # Returns
    ///
    /// `Some(usize)` on successful parse, `None` on failure (with warning logged)
    pub fn parse_index_numeric_field(
        value: &str,
        field_name: &str,
        index_name: &str,
    ) -> Option<usize> {
        match value.parse::<usize>() {
            Ok(n) => Some(n),
            Err(e) => {
                tracing::warn!(
                    "Failed to parse {} '{}' for index '{}': {} - value will be treated as unset",
                    field_name,
                    value,
                    index_name,
                    e
                );
                None
            }
        }
    }

    /// Convenience wrapper for parsing max_hot_buckets specifically.
    ///
    /// See [`Self::parse_index_numeric_field`] for details.
    pub fn parse_max_hot_buckets(value: &str, index_name: &str) -> Option<usize> {
        Self::parse_index_numeric_field(value, "max_hot_buckets", index_name)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_max_hot_buckets_valid() {
        assert_eq!(App::parse_max_hot_buckets("10", "test_idx"), Some(10));
        assert_eq!(App::parse_max_hot_buckets("0", "test_idx"), Some(0));
        assert_eq!(App::parse_max_hot_buckets("100", "test_idx"), Some(100));
    }

    #[test]
    fn test_parse_max_hot_buckets_invalid_string() {
        assert_eq!(App::parse_max_hot_buckets("auto", "test_idx"), None);
        assert_eq!(App::parse_max_hot_buckets("invalid", "test_idx"), None);
    }

    #[test]
    fn test_parse_max_hot_buckets_empty() {
        assert_eq!(App::parse_max_hot_buckets("", "test_idx"), None);
    }

    #[test]
    fn test_parse_max_hot_buckets_negative() {
        assert_eq!(App::parse_max_hot_buckets("-1", "test_idx"), None);
    }

    #[test]
    fn test_parse_max_hot_buckets_whitespace() {
        assert_eq!(App::parse_max_hot_buckets(" 10 ", "test_idx"), None);
        assert_eq!(App::parse_max_hot_buckets("10 ", "test_idx"), None);
    }

    #[test]
    fn test_parse_index_numeric_field_with_custom_name() {
        assert_eq!(
            App::parse_index_numeric_field("5", "max_warm_db_count", "my_index"),
            Some(5)
        );
        assert_eq!(
            App::parse_index_numeric_field("invalid", "max_warm_db_count", "my_index"),
            None
        );
    }
}
