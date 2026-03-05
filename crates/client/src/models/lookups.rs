//! Lookup table models for Splunk lookup API.
//!
//! This module contains types for managing lookup table files (CSV-based lookups)
//! via the Splunk REST API.
//!
//! # What this module handles:
//! - Lookup table file metadata (name, filename, size, etc.)
//! - ACL information for lookup tables
//! - API response structures for lookup table listings
//! - Lookup file upload parameters
//!
//! # What this module does NOT handle:
//! - KV store lookups (different endpoint)
//! - Lookup transformations

use serde::{Deserialize, Serialize};

/// Lookup table file metadata.
#[derive(Debug, Deserialize, Serialize, Clone)]
pub struct LookupTable {
    /// The lookup table name
    pub name: String,
    /// The actual filename
    pub filename: String,
    /// Owner of the lookup
    pub owner: String,
    /// App context
    pub app: String,
    /// Sharing level (user, app, global)
    pub sharing: String,
    /// File size in bytes
    #[serde(
        default,
        deserialize_with = "crate::serde_helpers::usize_from_string_or_number"
    )]
    pub size: usize,
}

/// Entry wrapper for lookup table responses.
#[derive(Debug, Deserialize, Clone)]
pub struct LookupTableEntry {
    pub name: String,
    pub content: LookupTable,
}

/// Lookup table list response.
#[derive(Debug, Deserialize, Clone)]
pub struct LookupTableListResponse {
    pub entry: Vec<LookupTableEntry>,
}

/// Parameters for uploading a lookup table file.
#[derive(Debug, Clone)]
pub struct UploadLookupParams {
    /// The lookup name (identifier in Splunk)
    pub name: String,
    /// The CSV filename (e.g., "my_lookup.csv")
    pub filename: String,
    /// The file content as bytes
    pub content: Vec<u8>,
    /// App context (namespace) - defaults to "search"
    pub app: Option<String>,
    /// Owner context (namespace) - defaults to "-" (all users)
    pub owner: Option<String>,
    /// Sharing level (user, app, global)
    pub sharing: Option<String>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_deserialize_lookup_table() {
        let json = r#"{
            "name": "my_lookup",
            "filename": "my_lookup.csv",
            "owner": "admin",
            "app": "search",
            "sharing": "app",
            "size": "1024"
        }"#;
        let lookup: LookupTable = serde_json::from_str(json).unwrap();
        assert_eq!(lookup.name, "my_lookup");
        assert_eq!(lookup.filename, "my_lookup.csv");
        assert_eq!(lookup.owner, "admin");
        assert_eq!(lookup.app, "search");
        assert_eq!(lookup.sharing, "app");
        assert_eq!(lookup.size, 1024);
    }

    #[test]
    fn test_deserialize_lookup_table_with_numeric_size() {
        let json = r#"{
            "name": "numeric_size_lookup",
            "filename": "test.csv",
            "owner": "admin",
            "app": "search",
            "sharing": "global",
            "size": 2048
        }"#;
        let lookup: LookupTable = serde_json::from_str(json).unwrap();
        assert_eq!(lookup.size, 2048);
    }

    #[test]
    fn test_deserialize_lookup_table_list_response() {
        let json = r#"{
            "entry": [
                {
                    "name": "lookup1",
                    "content": {
                        "name": "lookup1",
                        "filename": "lookup1.csv",
                        "owner": "admin",
                        "app": "search",
                        "sharing": "app",
                        "size": "1024"
                    }
                },
                {
                    "name": "lookup2",
                    "content": {
                        "name": "lookup2",
                        "filename": "lookup2.csv",
                        "owner": "user1",
                        "app": "myapp",
                        "sharing": "user",
                        "size": "512"
                    }
                }
            ]
        }"#;
        let response: LookupTableListResponse = serde_json::from_str(json).unwrap();
        assert_eq!(response.entry.len(), 2);
        assert_eq!(response.entry[0].content.name, "lookup1");
        assert_eq!(response.entry[1].content.name, "lookup2");
    }

    #[test]
    fn test_deserialize_lookup_table_with_empty_size() {
        let json = r#"{
            "name": "empty_lookup",
            "filename": "empty.csv",
            "owner": "admin",
            "app": "search",
            "sharing": "app"
        }"#;
        let lookup: LookupTable = serde_json::from_str(json).unwrap();
        assert_eq!(lookup.size, 0);
    }
}
