//! Input models for Splunk data inputs (TCP, UDP, Monitor, Script).
//!
//! This module contains types for listing and managing Splunk data inputs.
//!
//! # What this module handles:
//! - Deserialization of data input data from Splunk REST API
//! - Type-safe representation of input metadata (TCP, UDP, Monitor, Script)
//!
//! # What this module does NOT handle:
//! - Direct HTTP API calls (see [`crate::endpoints::inputs`])
//! - Client-side filtering or searching of inputs
//!
//! Splunk inputs API endpoints:
//! - /services/data/inputs/tcp/raw
//! - /services/data/inputs/tcp/cooked
//! - /services/data/inputs/udp
//! - /services/data/inputs/monitor
//! - /services/data/inputs/script

use serde::{Deserialize, Serialize};

/// Splunk data input information.
///
/// Represents a Splunk data input (TCP, UDP, Monitor, or Script).
/// The input_type field indicates which type of input this is.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Input {
    /// The input name (entry name from API).
    #[serde(default)]
    pub name: String,
    /// Input type: tcp/raw, tcp/cooked, udp, monitor, script.
    #[serde(default)]
    pub input_type: String,
    /// Whether the input is disabled.
    #[serde(default)]
    pub disabled: bool,
    /// Host value for the input.
    pub host: Option<String>,
    /// Source value for the input.
    pub source: Option<String>,
    /// Sourcetype value for the input.
    pub sourcetype: Option<String>,
    /// Connection host setting (TCP/UDP).
    #[serde(rename = "connection_host")]
    pub connection_host: Option<String>,
    /// Port number (TCP/UDP).
    pub port: Option<String>,
    /// Path to monitor (Monitor inputs).
    pub path: Option<String>,
    /// Blacklist pattern (Monitor inputs).
    pub blacklist: Option<String>,
    /// Whitelist pattern (Monitor inputs).
    pub whitelist: Option<String>,
    /// Recursive monitoring (Monitor inputs).
    pub recursive: Option<bool>,
    /// Command to execute (Script inputs).
    pub command: Option<String>,
    /// Execution interval (Script inputs).
    pub interval: Option<String>,
}

/// Input entry wrapper.
///
/// Splunk's REST API wraps each resource in an entry structure containing
/// metadata and the actual content.
#[derive(Debug, Deserialize, Clone)]
pub struct InputEntry {
    pub name: String,
    pub content: Input,
}

/// Input list response.
///
/// Wrapper struct for deserializing the Splunk API response when listing inputs.
#[derive(Debug, Deserialize, Clone)]
pub struct InputListResponse {
    /// The list of input entries returned by the API.
    pub entry: Vec<InputEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_tcp_input() {
        let json = r#"{
            "name": "9997",
            "input_type": "tcp/raw",
            "disabled": false,
            "host": "$decideOnStartup",
            "source": "tcp:9997",
            "sourcetype": "tcp",
            "connection_host": "ip",
            "port": "9997"
        }"#;
        let input: Input = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "9997");
        assert_eq!(input.input_type, "tcp/raw");
        assert!(!input.disabled);
        assert_eq!(input.host, Some("$decideOnStartup".to_string()));
        assert_eq!(input.source, Some("tcp:9997".to_string()));
        assert_eq!(input.sourcetype, Some("tcp".to_string()));
        assert_eq!(input.connection_host, Some("ip".to_string()));
        assert_eq!(input.port, Some("9997".to_string()));
    }

    #[test]
    fn test_deserialize_monitor_input() {
        let json = r#"{
            "name": "/var/log",
            "input_type": "monitor",
            "disabled": false,
            "host": "default",
            "sourcetype": "syslog",
            "path": "/var/log",
            "recursive": true
        }"#;
        let input: Input = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "/var/log");
        assert_eq!(input.input_type, "monitor");
        assert!(!input.disabled);
        assert_eq!(input.host, Some("default".to_string()));
        assert_eq!(input.sourcetype, Some("syslog".to_string()));
        assert_eq!(input.path, Some("/var/log".to_string()));
        assert_eq!(input.recursive, Some(true));
    }

    #[test]
    fn test_deserialize_input_with_optional_fields_missing() {
        let json = r#"{
            "name": "minimal_input",
            "input_type": "udp",
            "disabled": true
        }"#;
        let input: Input = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "minimal_input");
        assert_eq!(input.input_type, "udp");
        assert!(input.disabled);
        assert_eq!(input.host, None);
        assert_eq!(input.source, None);
        assert_eq!(input.sourcetype, None);
        assert_eq!(input.port, None);
    }

    #[test]
    fn test_deserialize_input_list_response() {
        let json = r#"{
            "entry": [
                {
                    "name": "9997",
                    "content": {
                        "name": "",
                        "input_type": "tcp/raw",
                        "disabled": false,
                        "host": "$decideOnStartup",
                        "port": "9997"
                    }
                }
            ]
        }"#;
        let response: InputListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.entry.len(), 1);
        assert_eq!(response.entry[0].name, "9997");
        assert_eq!(response.entry[0].content.input_type, "tcp/raw");
    }
}
