//! Apps CSV formatter.
//!
//! This module previously contained manual CSV formatting functions for apps.
//! The `format_apps` function has been replaced by the `impl_csv_formatter!` macro
//! which uses the `ResourceDisplay` trait.
//!
//! The `format_app_info` function remains for detailed single-app formatting
//! until it can be migrated to use the `impl_csv_formatter_detailed!` macro.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_client::App;

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
