//! Macros table formatter.
//!
//! Responsibilities:
//! - Format macro lists and details as formatted tables.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_client::Macro;

/// Format macros as a formatted table.
pub fn format_macros(macros: &[Macro]) -> Result<String> {
    let mut output = String::new();

    if macros.is_empty() {
        output.push_str("No macros found.");
        return Ok(output);
    }

    output.push_str(&format!(
        "{:<30} {:<10} {:<8} {:<40}\n",
        "NAME", "DISABLED", "EVAL", "DESCRIPTION"
    ));
    output.push_str(&format!(
        "{:<30} {:<10} {:<8} {:<40}\n",
        "====", "========", "====", "==========="
    ));

    for macro_item in macros {
        let description = macro_item.description.as_deref().unwrap_or("");
        let truncated_desc = if description.len() > 40 {
            format!("{}...", &description[..37])
        } else {
            description.to_string()
        };

        output.push_str(&format!(
            "{:<30} {:<10} {:<8} {:<40}\n",
            macro_item.name,
            if macro_item.disabled { "Yes" } else { "No" },
            if macro_item.iseval { "Yes" } else { "No" },
            truncated_desc
        ));
    }

    Ok(output)
}

/// Format detailed macro information.
pub fn format_macro_info(macro_info: &Macro) -> Result<String> {
    let mut output = String::new();

    output.push_str("--- Macro Information ---\n");
    output.push_str(&format!("Name: {}\n", macro_info.name));
    output.push_str(&format!(
        "Disabled: {}\n",
        if macro_info.disabled { "Yes" } else { "No" }
    ));
    output.push_str(&format!(
        "Eval Expression: {}\n",
        if macro_info.iseval { "Yes" } else { "No" }
    ));
    if let Some(ref args) = macro_info.args {
        output.push_str(&format!("Arguments: {}\n", args));
    }
    if let Some(ref desc) = macro_info.description {
        output.push_str(&format!("Description: {}\n", desc));
    }
    output.push_str(&format!("Definition:\n{}\n", macro_info.definition));
    if let Some(ref validation) = macro_info.validation {
        output.push_str(&format!("Validation: {}\n", validation));
    }
    if let Some(ref errormsg) = macro_info.errormsg {
        output.push_str(&format!("Error Message: {}\n", errormsg));
    }

    Ok(output)
}
