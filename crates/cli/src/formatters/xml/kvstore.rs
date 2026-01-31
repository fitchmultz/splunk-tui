//! KVStore XML formatter.
//!
//! Responsibilities:
//! - Format KVStore collections and records as XML.
//!
//! Does NOT handle:
//! - Other resource types.

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

        if let Some(key) = &record.key {
            output.push_str(&format!("    <_key>{}</_key>\n", escape_xml(key)));
        }
        if let Some(owner) = &record.owner {
            output.push_str(&format!("    <_owner>{}</_owner>\n", escape_xml(owner)));
        }
        if let Some(user) = &record.user {
            output.push_str(&format!("    <_user>{}</_user>\n", escape_xml(user)));
        }

        // Format the data fields
        if let serde_json::Value::Object(map) = &record.data {
            for (k, v) in map {
                let value_str = match v {
                    serde_json::Value::String(s) => escape_xml(s),
                    serde_json::Value::Number(n) => n.to_string(),
                    serde_json::Value::Bool(b) => b.to_string(),
                    serde_json::Value::Null => "null".to_string(),
                    _ => escape_xml(&v.to_string()),
                };
                output.push_str(&format!("    <{0}>{1}</{0}>\n", escape_xml(k), value_str));
            }
        }

        output.push_str("  </record>\n");
    }

    output.push_str("</records>\n");
    Ok(output)
}

fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
