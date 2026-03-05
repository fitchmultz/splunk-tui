//! Dashboards CSV formatter.
//!
//! Responsibilities:
//! - Format dashboard lists as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv};
use anyhow::Result;
use splunk_client::models::Dashboard;

/// Format dashboards as CSV.
pub fn format_dashboards(dashboards: &[Dashboard], detailed: bool) -> Result<String> {
    let mut output = String::new();

    if detailed {
        output.push_str(&build_csv_header(&[
            "Name",
            "Label",
            "Author",
            "Is Dashboard",
            "Is Visible",
            "Description",
            "Version",
            "Updated",
        ]));

        for dashboard in dashboards {
            output.push_str(&build_csv_row(&[
                escape_csv(&dashboard.name),
                escape_csv(&dashboard.label),
                escape_csv(&dashboard.author),
                escape_csv(&dashboard.is_dashboard.to_string()),
                escape_csv(&dashboard.is_visible.to_string()),
                escape_csv(dashboard.description.as_deref().unwrap_or("N/A")),
                escape_csv(dashboard.version.as_deref().unwrap_or("N/A")),
                escape_csv(dashboard.updated.as_deref().unwrap_or("N/A")),
            ]));
        }
    } else {
        output.push_str(&build_csv_header(&["Name", "Label", "Author"]));

        for dashboard in dashboards {
            output.push_str(&build_csv_row(&[
                escape_csv(&dashboard.name),
                escape_csv(&dashboard.label),
                escape_csv(&dashboard.author),
            ]));
        }
    }

    Ok(output)
}

/// Format a single dashboard in detail.
pub fn format_dashboard(dashboard: &Dashboard) -> Result<String> {
    let mut output = String::new();

    output.push_str(&build_csv_header(&[
        "Name",
        "Label",
        "Author",
        "Is Dashboard",
        "Is Visible",
        "Description",
        "Version",
        "Updated",
        "XML Data",
    ]));

    output.push_str(&build_csv_row(&[
        escape_csv(&dashboard.name),
        escape_csv(&dashboard.label),
        escape_csv(&dashboard.author),
        escape_csv(&dashboard.is_dashboard.to_string()),
        escape_csv(&dashboard.is_visible.to_string()),
        escape_csv(dashboard.description.as_deref().unwrap_or("")),
        escape_csv(dashboard.version.as_deref().unwrap_or("")),
        escape_csv(dashboard.updated.as_deref().unwrap_or("N/A")),
        escape_csv(dashboard.xml_data.as_deref().unwrap_or("N/A")),
    ]));

    Ok(output)
}
