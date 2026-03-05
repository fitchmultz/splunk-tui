//! Saved searches XML formatter.
//!
//! Responsibilities:
//! - Format saved searches list and details as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_client::SavedSearch;

/// Format saved searches as XML.
pub fn format_saved_searches(searches: &[SavedSearch]) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<saved-searches>\n");

    for search in searches {
        xml.push_str("  <saved-search>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&search.name)));
        xml.push_str(&format!("    <disabled>{}</disabled>\n", search.disabled));
        if let Some(ref desc) = search.description {
            xml.push_str(&format!(
                "    <description>{}</description>\n",
                escape_xml(desc)
            ));
        }
        xml.push_str("  </saved-search>\n");
    }

    xml.push_str("</saved-searches>");
    Ok(xml)
}

/// Format detailed saved search information as XML.
pub fn format_saved_search_info(search: &SavedSearch) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<saved-search>\n");

    xml.push_str(&format!("  <name>{}</name>\n", escape_xml(&search.name)));
    xml.push_str(&format!("  <disabled>{}</disabled>\n", search.disabled));
    if let Some(ref desc) = search.description {
        xml.push_str(&format!(
            "  <description>{}</description>\n",
            escape_xml(desc)
        ));
    }
    xml.push_str(&format!(
        "  <search>{}</search>\n",
        escape_xml(&search.search)
    ));
    xml.push_str("</saved-search>");
    Ok(xml)
}
