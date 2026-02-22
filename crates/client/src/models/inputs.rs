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
use std::fmt;

/// Type of Splunk data input.
///
/// Represents the various input types supported by Splunk:
/// - TCP raw and cooked inputs
/// - UDP inputs
/// - File monitoring inputs
/// - Script-based inputs
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum InputType {
    /// TCP raw input (receives unparsed data).
    #[serde(rename = "tcp/raw")]
    TcpRaw,
    /// TCP cooked input (receives parsed Splunk events).
    #[serde(rename = "tcp/cooked")]
    TcpCooked,
    /// UDP input.
    #[serde(rename = "udp")]
    Udp,
    /// File monitor input.
    #[serde(rename = "monitor")]
    Monitor,
    /// Scripted input.
    #[serde(rename = "script")]
    Script,
    /// Unknown or unrecognized input type.
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for InputType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            InputType::TcpRaw => write!(f, "tcp/raw"),
            InputType::TcpCooked => write!(f, "tcp/cooked"),
            InputType::Udp => write!(f, "udp"),
            InputType::Monitor => write!(f, "monitor"),
            InputType::Script => write!(f, "script"),
            InputType::Unknown => write!(f, "unknown"),
        }
    }
}

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
    pub input_type: InputType,
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
    #[serde(rename = "connectionHost")]
    pub connection_host: Option<String>,
    /// Port number (TCP/UDP).
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::opt_string_from_number_or_string"
    )]
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
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::opt_string_from_number_or_string"
    )]
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
            "connectionHost": "ip",
            "port": "9997"
        }"#;
        let input: Input = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "9997");
        assert_eq!(input.input_type, InputType::TcpRaw);
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
        assert_eq!(input.input_type, InputType::Monitor);
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
        assert_eq!(input.input_type, InputType::Udp);
        assert!(input.disabled);
        assert_eq!(input.host, None);
        assert_eq!(input.source, None);
        assert_eq!(input.sourcetype, None);
        assert_eq!(input.port, None);
    }

    #[test]
    fn test_deserialize_script_input_with_numeric_interval() {
        let json = r#"{
            "name": "script://./bin/collect.sh",
            "input_type": "script",
            "disabled": false,
            "command": "./bin/collect.sh",
            "interval": 3600
        }"#;
        let input: Input = serde_json::from_str(json).unwrap();
        assert_eq!(input.input_type, InputType::Script);
        assert_eq!(input.interval, Some("3600".to_string()));
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
        assert_eq!(response.entry[0].content.input_type, InputType::TcpRaw);
    }

    // =========================================================================
    // InputType enum tests
    // =========================================================================

    #[test]
    fn test_deserialize_all_known_input_types() {
        // TcpRaw
        let json = r#""tcp/raw""#;
        let input_type: InputType = serde_json::from_str(json).unwrap();
        assert_eq!(input_type, InputType::TcpRaw);

        // TcpCooked
        let json = r#""tcp/cooked""#;
        let input_type: InputType = serde_json::from_str(json).unwrap();
        assert_eq!(input_type, InputType::TcpCooked);

        // Udp
        let json = r#""udp""#;
        let input_type: InputType = serde_json::from_str(json).unwrap();
        assert_eq!(input_type, InputType::Udp);

        // Monitor
        let json = r#""monitor""#;
        let input_type: InputType = serde_json::from_str(json).unwrap();
        assert_eq!(input_type, InputType::Monitor);

        // Script
        let json = r#""script""#;
        let input_type: InputType = serde_json::from_str(json).unwrap();
        assert_eq!(input_type, InputType::Script);
    }

    #[test]
    fn test_deserialize_unknown_input_type() {
        let json = r#""some_future_type""#;
        let input_type: InputType = serde_json::from_str(json).unwrap();
        assert_eq!(input_type, InputType::Unknown);
    }

    #[test]
    fn test_display_input_type() {
        assert_eq!(InputType::TcpRaw.to_string(), "tcp/raw");
        assert_eq!(InputType::TcpCooked.to_string(), "tcp/cooked");
        assert_eq!(InputType::Udp.to_string(), "udp");
        assert_eq!(InputType::Monitor.to_string(), "monitor");
        assert_eq!(InputType::Script.to_string(), "script");
        assert_eq!(InputType::Unknown.to_string(), "unknown");
    }

    #[test]
    fn test_input_type_default() {
        let default = InputType::default();
        assert_eq!(default, InputType::Unknown);
    }

    #[test]
    fn test_serialize_input_type() {
        assert_eq!(
            serde_json::to_string(&InputType::TcpRaw).unwrap(),
            r#""tcp/raw""#
        );
        assert_eq!(
            serde_json::to_string(&InputType::TcpCooked).unwrap(),
            r#""tcp/cooked""#
        );
        assert_eq!(serde_json::to_string(&InputType::Udp).unwrap(), r#""udp""#);
        assert_eq!(
            serde_json::to_string(&InputType::Monitor).unwrap(),
            r#""monitor""#
        );
        assert_eq!(
            serde_json::to_string(&InputType::Script).unwrap(),
            r#""script""#
        );
    }

    #[test]
    fn test_deserialize_tcp_cooked_input() {
        let json = r#"{
            "name": "cooked_tcp",
            "input_type": "tcp/cooked",
            "disabled": false,
            "host": "localhost",
            "port": "8088"
        }"#;
        let input: Input = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "cooked_tcp");
        assert_eq!(input.input_type, InputType::TcpCooked);
        assert_eq!(input.input_type.to_string(), "tcp/cooked");
    }

    #[test]
    fn test_deserialize_script_input() {
        let json = r#"{
            "name": "my_script",
            "input_type": "script",
            "disabled": false,
            "command": "/opt/scripts/collect_data.sh",
            "interval": "60"
        }"#;
        let input: Input = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "my_script");
        assert_eq!(input.input_type, InputType::Script);
        assert_eq!(
            input.command,
            Some("/opt/scripts/collect_data.sh".to_string())
        );
        assert_eq!(input.interval, Some("60".to_string()));
    }

    #[test]
    fn test_deserialize_input_with_unknown_type() {
        let json = r#"{
            "name": "future_input",
            "input_type": "future/new_type",
            "disabled": false
        }"#;
        let input: Input = serde_json::from_str(json).unwrap();
        assert_eq!(input.name, "future_input");
        assert_eq!(input.input_type, InputType::Unknown);
        assert_eq!(input.input_type.to_string(), "unknown");
    }

    #[test]
    fn test_input_type_clone() {
        let original = InputType::TcpRaw;
        let cloned = original.clone();
        assert_eq!(original, cloned);
    }

    #[test]
    fn test_input_type_equality() {
        assert_eq!(InputType::TcpRaw, InputType::TcpRaw);
        assert_ne!(InputType::TcpRaw, InputType::TcpCooked);
        assert_ne!(InputType::Udp, InputType::Monitor);
    }
}
