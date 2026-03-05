//! Data models table formatter.
//!
//! Responsibilities:
//! - Format data model lists as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.
//! - Pagination (handled in mod.rs).

use anyhow::Result;
use splunk_client::models::DataModel;

/// Format data models as a tab-separated table.
pub fn format_datamodels(datamodels: &[DataModel], detailed: bool) -> Result<String> {
    let mut output = String::new();

    if datamodels.is_empty() {
        return Ok("No data models found.".to_string());
    }

    // Header
    if detailed {
        output.push_str("Name\tDisplay Name\tOwner\tApp\tAccelerated\tDescription\n");
    } else {
        output.push_str("Name\tDisplay Name\tOwner\tApp\n");
    }

    for datamodel in datamodels {
        let display_name = if datamodel.displayName.is_empty() {
            &datamodel.name
        } else {
            &datamodel.displayName
        };

        if detailed {
            let accelerated = if datamodel.is_accelerated {
                "Yes"
            } else {
                "No"
            };
            let description = datamodel.description.as_deref().unwrap_or("");
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\t{}\n",
                datamodel.name,
                display_name,
                datamodel.owner,
                datamodel.app,
                accelerated,
                description
            ));
        } else {
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\n",
                datamodel.name, display_name, datamodel.owner, datamodel.app
            ));
        }
    }

    Ok(output)
}

/// Format a single data model in detail.
pub fn format_datamodel(datamodel: &DataModel) -> Result<String> {
    let mut output = String::new();

    let display_name = if datamodel.displayName.is_empty() {
        &datamodel.name
    } else {
        &datamodel.displayName
    };

    output.push_str(&format!("Name: {}\n", datamodel.name));
    output.push_str(&format!("Display Name: {}\n", display_name));
    output.push_str(&format!("Owner: {}\n", datamodel.owner));
    output.push_str(&format!("App: {}\n", datamodel.app));
    output.push_str(&format!(
        "Accelerated: {}\n",
        if datamodel.is_accelerated {
            "Yes"
        } else {
            "No"
        }
    ));

    if let Some(ref updated) = datamodel.updated {
        output.push_str(&format!("Updated: {}\n", updated));
    }

    if let Some(ref description) = datamodel.description
        && !description.is_empty()
    {
        output.push_str(&format!("Description: {}\n", description));
    }

    if let Some(ref json_data) = datamodel.json_data
        && !json_data.is_empty()
    {
        output.push_str("\n--- JSON Definition ---\n");
        output.push_str(json_data);
        output.push('\n');
    }

    Ok(output)
}
