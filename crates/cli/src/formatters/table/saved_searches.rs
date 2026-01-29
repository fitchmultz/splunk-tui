//! Saved searches table formatter.
//!
//! Responsibilities:
//! - Format saved search lists and details as formatted tables.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_client::SavedSearch;

/// Format saved searches as a formatted table.
pub fn format_saved_searches(searches: &[SavedSearch]) -> Result<String> {
    let mut output = String::new();

    if searches.is_empty() {
        output.push_str("No saved searches found.");
        return Ok(output);
    }

    output.push_str(&format!(
        "{:<30} {:<10} {:<40}\n",
        "NAME", "DISABLED", "DESCRIPTION"
    ));
    output.push_str(&format!(
        "{:<30} {:<10} {:<40}\n",
        "====", "========", "==========="
    ));

    for search in searches {
        let description = search.description.as_deref().unwrap_or("");
        let truncated_desc = if description.len() > 40 {
            format!("{}...", &description[..37])
        } else {
            description.to_string()
        };

        output.push_str(&format!(
            "{:<30} {:<10} {:<40}\n",
            search.name,
            if search.disabled { "Yes" } else { "No" },
            truncated_desc
        ));
    }

    Ok(output)
}

/// Format detailed saved search information.
pub fn format_saved_search_info(search: &SavedSearch) -> Result<String> {
    let mut output = String::new();

    output.push_str("--- Saved Search Information ---\n");
    output.push_str(&format!("Name: {}\n", search.name));
    output.push_str(&format!("Disabled: {}\n", search.disabled));
    output.push_str(&format!("Search Query:\n{}\n", search.search));
    if let Some(ref desc) = search.description {
        output.push_str(&format!("Description: {}\n", desc));
    }

    Ok(output)
}
