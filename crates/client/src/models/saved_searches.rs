//! Saved search models for Splunk saved search API.
//!
//! This module contains types for listing and managing saved searches.

use serde::{Deserialize, Serialize};

/// Saved search information.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct SavedSearch {
    #[serde(default)]
    pub name: String,
    pub search: String,
    pub description: Option<String>,
    #[serde(default)]
    pub disabled: bool,
}

/// Saved search entry.
#[derive(Debug, Deserialize, Clone)]
pub struct SavedSearchEntry {
    pub name: String,
    pub content: SavedSearch,
}

/// Saved search list response.
#[derive(Debug, Deserialize, Clone)]
pub struct SavedSearchListResponse {
    pub entry: Vec<SavedSearchEntry>,
}
