//! URL encoding utilities for constructing safe API paths.
//!
//! Provides percent-encoding for URL path segments to handle special characters
//! in resource names (usernames, index names, etc.) that could otherwise cause
//! path traversal or incorrect URL resolution.
//!
//! # Security Considerations
//!
//! Without percent-encoding, special characters in resource names could:
//! - Cause path traversal (e.g., `user/name` would create a nested path)
//! - Break URL parsing (e.g., `user?name` would create a query parameter)
//! - Cause double-decode issues (e.g., `user%20name` might be decoded prematurely)
//!
//! # Example
//!
//! ```
//! use splunk_client::endpoints::url_encoding::encode_path_segment;
//!
//! let encoded = encode_path_segment("user/name");
//! assert_eq!(encoded, "user%2Fname");
//! ```

use percent_encoding::{AsciiSet, CONTROLS, percent_encode};

/// Characters that must be percent-encoded in URL path segments.
///
/// Based on RFC 3986 section 3.3, plus additional characters that have
/// special meaning in Splunk REST API paths or could cause issues:
/// - Space, quotes, angle brackets: problematic in URLs
/// - Backslash, pipe, caret, backtick, tilde: often blocked or problematic
/// - Plus, comma, semicolon: can have special meaning in some contexts
/// - Curly braces, square brackets: reserved in URI templates
/// - Percent: must be encoded to prevent double-encoding issues
/// - Slash: must be encoded to prevent path traversal
/// - Question mark and hash: have special URL meaning
pub const PATH_SEGMENT_ENCODE_SET: &AsciiSet = &CONTROLS
    .add(b' ')      // Space
    .add(b'"')      // Double quote
    .add(b'<')      // Less than
    .add(b'>')      // Greater than
    .add(b'`')      // Backtick
    .add(b'{')      // Left curly brace
    .add(b'}')      // Right curly brace
    .add(b'|')      // Pipe
    .add(b'\\')     // Backslash
    .add(b'^')      // Caret
    .add(b'~')      // Tilde
    .add(b'%')      // Percent (prevents double-encoding)
    .add(b'/')      // Forward slash (prevents path traversal)
    .add(b'?')      // Question mark
    .add(b'#')      // Hash
    .add(b'+')      // Plus
    .add(b',')      // Comma
    .add(b';')      // Semicolon
    .add(b'[')      // Left square bracket
    .add(b']'); // Right square bracket

/// Percent-encode a string for safe use as a URL path segment.
///
/// This function should be used for any user-provided value that will be
/// interpolated into a URL path, including:
/// - User names
/// - Role names
/// - Index names
/// - Collection names
/// - App names
/// - Saved search names
/// - Job IDs (SIDs)
/// - Any other resource identifiers
///
/// # Examples
///
/// ```
/// use splunk_client::endpoints::url_encoding::encode_path_segment;
///
/// assert_eq!(encode_path_segment("simple"), "simple");
/// assert_eq!(encode_path_segment("user name"), "user%20name");
/// assert_eq!(encode_path_segment("user/name"), "user%2Fname");
/// assert_eq!(encode_path_segment("user%test"), "user%25test");
/// ```
pub fn encode_path_segment(segment: &str) -> String {
    percent_encode(segment.as_bytes(), PATH_SEGMENT_ENCODE_SET).to_string()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encode_simple() {
        assert_eq!(encode_path_segment("simple"), "simple");
        assert_eq!(encode_path_segment("user123"), "user123");
        assert_eq!(encode_path_segment("my_index"), "my_index");
    }

    #[test]
    fn test_encode_space() {
        assert_eq!(encode_path_segment("user name"), "user%20name");
        assert_eq!(encode_path_segment("two words"), "two%20words");
    }

    #[test]
    fn test_encode_slash() {
        // Critical: prevents path traversal
        assert_eq!(encode_path_segment("user/name"), "user%2Fname");
        assert_eq!(encode_path_segment("a/b/c"), "a%2Fb%2Fc");
    }

    #[test]
    fn test_encode_percent() {
        // Critical: prevents double-encoding issues
        assert_eq!(encode_path_segment("user%20name"), "user%2520name");
        assert_eq!(encode_path_segment("100%"), "100%25");
    }

    #[test]
    fn test_encode_unicode() {
        // Non-ASCII characters are percent-encoded as UTF-8 bytes
        assert_eq!(encode_path_segment("user\u{00e9}"), "user%C3%A9");
        assert_eq!(encode_path_segment("\u{2603}"), "%E2%98%83");
    }

    #[test]
    fn test_encode_special_chars() {
        assert_eq!(encode_path_segment("user{name}"), "user%7Bname%7D");
        assert_eq!(encode_path_segment("user[name]"), "user%5Bname%5D");
        assert_eq!(encode_path_segment("user+name"), "user%2Bname");
        assert_eq!(encode_path_segment("user,name"), "user%2Cname");
        assert_eq!(encode_path_segment("user;name"), "user%3Bname");
    }

    #[test]
    fn test_encode_question_and_hash() {
        assert_eq!(encode_path_segment("user?name"), "user%3Fname");
        assert_eq!(encode_path_segment("user#name"), "user%23name");
    }

    #[test]
    fn test_encode_quotes() {
        assert_eq!(encode_path_segment("user\"name"), "user%22name");
        assert_eq!(encode_path_segment("user'name"), "user'name"); // Single quote is OK
    }

    #[test]
    fn test_empty_string() {
        assert_eq!(encode_path_segment(""), "");
    }

    #[test]
    fn test_already_encoded() {
        // If something is already encoded, encoding again will double-encode the %
        assert_eq!(encode_path_segment("user%20name"), "user%2520name");
    }

    #[test]
    fn test_hyphen_underscore_dot() {
        // These characters should pass through unchanged
        assert_eq!(encode_path_segment("my-index"), "my-index");
        assert_eq!(encode_path_segment("my_index"), "my_index");
        assert_eq!(encode_path_segment("my.index"), "my.index");
    }

    #[test]
    fn test_colon() {
        // Colon is allowed in path segments (used in Splunk stanza patterns)
        assert_eq!(encode_path_segment("source::log"), "source::log");
    }
}
