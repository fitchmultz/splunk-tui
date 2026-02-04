//! Data models CSV formatter.
//!
//! Responsibilities:
//! - Format data model lists as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv};
use anyhow::Result;
use splunk_client::models::DataModel;

/// Format data models as CSV.
pub fn format_datamodels(datamodels: &[DataModel], detailed: bool) -> Result<String> {
    let mut output = String::new();

    if detailed {
        output.push_str(&build_csv_header(&[
            "Name",
            "Display Name",
            "Owner",
            "App",
            "Accelerated",
            "Description",
            "Updated",
        ]));

        for datamodel in datamodels {
            output.push_str(&build_csv_row(&[
                escape_csv(&datamodel.name),
                escape_csv(&datamodel.displayName),
                escape_csv(&datamodel.owner),
                escape_csv(&datamodel.app),
                escape_csv(&datamodel.is_accelerated.to_string()),
                escape_csv(datamodel.description.as_deref().unwrap_or("")),
                escape_csv(datamodel.updated.as_deref().unwrap_or("")),
            ]));
        }
    } else {
        output.push_str(&build_csv_header(&["Name", "Display Name", "Owner", "App"]));

        for datamodel in datamodels {
            output.push_str(&build_csv_row(&[
                escape_csv(&datamodel.name),
                escape_csv(&datamodel.displayName),
                escape_csv(&datamodel.owner),
                escape_csv(&datamodel.app),
            ]));
        }
    }

    Ok(output)
}

/// Format a single data model in detail.
pub fn format_datamodel(datamodel: &DataModel) -> Result<String> {
    let mut output = String::new();

    output.push_str(&build_csv_header(&[
        "Name",
        "Display Name",
        "Owner",
        "App",
        "Accelerated",
        "Description",
        "Updated",
        "JSON Data",
    ]));

    output.push_str(&build_csv_row(&[
        escape_csv(&datamodel.name),
        escape_csv(&datamodel.displayName),
        escape_csv(&datamodel.owner),
        escape_csv(&datamodel.app),
        escape_csv(&datamodel.is_accelerated.to_string()),
        escape_csv(datamodel.description.as_deref().unwrap_or("")),
        escape_csv(datamodel.updated.as_deref().unwrap_or("")),
        escape_csv(datamodel.json_data.as_deref().unwrap_or("")),
    ]));

    Ok(output)
}
