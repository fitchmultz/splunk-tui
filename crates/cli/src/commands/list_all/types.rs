//! Type definitions for list-all command output structures.
//!
//! Responsibilities:
//! - Define data structures for resource summaries and multi-profile aggregation.
//! - Provide serialization support for JSON, CSV, and XML output formats.
//!
//! Does NOT handle:
//! - Fetching logic (see `fetchers.rs`).
//! - Output formatting/rendering (see `output.rs`).
//!
//! Invariants:
//! - All timestamp fields use RFC3339 format.
//! - Error fields are skipped during serialization if None.

use serde::{Deserialize, Serialize};

/// Per-resource summary for a single resource type.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResourceSummary {
    pub resource_type: String,
    pub count: u64,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Single-profile list-all output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[allow(dead_code)]
pub struct ListAllOutput {
    pub timestamp: String,
    pub resources: Vec<ResourceSummary>,
}

/// Per-profile resource summary for multi-profile aggregation.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProfileResult {
    pub profile_name: String,
    pub base_url: String,
    pub resources: Vec<ResourceSummary>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// Multi-profile list-all output structure.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListAllMultiOutput {
    pub timestamp: String,
    pub profiles: Vec<ProfileResult>,
}

/// Valid resource types that can be queried.
pub const VALID_RESOURCES: &[&str] = &[
    "indexes",
    "jobs",
    "apps",
    "users",
    "cluster",
    "health",
    "kvstore",
    "license",
    "saved-searches",
];
