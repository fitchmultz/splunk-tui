//! Apps table formatter.
//!
//! Responsibilities:
//! - Format app lists and app details as formatted tables.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_client::App;

/// Format apps as a formatted table.
pub fn format_apps(apps: &[App]) -> Result<String> {
    let mut output = String::new();

    if apps.is_empty() {
        output.push_str("No apps found.");
        return Ok(output);
    }

    // Header
    output.push_str(&format!(
        "{:<25} {:<20} {:<10} {:<10} {:<20}\n",
        "NAME", "LABEL", "VERSION", "DISABLED", "AUTHOR"
    ));
    output.push_str(&format!(
        "{:<25} {:<20} {:<10} {:<10} {:<20}\n",
        "=====", "=====", "=======", "========", "======="
    ));

    // Rows
    for app in apps {
        let label = app.label.as_deref().unwrap_or("-");
        let version = app.version.as_deref().unwrap_or("-");
        let author = app.author.as_deref().unwrap_or("-");

        output.push_str(&format!(
            "{:<25} {:<20} {:<10} {:<10} {:<20}\n",
            app.name, label, version, app.disabled, author
        ));
    }

    Ok(output)
}

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
