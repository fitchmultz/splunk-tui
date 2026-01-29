//! Common types shared across Splunk API models.
//!
//! This module contains generic wrappers and shared types used by multiple
//! resource modules. It does NOT contain resource-specific models.

use serde::{Deserialize, Serialize};

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
    pub message_type: String,
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
        assert_eq!(msgs.messages[0].message_type, "ERROR");
        assert_eq!(msgs.messages[0].text, "Invalid username or password");
    }
}
