//! Dashboards table formatter.
//!
//! Responsibilities:
//! - Format dashboard lists as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.
//! - Pagination (handled in mod.rs).

use anyhow::Result;
use splunk_client::models::Dashboard;

/// Format dashboards as a tab-separated table.
pub fn format_dashboards(dashboards: &[Dashboard], detailed: bool) -> Result<String> {
    let mut output = String::new();

    if dashboards.is_empty() {
        return Ok("No dashboards found.".to_string());
    }

    // Header
    if detailed {
        output.push_str("Name\tLabel\tAuthor\tVisible\tDescription\n");
    } else {
        output.push_str("Name\tLabel\tAuthor\n");
    }

    for dashboard in dashboards {
        let label = if dashboard.label.is_empty() {
            &dashboard.name
        } else {
            &dashboard.label
        };

        if detailed {
            let visible = if dashboard.is_visible { "Yes" } else { "No" };
            let description = dashboard.description.as_deref().unwrap_or("");
            output.push_str(&format!(
                "{}\t{}\t{}\t{}\t{}\n",
                dashboard.name, label, dashboard.author, visible, description
            ));
        } else {
            output.push_str(&format!(
                "{}\t{}\t{}\n",
                dashboard.name, label, dashboard.author
            ));
        }
    }

    Ok(output)
}

/// Format a single dashboard in detail.
pub fn format_dashboard(dashboard: &Dashboard) -> Result<String> {
    let mut output = String::new();

    let label = if dashboard.label.is_empty() {
        &dashboard.name
    } else {
        &dashboard.label
    };

    output.push_str(&format!("Name: {}\n", dashboard.name));
    output.push_str(&format!("Label: {}\n", label));
    output.push_str(&format!("Author: {}\n", dashboard.author));
    output.push_str(&format!(
        "Is Dashboard: {}\n",
        if dashboard.is_dashboard { "Yes" } else { "No" }
    ));
    output.push_str(&format!(
        "Is Visible: {}\n",
        if dashboard.is_visible { "Yes" } else { "No" }
    ));

    if let Some(ref version) = dashboard.version {
        output.push_str(&format!("Version: {}\n", version));
    }

    if let Some(ref updated) = dashboard.updated {
        output.push_str(&format!("Updated: {}\n", updated));
    }

    if let Some(ref description) = dashboard.description
        && !description.is_empty()
    {
        output.push_str(&format!("Description: {}\n", description));
    }

    if let Some(ref xml_data) = dashboard.xml_data
        && !xml_data.is_empty()
    {
        output.push_str("\n--- XML Definition ---\n");
        output.push_str(xml_data);
        output.push('\n');
    }

    Ok(output)
}
