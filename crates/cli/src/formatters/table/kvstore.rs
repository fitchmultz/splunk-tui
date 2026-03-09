//! KVStore table formatter.
//!
//! Responsibilities:
//! - Format KVStore collections and records as formatted text tables.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::flatten_kvstore_record;
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

        for (key, value) in flatten_kvstore_record(record) {
            output.push_str(&format!("{}: {}\n", key, value));
        }
    }

    Ok(output)
}
