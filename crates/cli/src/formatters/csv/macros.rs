//! Macros CSV formatter.
//!
//! Responsibilities:
//! - Format macro lists and details as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_client::Macro;

/// Format macros as CSV.
pub fn format_macros(macros: &[Macro]) -> Result<String> {
    let mut output = String::new();

    output.push_str(&build_csv_header(&[
        "name",
        "definition",
        "args",
        "description",
        "disabled",
        "iseval",
        "validation",
        "errormsg",
    ]));

    for macro_item in macros {
        output.push_str(&build_csv_row(&[
            escape_csv(&macro_item.name),
            escape_csv(&macro_item.definition),
            format_opt_str(macro_item.args.as_deref(), ""),
            format_opt_str(macro_item.description.as_deref(), ""),
            escape_csv(&macro_item.disabled.to_string()),
            escape_csv(&macro_item.iseval.to_string()),
            format_opt_str(macro_item.validation.as_deref(), ""),
            format_opt_str(macro_item.errormsg.as_deref(), ""),
        ]));
    }

    Ok(output)
}

/// Format detailed macro information as CSV.
pub fn format_macro_info(macro_info: &Macro) -> Result<String> {
    let mut output = String::new();

    output.push_str(&build_csv_header(&[
        "name",
        "definition",
        "args",
        "description",
        "disabled",
        "iseval",
        "validation",
        "errormsg",
    ]));

    output.push_str(&build_csv_row(&[
        escape_csv(&macro_info.name),
        escape_csv(&macro_info.definition),
        format_opt_str(macro_info.args.as_deref(), ""),
        format_opt_str(macro_info.description.as_deref(), ""),
        escape_csv(&macro_info.disabled.to_string()),
        escape_csv(&macro_info.iseval.to_string()),
        format_opt_str(macro_info.validation.as_deref(), ""),
        format_opt_str(macro_info.errormsg.as_deref(), ""),
    ]));

    Ok(output)
}
