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

/// Parameters for creating a new saved search.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SavedSearchCreateParams {
    /// The name of the saved search (required).
    pub name: String,
    /// The search query (required).
    pub search: String,
    /// Optional description.
    pub description: Option<String>,
    /// Whether the search is disabled.
    pub disabled: bool,
}

/// Parameters for updating an existing saved search.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct SavedSearchUpdateParams {
    /// New search query (SPL).
    pub search: Option<String>,
    /// New description.
    pub description: Option<String>,
    /// Enable/disable flag.
    pub disabled: Option<bool>,
}
