//! Configs table formatter.
//!
//! Responsibilities:
//! - Format configuration files and stanzas as tab-separated tables.
//!
//! Does NOT handle:
//! - Other resource types.
//! - Pagination (handled in imp.rs).

use anyhow::Result;
use splunk_client::models::{ConfigFile, ConfigStanza};

/// Format config files as a tab-separated table.
pub fn format_config_files(files: &[ConfigFile]) -> Result<String> {
    let mut output = String::new();

    if files.is_empty() {
        return Ok("No config files found.".to_string());
    }

    // Header
    output.push_str("Name\tTitle\tDescription\n");

    for file in files {
        let name = &file.name;
        let title = &file.title;
        let description = file.description.as_deref().unwrap_or("N/A");
        output.push_str(&format!("{}\t{}\t{}\n", name, title, description));
    }

    Ok(output)
}

/// Format config stanzas as a tab-separated table.
pub fn format_config_stanzas(stanzas: &[ConfigStanza]) -> Result<String> {
    let mut output = String::new();

    if stanzas.is_empty() {
        return Ok("No config stanzas found.".to_string());
    }

    // Header
    output.push_str("Config File\tStanza Name\tSettings Preview\n");

    for stanza in stanzas {
        let config_file = &stanza.config_file;
        let name = &stanza.name;
        // Show a preview of the first few settings
        let settings_preview: String = stanza
            .settings
            .iter()
            .take(3)
            .map(|(k, v)| format!("{}={}", k, v))
            .collect::<Vec<_>>()
            .join(", ");
        let preview = if settings_preview.len() > 60 {
            format!("{}...", &settings_preview[..57])
        } else {
            settings_preview
        };
        let preview_display = if preview.is_empty() {
            "(no settings)".to_string()
        } else {
            preview
        };
        output.push_str(&format!("{}\t{}\t{}\n", config_file, name, preview_display));
    }

    Ok(output)
}

/// Format a single config stanza in detail.
pub fn format_config_stanza_detail(stanza: &ConfigStanza) -> Result<String> {
    let mut output = String::new();

    output.push_str(&format!("Config File: {}\n", stanza.config_file));
    output.push_str(&format!("Stanza Name: {}\n", stanza.name));
    output.push_str("\nSettings:\n");

    if stanza.settings.is_empty() {
        output.push_str("  (no settings)\n");
    } else {
        for (key, value) in &stanza.settings {
            output.push_str(&format!("  {} = {}\n", key, value));
        }
    }

    Ok(output)
}
