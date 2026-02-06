//! Apps table formatter.
//!
//! This module previously contained manual table formatting functions for apps.
//! The `format_apps` function has been replaced by the `impl_table_formatter!` macro
//! which uses the `ResourceDisplay` trait.
//!
//! The `format_app_info` function remains for detailed single-app formatting
//! until it can be migrated to use the `impl_table_formatter_detailed!` macro.

use anyhow::Result;
use splunk_client::App;

/// Format detailed app information.
pub fn format_app_info(app: &App) -> Result<String> {
    let mut output = String::new();

    output.push_str("--- App Information ---\n");
    output.push_str(&format!("Name: {}\n", app.name));
    output.push_str(&format!(
        "Label: {}\n",
        app.label.as_deref().unwrap_or("N/A")
    ));
    output.push_str(&format!(
        "Version: {}\n",
        app.version.as_deref().unwrap_or("N/A")
    ));
    output.push_str(&format!("Disabled: {}\n", app.disabled));
    output.push_str(&format!(
        "Author: {}\n",
        app.author.as_deref().unwrap_or("N/A")
    ));
    if let Some(ref desc) = app.description {
        output.push_str(&format!("Description: {}\n", desc));
    }
    if let Some(configured) = app.is_configured {
        output.push_str(&format!("Configured: {}\n", configured));
    }
    if let Some(visible) = app.is_visible {
        output.push_str(&format!("Visible: {}\n", visible));
    }

    Ok(output)
}
