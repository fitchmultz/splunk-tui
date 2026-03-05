//! Saved Searches CSV formatter.
//!
//! Responsibilities:
//! - Format saved searches as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_client::SavedSearch;

/// Format saved searches list as CSV.
pub fn format_saved_searches(searches: &[SavedSearch]) -> Result<String> {
    let mut output = String::new();

    output.push_str(&build_csv_header(&["name", "disabled", "description"]));

    for search in searches {
        output.push_str(&build_csv_row(&[
            escape_csv(&search.name),
            escape_csv(&search.disabled.to_string()),
            format_opt_str(search.description.as_deref(), ""),
        ]));
    }

    Ok(output)
}

/// Format detailed saved search info as CSV.
pub fn format_saved_search_info(search: &SavedSearch) -> Result<String> {
    let mut output = String::new();

    output.push_str(&build_csv_header(&[
        "name",
        "disabled",
        "search",
        "description",
    ]));

    output.push_str(&build_csv_row(&[
        escape_csv(&search.name),
        escape_csv(&search.disabled.to_string()),
        escape_csv(&search.search),
        format_opt_str(search.description.as_deref(), ""),
    ]));

    Ok(output)
}
