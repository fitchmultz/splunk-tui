//! KVStore table formatter.
//!
//! Responsibilities:
//! - Format KVStore collections and records as formatted text tables.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_client::models::{KvStoreCollection, KvStoreRecord};

/// Format KVStore collections as a table.
pub fn format_kvstore_collections(collections: &[KvStoreCollection]) -> Result<String> {
    let mut output = String::new();

    if collections.is_empty() {
        output.push_str("No KVStore collections found.\n");
        return Ok(output);
    }

    // Header
    output.push_str("Name\tApp\tOwner\tSharing\tDisabled\n");

    for collection in collections {
        let disabled = collection
            .disabled
            .map_or("N/A", |d| if d { "true" } else { "false" });
        output.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\n",
            collection.name, collection.app, collection.owner, collection.sharing, disabled
        ));
    }

    Ok(output)
}

/// Format KVStore collection records as a table.
pub fn format_kvstore_records(records: &[KvStoreRecord]) -> Result<String> {
    let mut output = String::new();

    if records.is_empty() {
        output.push_str("No records found in collection.\n");
        return Ok(output);
    }

    // For records, we use JSON-like formatting since the schema is dynamic
    for (i, record) in records.iter().enumerate() {
        if i > 0 {
            output.push_str("---\n");
        }

        if let Some(key) = &record.key {
            output.push_str(&format!("_key: {}\n", key));
        }
        if let Some(owner) = &record.owner {
            output.push_str(&format!("_owner: {}\n", owner));
        }
        if let Some(user) = &record.user {
            output.push_str(&format!("_user: {}\n", user));
        }

        // Format the data fields
        if let serde_json::Value::Object(map) = &record.data {
            for (k, v) in map {
                let value_str = match v {
                    serde_json::Value::String(s) => s.clone(),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "null".to_string(),
                    _ => v.to_string(),
                };
                output.push_str(&format!("{}: {}\n", k, value_str));
            }
        }
    }

    Ok(output)
}
