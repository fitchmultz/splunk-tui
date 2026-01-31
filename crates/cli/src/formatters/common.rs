//! Common utilities for formatters.
//!
//! Responsibilities:
//! - JSON flattening for CSV output.
//! - String escaping for CSV and XML.
//! - Atomic file writing.
//!
//! Does NOT handle:
//! - Format-specific logic (lives in respective formatter modules).
//! - Direct output formatting.

use anyhow::{Context, Result};
use std::collections::BTreeMap;

/// Flatten a JSON object into a map of dot-notation keys to string values.
///
/// # Arguments
/// * `value` - The JSON value to flatten
/// * `prefix` - The current key prefix (for nested recursion)
/// * `output` - The output map to populate
///
/// # Flattening Rules
/// - Primitive values (string, number, bool, null): stored as-is with string conversion
/// - Nested objects: keys are prefixed with parent key and dot (e.g., `user.name`)
/// - Arrays: each element gets indexed key (e.g., `tags.0`, `tags.1`)
/// - Nested arrays within objects: combined notation (e.g., `users.0.name`)
pub fn flatten_json_object(
    value: &serde_json::Value,
    prefix: &str,
    output: &mut BTreeMap<String, String>,
) {
    match value {
        serde_json::Value::Null => {
            output.insert(prefix.to_string(), String::new());
        }
        serde_json::Value::Bool(b) => {
            output.insert(prefix.to_string(), b.to_string());
        }
        serde_json::Value::Number(n) => {
            output.insert(prefix.to_string(), n.to_string());
        }
        serde_json::Value::String(s) => {
            output.insert(prefix.to_string(), s.clone());
        }
        serde_json::Value::Array(arr) => {
            for (i, item) in arr.iter().enumerate() {
                let new_key = format!("{}.{}", prefix, i);
                flatten_json_object(item, &new_key, output);
            }
        }
        serde_json::Value::Object(obj) => {
            for (key, val) in obj {
                let new_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };
                flatten_json_object(val, &new_key, output);
            }
        }
    }
}

/// Extract all flattened keys from a slice of JSON results.
///
/// Returns a sorted list of all unique dot-notation keys across all results.
pub fn get_all_flattened_keys(results: &[serde_json::Value]) -> Vec<String> {
    let mut all_keys: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
    for result in results {
        let mut flat = BTreeMap::new();
        flatten_json_object(result, "", &mut flat);
        all_keys.extend(flat.into_keys());
    }
    all_keys.into_iter().collect()
}

/// Format a JSON value as a string for display.
///
/// Converts any JSON value to its string representation:
/// - Strings are returned as-is
/// - Numbers and booleans are converted to their string representation
/// - Null values become empty strings
/// - Arrays and objects are serialized as compact JSON
pub fn format_json_value(v: &serde_json::Value) -> String {
    match v {
        serde_json::Value::String(s) => s.clone(),
        serde_json::Value::Number(n) => n.to_string(),
        serde_json::Value::Bool(b) => b.to_string(),
        serde_json::Value::Null => String::new(),
        serde_json::Value::Array(_) | serde_json::Value::Object(_) => {
            // Serialize arrays/objects as compact JSON
            serde_json::to_string(v).unwrap_or_default()
        }
    }
}

/// Escape a string value for CSV output according to RFC 4180.
///
/// Rules:
/// - Wrap in double quotes if the field contains comma, double quote, or newline
/// - Double any internal double quotes (e.g., `"hello"` -> `""hello""`)
pub fn escape_csv(s: &str) -> String {
    let needs_quoting = s.contains(',') || s.contains('"') || s.contains('\n') || s.contains('\r');
    if !needs_quoting {
        return s.to_string();
    }
    // Double all quotes and wrap in quotes
    format!("\"{}\"", s.replace('"', "\"\""))
}

/// Build a CSV header row from field names.
///
/// Escapes each field name and joins with commas, appending a newline.
pub fn build_csv_header(fields: &[&str]) -> String {
    let escaped: Vec<String> = fields.iter().map(|f| escape_csv(f)).collect();
    format!("{}\n", escaped.join(","))
}

/// Build a CSV data row from field values.
///
/// Escapes each value and joins with commas, appending a newline.
pub fn build_csv_row(values: &[String]) -> String {
    let escaped: Vec<String> = values.iter().map(|v| escape_csv(v)).collect();
    format!("{}\n", escaped.join(","))
}

/// Format an optional string for CSV output.
///
/// Returns the escaped value if present, otherwise returns the default.
pub fn format_opt_str(opt: Option<&str>, default: &str) -> String {
    escape_csv(opt.unwrap_or(default))
}

/// Escape special XML characters.
pub fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}

/// Write formatted output to a file atomically.
///
/// Creates parent directories if needed, writes to temp file then renames
/// for atomicity. Returns error with helpful context on failure.
pub fn write_to_file(content: &str, path: &std::path::Path) -> Result<()> {
    use std::fs;
    use std::io::Write;
    use tempfile::NamedTempFile;

    // Get parent directory for temp file creation
    // If path has no parent (e.g., just "results.json"), use current directory
    let parent_dir = path.parent().unwrap_or_else(|| std::path::Path::new("."));

    // Create parent directories if they don't exist
    if !parent_dir.as_os_str().is_empty() && parent_dir != std::path::Path::new(".") {
        fs::create_dir_all(parent_dir)
            .with_context(|| format!("Failed to create directory: {}", parent_dir.display()))?;
    }

    // Write to temp file first for atomicity
    let mut temp_file = NamedTempFile::new_in(parent_dir)
        .with_context(|| format!("Failed to create temp file in: {}", parent_dir.display()))?;

    temp_file
        .write_all(content.as_bytes())
        .with_context(|| "Failed to write to temp file")?;
    temp_file
        .flush()
        .with_context(|| "Failed to flush temp file")?;

    // Atomic rename
    temp_file
        .persist(path)
        .with_context(|| format!("Failed to write file: {}", path.display()))?;

    Ok(())
}
