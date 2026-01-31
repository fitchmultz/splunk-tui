//! Search results CSV formatter.
//!
//! Responsibilities:
//! - Format search results and KV store records as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{escape_csv, flatten_json_object, get_all_flattened_keys};
use anyhow::Result;
use splunk_client::models::KvStoreRecord;

/// Format search results as CSV with flattened JSON.
pub fn format_search_results(results: &[serde_json::Value]) -> Result<String> {
    if results.is_empty() {
        return Ok(String::new());
    }

    let mut output = String::new();

    // Get all unique flattened keys from all results (sorted)
    let all_keys = get_all_flattened_keys(results);

    // Print header (escaped)
    let header: Vec<String> = all_keys.iter().map(|k| escape_csv(k)).collect();
    output.push_str(&header.join(","));
    output.push('\n');

    // Print rows with flattened values
    for result in results {
        let mut flat = std::collections::BTreeMap::new();
        flatten_json_object(result, "", &mut flat);

        let row: Vec<String> = all_keys
            .iter()
            .map(|key| {
                let value = flat.get(key).cloned().unwrap_or_default();
                escape_csv(&value)
            })
            .collect();
        output.push_str(&row.join(","));
        output.push('\n');
    }

    Ok(output)
}

/// Format KV store records as CSV.
pub fn format_kvstore_records(records: &[KvStoreRecord]) -> Result<String> {
    if records.is_empty() {
        return Ok(String::new());
    }

    let mut output = String::new();

    // Get all unique flattened keys from all records (sorted)
    let all_keys =
        get_all_flattened_keys(&records.iter().map(|r| r.data.clone()).collect::<Vec<_>>());

    // Print header with _key, _owner, _user plus data fields
    let mut headers = vec![
        "_key".to_string(),
        "_owner".to_string(),
        "_user".to_string(),
    ];
    headers.extend(all_keys.clone());
    let header: Vec<String> = headers.iter().map(|k| escape_csv(k)).collect();
    output.push_str(&header.join(","));
    output.push('\n');

    // Print rows
    for record in records {
        let mut flat = std::collections::BTreeMap::new();
        flatten_json_object(&record.data, "", &mut flat);

        let mut row = vec![
            escape_csv(record.key.as_deref().unwrap_or("")),
            escape_csv(record.owner.as_deref().unwrap_or("")),
            escape_csv(record.user.as_deref().unwrap_or("")),
        ];

        for key in &all_keys {
            let value = flat.get(key).cloned().unwrap_or_default();
            row.push(escape_csv(&value));
        }

        output.push_str(&row.join(","));
        output.push('\n');
    }

    Ok(output)
}
