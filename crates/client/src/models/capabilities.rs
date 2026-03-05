//! Capability models for Splunk authorization API.
//!
//! This module contains types for listing Splunk capabilities.
//! Capabilities are read-only in Splunk - they represent permissions
//! that can be assigned to roles.

use serde::{Deserialize, Serialize};

/// Splunk capability information.
///
/// Capabilities are predefined strings in Splunk that represent
/// specific permissions (e.g., "admin_all_objects", "search", etc.).
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Capability {
    #[serde(default)]
    pub name: String,
}

/// Capability entry wrapper.
#[derive(Debug, Deserialize, Clone)]
pub struct CapabilityEntry {
    pub name: String,
    pub content: Capability,
}

/// Capability list response.
#[derive(Debug, Deserialize, Clone)]
pub struct CapabilityListResponse {
    pub entry: Vec<CapabilityEntry>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_capability() {
        let json = r#"{
            "name": "admin_all_objects"
        }"#;
        let capability: Capability = serde_json::from_str(json).unwrap();
        assert_eq!(capability.name, "admin_all_objects");
    }

    #[test]
    fn test_deserialize_capability_entry() {
        let json = r#"{
            "name": "search",
            "content": {
                "name": "search"
            }
        }"#;
        let entry: CapabilityEntry = serde_json::from_str(json).unwrap();
        assert_eq!(entry.name, "search");
        assert_eq!(entry.content.name, "search");
    }

    #[test]
    fn test_deserialize_capability_list_response() {
        let json = r#"{
            "entry": [
                {
                    "name": "admin_all_objects",
                    "content": {
                        "name": "admin_all_objects"
                    }
                },
                {
                    "name": "search",
                    "content": {
                        "name": "search"
                    }
                }
            ]
        }"#;
        let response: CapabilityListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.entry.len(), 2);
        assert_eq!(response.entry[0].name, "admin_all_objects");
        assert_eq!(response.entry[1].name, "search");
    }
}
