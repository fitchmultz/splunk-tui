//! KVStore CSV formatter.
//!
//! Responsibilities:
//! - Format KV store collections as CSV.
//!
//! Does NOT handle:
//! - Other resource types.
//! - KV store records (handled in search.rs).

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv};
use anyhow::Result;
use splunk_client::models::KvStoreCollection;

/// Format KV store collections as CSV.
pub fn format_kvstore_collections(collections: &[KvStoreCollection]) -> Result<String> {
    if collections.is_empty() {
        return Ok(String::new());
    }

    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "name", "app", "owner", "sharing", "disabled",
    ]));

    for collection in collections {
        let disabled = collection
            .disabled
            .map_or("", |d| if d { "true" } else { "false" });
        output.push_str(&build_csv_row(&[
            escape_csv(&collection.name),
            escape_csv(&collection.app),
            escape_csv(&collection.owner),
            escape_csv(&collection.sharing),
            escape_csv(disabled),
        ]));
    }

    Ok(output)
}
