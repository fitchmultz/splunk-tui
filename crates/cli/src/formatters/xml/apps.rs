//! Apps XML formatter.
//!
//! Responsibilities:
//! - Format app lists and app info as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::escape_xml;
use anyhow::Result;
use splunk_client::App;

/// Format apps as XML.
pub fn format_apps(apps: &[App]) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<apps>\n");

    for app in apps {
        xml.push_str("  <app>\n");
        xml.push_str(&format!("    <name>{}</name>\n", escape_xml(&app.name)));

        if let Some(ref label) = app.label {
            xml.push_str(&format!("    <label>{}</label>\n", escape_xml(label)));
        }

        if let Some(ref version) = app.version {
            xml.push_str(&format!("    <version>{}</version>\n", escape_xml(version)));
        }

        xml.push_str(&format!("    <disabled>{}</disabled>\n", app.disabled));

        if let Some(ref author) = app.author {
            xml.push_str(&format!("    <author>{}</author>\n", escape_xml(author)));
        }

        xml.push_str("  </app>\n");
    }

    xml.push_str("</apps>");
    Ok(xml)
}

/// Format detailed app information as XML.
pub fn format_app_info(app: &App) -> Result<String> {
    let mut xml = String::from("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n<app>\n");

    xml.push_str(&format!("  <name>{}</name>\n", escape_xml(&app.name)));

    if let Some(ref label) = app.label {
        xml.push_str(&format!("  <label>{}</label>\n", escape_xml(label)));
    }

    if let Some(ref version) = app.version {
        xml.push_str(&format!("  <version>{}</version>\n", escape_xml(version)));
    }

    xml.push_str(&format!("  <disabled>{}</disabled>\n", app.disabled));

    if let Some(ref author) = app.author {
        xml.push_str(&format!("  <author>{}</author>\n", escape_xml(author)));
    }

    if let Some(ref desc) = app.description {
        xml.push_str(&format!(
            "  <description>{}</description>\n",
            escape_xml(desc)
        ));
    }

    if let Some(configured) = app.is_configured {
        xml.push_str(&format!(
            "  <is_configured>{}</is_configured>\n",
            configured
        ));
    }

    if let Some(visible) = app.is_visible {
        xml.push_str(&format!("  <is_visible>{}</is_visible>\n", visible));
    }

    xml.push_str("</app>");
    Ok(xml)
}
