//! Configs XML formatter.
//!
//! Responsibilities:
//! - Format configuration files and stanzas as XML.
//!
//! Does NOT handle:
//! - Other resource types.

use anyhow::Result;
use splunk_client::models::{ConfigFile, ConfigStanza};

/// Format config files as XML.
pub fn format_config_files(files: &[ConfigFile]) -> Result<String> {
    let mut output = String::new();
    output.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    output.push('\n');
    output.push_str("<config_files>\n");

    for file in files {
        output.push_str("  <config_file>\n");
        output.push_str(&format!("    <name>{}</name>\n", escape_xml(&file.name)));
        output.push_str(&format!("    <title>{}</title>\n", escape_xml(&file.title)));
        if let Some(ref desc) = file.description {
            output.push_str(&format!(
                "    <description>{}</description>\n",
                escape_xml(desc)
            ));
        }
        output.push_str("  </config_file>\n");
    }

    output.push_str("</config_files>\n");
    Ok(output)
}

/// Format config stanzas as XML.
pub fn format_config_stanzas(stanzas: &[ConfigStanza]) -> Result<String> {
    let mut output = String::new();
    output.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    output.push('\n');
    output.push_str("<config_stanzas>\n");

    for stanza in stanzas {
        output.push_str("  <config_stanza>\n");
        output.push_str(&format!(
            "    <config_file>{}</config_file>\n",
            escape_xml(&stanza.config_file)
        ));
        output.push_str(&format!("    <name>{}</name>\n", escape_xml(&stanza.name)));
        output.push_str("  </config_stanza>\n");
    }

    output.push_str("</config_stanzas>\n");
    Ok(output)
}

/// Format a single config stanza as XML.
pub fn format_config_stanza(stanza: &ConfigStanza) -> Result<String> {
    let mut output = String::new();
    output.push_str(r#"<?xml version="1.0" encoding="UTF-8"?>"#);
    output.push('\n');
    output.push_str("<config_stanza>\n");
    output.push_str(&format!(
        "  <config_file>{}</config_file>\n",
        escape_xml(&stanza.config_file)
    ));
    output.push_str(&format!("  <name>{}</name>\n", escape_xml(&stanza.name)));

    if !stanza.settings.is_empty() {
        output.push_str("  <settings>\n");
        for (key, value) in &stanza.settings {
            output.push_str(&format!(
                r#"    <setting name="{}">{}</setting>"#,
                escape_xml(key),
                escape_xml(&value.to_string())
            ));
            output.push('\n');
        }
        output.push_str("  </settings>\n");
    }

    output.push_str("</config_stanza>\n");
    Ok(output)
}

/// Escape special XML characters.
fn escape_xml(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
