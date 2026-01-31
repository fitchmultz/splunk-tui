//! Apps CSV formatter.
//!
//! Responsibilities:
//! - Format app lists as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_client::App;

/// Format apps as CSV.
pub fn format_apps(apps: &[App]) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "name", "label", "version", "disabled", "author",
    ]));

    for app in apps {
        output.push_str(&build_csv_row(&[
            escape_csv(&app.name),
            format_opt_str(app.label.as_deref(), ""),
            format_opt_str(app.version.as_deref(), ""),
            escape_csv(&app.disabled.to_string()),
            format_opt_str(app.author.as_deref(), ""),
        ]));
    }

    Ok(output)
}

/// Format detailed app info as CSV.
pub fn format_app_info(app: &App) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&[
        "name",
        "label",
        "version",
        "disabled",
        "author",
        "description",
    ]));

    output.push_str(&build_csv_row(&[
        escape_csv(&app.name),
        format_opt_str(app.label.as_deref(), ""),
        format_opt_str(app.version.as_deref(), ""),
        escape_csv(&app.disabled.to_string()),
        format_opt_str(app.author.as_deref(), ""),
        format_opt_str(app.description.as_deref(), ""),
    ]));

    Ok(output)
}
