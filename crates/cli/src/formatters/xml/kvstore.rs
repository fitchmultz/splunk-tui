//! KVStore XML formatter.
//!
//! Responsibilities:
//! - Format KVStore collections and records as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{escape_xml, flatten_kvstore_record};
use anyhow::Result;
use splunk_client::models::{KvStoreCollection, KvStoreRecord};

/// Format KVStore collections as XML.
pub fn format_kvstore_collections(collections: &[KvStoreCollection]) -> Result<String> {
    let mut output = String::new();
    output.push_str(
        r#"<?xml version="1.0" encoding="UTF-8"?>
"#,
    );
    output.push_str("<collections>\n");

    for collection in collections {
        output.push_str("  <collection>\n");
        output.push_str(&format!(
            "    <name>{}</name>\n",
            escape_xml(&collection.name)
        ));
        output.push_str(&format!("    <app>{}</app>\n", escape_xml(&collection.app)));
        output.push_str(&format!(
            "    <owner>{}</owner>\n",
            escape_xml(&collection.owner)
        ));
        output.push_str(&format!(
            "    <sharing>{}</sharing>\n",
            escape_xml(&collection.sharing)
        ));
        if let Some(disabled) = collection.disabled {
            output.push_str(&format!("    <disabled>{}</disabled>\n", disabled));
        }
        output.push_str("  </collection>\n");
    }

    output.push_str("</collections>\n");
    Ok(output)
}

/// Format KVStore collection records as XML.
pub fn format_kvstore_records(records: &[KvStoreRecord]) -> Result<String> {
    let mut output = String::new();
    output.push_str(
        r#"<?xml version="1.0" encoding="UTF-8"?>
"#,
    );
    output.push_str("<records>\n");

    for record in records {
        output.push_str("  <record>\n");

        for (key, value) in flatten_kvstore_record(record) {
            if key.starts_with('_') {
                output.push_str(&format!("    <{0}>{1}</{0}>\n", key, escape_xml(&value)));
            } else {
                output.push_str(&format!(
                    "    <field name=\"{}\">{}</field>\n",
                    escape_xml(&key),
                    escape_xml(&value)
                ));
            }
        }

        output.push_str("  </record>\n");
    }

    output.push_str("</records>\n");
    Ok(output)
}
