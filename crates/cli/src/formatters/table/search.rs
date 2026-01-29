//! Search results table formatter.
//!
//! Responsibilities:
//! - Format search results as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::format_json_value;
use anyhow::Result;

/// Format search results as a tab-separated table.
pub fn format_search_results(results: &[serde_json::Value]) -> Result<String> {
    if results.is_empty() {
        return Ok("No results found.".to_string());
    }

    let mut output = String::new();

    // Get all unique keys from all results
    let mut all_keys: Vec<String> = Vec::new();
    for result in results {
        if let Some(obj) = result.as_object() {
            for key in obj.keys() {
                if !all_keys.contains(key) {
                    all_keys.push(key.clone());
                }
            }
        }
    }

    // Sort keys for consistent output
    all_keys.sort();

    // Print header
    output.push_str(&all_keys.join("\t"));
    output.push('\n');

    // Print rows
    for result in results {
        if let Some(obj) = result.as_object() {
            let row: Vec<String> = all_keys
                .iter()
                .map(|key| obj.get(key).map(format_json_value).unwrap_or_default())
                .collect();
            output.push_str(&row.join("\t"));
            output.push('\n');
        }
    }

    Ok(output)
}
