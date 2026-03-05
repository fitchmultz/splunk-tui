//! Configs CSV formatter.
//!
//! Responsibilities:
//! - Format configuration files and stanzas as CSV.
//!
//! Does NOT handle:
//! - Other resource types.

use crate::formatters::common::{build_csv_header, build_csv_row, escape_csv, format_opt_str};
use anyhow::Result;
use splunk_client::models::{ConfigFile, ConfigStanza};

/// Format config files as CSV.
pub fn format_config_files(files: &[ConfigFile]) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&["Name", "Title", "Description"]));

    for file in files {
        output.push_str(&build_csv_row(&[
            escape_csv(&file.name),
            escape_csv(&file.title),
            format_opt_str(file.description.as_deref(), ""),
        ]));
    }

    Ok(output)
}

/// Format config stanzas as CSV.
pub fn format_config_stanzas(stanzas: &[ConfigStanza]) -> Result<String> {
    let mut output = String::new();

    // Header
    output.push_str(&build_csv_header(&["Config File", "Stanza Name"]));

    for stanza in stanzas {
        output.push_str(&build_csv_row(&[
            escape_csv(&stanza.config_file),
            escape_csv(&stanza.name),
        ]));
    }

    Ok(output)
}

/// Format single config stanza detail as CSV.
pub fn format_config_stanza_detail(stanza: &ConfigStanza) -> Result<String> {
    let mut output = String::new();

    output.push_str(&build_csv_header(&["Config File", &stanza.config_file]));
    output.push_str(&build_csv_row(&[
        escape_csv("Stanza Name"),
        escape_csv(&stanza.name),
    ]));

    output.push('\n');
    output.push_str(&build_csv_header(&["Setting", "Value"]));

    for (key, value) in &stanza.settings {
        output.push_str(&build_csv_row(&[
            escape_csv(key),
            escape_csv(&value.to_string()),
        ]));
    }

    Ok(output)
}
