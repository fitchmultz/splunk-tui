//! Configuration file models for Splunk configuration files (props.conf, transforms.conf, etc.).
//!
//! This module contains types for listing and viewing Splunk configuration files.
//!
//! # What this module handles:
//! - Deserialization of configuration stanza data from Splunk REST API
//! - Type-safe representation of configuration stanzas and their settings
//!
//! # What this module does NOT handle:
//! - Direct HTTP API calls (see [`crate::endpoints::configs`])
//! - Client-side filtering or searching of configurations
//!
//! Splunk configs API endpoints:
//! - /services/configs/conf-{config_file} - List stanzas for a config file
//! - /services/configs/conf-{config_file}/{stanza_name} - Get a specific stanza

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Represents a configuration stanza (a section within a .conf file).
///
/// Configuration stanzas have a name and a set of key-value pairs representing
/// the settings for that stanza. Different config files have different schemas,
/// so settings are stored as a dynamic HashMap.
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigStanza {
    /// The stanza name (entry name from API, e.g., "source::...", "host::...").
    #[serde(default)]
    pub name: String,
    /// The configuration file this stanza belongs to (e.g., "props", "transforms").
    #[serde(skip)]
    pub config_file: String,
    /// Dynamic key-value pairs for stanza settings.
    #[serde(flatten)]
    pub settings: HashMap<String, serde_json::Value>,
}

/// Wrapper for a single config stanza entry in API response.
///
/// Splunk's REST API wraps each resource in an entry structure containing
/// metadata and the actual content.
#[derive(Debug, Deserialize, Clone)]
pub struct ConfigStanzaEntry {
    pub name: String,
    pub content: ConfigStanza,
}

/// Response from listing configuration stanzas.
///
/// Wrapper struct for deserializing the Splunk API response when listing
/// configuration stanzas for a specific config file.
#[derive(Debug, Deserialize, Clone)]
pub struct ConfigListResponse {
    /// The list of config stanza entries returned by the API.
    pub entry: Vec<ConfigStanzaEntry>,
}

/// Represents a configuration file type (e.g., props.conf).
#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ConfigFile {
    /// The config file name (e.g., "props", "transforms", "inputs").
    pub name: String,
    /// Human-readable title.
    pub title: String,
    /// Optional description of the config file.
    pub description: Option<String>,
}

/// Supported configuration file types.
///
/// This is a curated list of commonly accessed Splunk configuration files.
/// The API supports arbitrary config files, but these are the most frequently used.
pub const SUPPORTED_CONFIG_FILES: &[&str] = &[
    "props",
    "transforms",
    "inputs",
    "outputs",
    "server",
    "indexes",
    "savedsearches",
    "authentication",
    "authorize",
    "distsearch",
    "limits",
    "web",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_config_stanza() {
        let json = r#"{
            "name": "source::...",
            "sourcetype": "access_combined",
            "TIME_PREFIX": "^\\[",
            "TIME_FORMAT": "%d/%b/%Y:%H:%M:%S %z"
        }"#;
        let stanza: ConfigStanza = serde_json::from_str(json).unwrap();
        assert_eq!(stanza.name, "source::...");
        // config_file is set programmatically, not deserialized
        assert_eq!(stanza.config_file, "");
        assert_eq!(
            stanza.settings.get("sourcetype").unwrap().as_str().unwrap(),
            "access_combined"
        );
    }

    #[test]
    fn test_deserialize_config_stanza_with_nested_values() {
        let json = r#"{
            "name": "host::myhost",
            "complex_setting": {"nested": "value"},
            "simple_setting": "simple_value"
        }"#;
        let stanza: ConfigStanza = serde_json::from_str(json).unwrap();
        assert_eq!(stanza.name, "host::myhost");
        assert!(stanza.settings.contains_key("complex_setting"));
        assert!(stanza.settings.contains_key("simple_setting"));
    }

    #[test]
    fn test_deserialize_config_list_response() {
        let json = r#"{
            "entry": [
                {
                    "name": "source::...",
                    "content": {
                        "sourcetype": "access_combined",
                        "TIME_PREFIX": "^\\["
                    }
                },
                {
                    "name": "host::myhost",
                    "content": {
                        "sourcetype": "syslog"
                    }
                }
            ]
        }"#;
        let response: ConfigListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.entry.len(), 2);
        assert_eq!(response.entry[0].name, "source::...");
        assert_eq!(
            response.entry[0]
                .content
                .settings
                .get("sourcetype")
                .unwrap()
                .as_str()
                .unwrap(),
            "access_combined"
        );
    }

    #[test]
    fn test_supported_config_files_list() {
        assert!(SUPPORTED_CONFIG_FILES.contains(&"props"));
        assert!(SUPPORTED_CONFIG_FILES.contains(&"transforms"));
        assert!(SUPPORTED_CONFIG_FILES.contains(&"inputs"));
        assert!(!SUPPORTED_CONFIG_FILES.contains(&"nonexistent"));
    }

    #[test]
    fn test_config_file_struct() {
        let config_file = ConfigFile {
            name: "props".to_string(),
            title: "Props Configuration".to_string(),
            description: Some("Props configuration file".to_string()),
        };
        assert_eq!(config_file.name, "props");
        assert_eq!(config_file.title, "Props Configuration");
        assert_eq!(
            config_file.description,
            Some("Props configuration file".to_string())
        );
    }
}
