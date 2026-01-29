//! Search results XML formatter.
//!
//! Responsibilities:
//! - Format search results as XML with nested element structure.
//!
//! Does NOT handle:
//! - Other resource types.
//! - Flattened or tabular output.

use crate::formatters::common::escape_xml;
use anyhow::Result;

/// Format search results as XML.
pub fn format_search_results(results: &[serde_json::Value]) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<results>\n");

    for result in results {
        // Use nested XML structure instead of flat fields
        let nested = value_to_xml_elements("result", result, "  ");
        xml.push_str(&nested.join("\n"));
        xml.push('\n');
    }

    xml.push_str("</results>");
    Ok(xml)
}

/// Convert a JSON value to nested XML element(s).
///
/// Returns a vector of XML element strings. For primitive values, returns
/// a single element. For arrays and objects, returns multiple nested elements.
fn value_to_xml_elements(name: &str, value: &serde_json::Value, indent: &str) -> Vec<String> {
    match value {
        serde_json::Value::Null => {
            vec![format!(
                "{}<{}></{}>",
                indent,
                escape_xml(name),
                escape_xml(name)
            )]
        }
        serde_json::Value::Bool(b) => {
            vec![format!(
                "{}<{}>{}</{}>",
                indent,
                escape_xml(name),
                b,
                escape_xml(name)
            )]
        }
        serde_json::Value::Number(n) => {
            vec![format!(
                "{}<{}>{}</{}>",
                indent,
                escape_xml(name),
                n,
                escape_xml(name)
            )]
        }
        serde_json::Value::String(s) => {
            vec![format!(
                "{}<{}>{}</{}>",
                indent,
                escape_xml(name),
                escape_xml(s),
                escape_xml(name)
            )]
        }
        serde_json::Value::Array(arr) => {
            let mut elems = vec![format!("{}<{}>", indent, escape_xml(name))];
            for item in arr.iter() {
                let item_name = "item";
                elems.extend(value_to_xml_elements(
                    item_name,
                    item,
                    &format!("{}  ", indent),
                ));
            }
            elems.push(format!("{}</{}>", indent, escape_xml(name)));
            elems
        }
        serde_json::Value::Object(obj) => {
            let mut elems = vec![format!("{}<{}>", indent, escape_xml(name))];
            for (key, val) in obj {
                elems.extend(value_to_xml_elements(key, val, &format!("{}  ", indent)));
            }
            elems.push(format!("{}</{}>", indent, escape_xml(name)));
            elems
        }
    }
}
