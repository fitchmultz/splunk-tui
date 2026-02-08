//! Common types shared across Splunk API models.
//!
//! This module contains generic wrappers and shared types used by multiple
//! resource modules. It does NOT contain resource-specific models.

use serde::{Deserialize, Serialize};
use std::fmt;

/// Type of message from Splunk API.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
pub enum MessageType {
    #[serde(rename = "ERROR")]
    Error,
    #[serde(rename = "WARN")]
    Warn,
    #[serde(rename = "INFO")]
    Info,
    /// Unknown or unrecognized message type.
    #[serde(other)]
    #[default]
    Unknown,
}

impl fmt::Display for MessageType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Error => write!(f, "ERROR"),
            Self::Warn => write!(f, "WARN"),
            Self::Info => write!(f, "INFO"),
            Self::Unknown => write!(f, "UNKNOWN"),
        }
    }
}

/// Generic Splunk REST API response wrapper.
#[derive(Debug, Deserialize, Clone)]
pub struct SplunkResponse<T> {
    pub entry: Vec<Entry<T>>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Entry<T> {
    pub name: String,
    pub content: T,
    pub acl: Option<Acl>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Acl {
    pub app: String,
    pub owner: String,
    pub perms: Option<Perms>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct Perms {
    pub read: Vec<String>,
    pub write: Vec<String>,
}

/// A single message from Splunk (usually in error responses).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SplunkMessage {
    #[serde(rename = "type")]
    pub message_type: MessageType,
    pub text: String,
}

/// A collection of messages from Splunk.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SplunkMessages {
    pub messages: Vec<SplunkMessage>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_message_type_deserialization() {
        assert_eq!(
            serde_json::from_str::<MessageType>("\"ERROR\"").unwrap(),
            MessageType::Error
        );
        assert_eq!(
            serde_json::from_str::<MessageType>("\"WARN\"").unwrap(),
            MessageType::Warn
        );
        assert_eq!(
            serde_json::from_str::<MessageType>("\"INFO\"").unwrap(),
            MessageType::Info
        );
    }

    #[test]
    fn test_message_type_unknown_fallback() {
        assert_eq!(
            serde_json::from_str::<MessageType>("\"UNKNOWN\"").unwrap(),
            MessageType::Unknown
        );
        assert_eq!(
            serde_json::from_str::<MessageType>("\"invalid\"").unwrap(),
            MessageType::Unknown
        );
    }

    #[test]
    fn test_message_type_display() {
        assert_eq!(MessageType::Error.to_string(), "ERROR");
        assert_eq!(MessageType::Warn.to_string(), "WARN");
        assert_eq!(MessageType::Info.to_string(), "INFO");
        assert_eq!(MessageType::Unknown.to_string(), "UNKNOWN");
    }

    #[test]
    fn test_message_type_default() {
        assert_eq!(MessageType::default(), MessageType::Unknown);
    }

    #[test]
    fn test_deserialize_splunk_messages() {
        let json = r#"{
            "messages": [
                {
                    "type": "ERROR",
                    "text": "Invalid username or password"
                }
            ]
        }"#;
        let msgs: SplunkMessages = serde_json::from_str(json).unwrap();
        assert_eq!(msgs.messages.len(), 1);
        assert_eq!(msgs.messages[0].message_type, MessageType::Error);
        assert_eq!(msgs.messages[0].text, "Invalid username or password");
    }

    #[test]
    fn test_deserialize_splunk_message_info() {
        let json = r#"{
            "type": "INFO",
            "text": "Operation completed"
        }"#;
        let msg: SplunkMessage = serde_json::from_str(json).unwrap();
        assert_eq!(msg.message_type, MessageType::Info);
        assert_eq!(msg.text, "Operation completed");
    }
}
